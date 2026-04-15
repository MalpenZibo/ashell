use super::{ReadOnlyService, Service, ServiceEvent};
use dbus::{Systemd1ManagerProxy, Systemd1UnitProxy};
use iced::{
    Subscription,
    futures::{SinkExt, StreamExt, stream::pending},
    stream::channel,
};
use log::{debug, error, warn};
use std::{any::TypeId, collections::HashSet, path::PathBuf};
use zbus::{MatchRule, MessageStream, message::Type as MessageType, zvariant::OwnedObjectPath};

mod dbus;

const SERVICE_SUFFIX: &str = ".service";

/// Properties we care about in PropertiesChanged signals.
const INTERESTING_PROPS: &[&str] = &["ActiveState", "SubState", "UnitFileState", "LoadState"];

const UNIT_IFACE: &str = "org.freedesktop.systemd1.Unit";

/// Snapshot of a systemd user service unit.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct UnitInfo {
    pub name: String,
    pub description: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub unit_file_state: String,
    pub object_path: OwnedObjectPath,
}

impl UnitInfo {
    /// Unit name with `.service` suffix stripped.
    pub fn display_name(&self) -> &str {
        self.name.strip_suffix(SERVICE_SUFFIX).unwrap_or(&self.name)
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self.active_state.as_str(),
            "active" | "activating" | "reloading"
        )
    }

    /// Whether the unit can be toggled (started/stopped) right now.
    pub fn can_toggle(&self) -> bool {
        if matches!(self.unit_file_state.as_str(), "alias" | "masked") {
            return false;
        }
        !matches!(
            self.active_state.as_str(),
            "activating" | "deactivating" | "reloading"
        )
    }

    /// Return a color for the status dot.
    pub fn status_color(&self) -> iced::Color {
        match self.active_state.as_str() {
            "active" => iced::Color::from_rgb(0.0, 0.8, 0.0), // green
            "activating" => iced::Color::from_rgb(1.0, 0.85, 0.0), // yellow
            "deactivating" => iced::Color::from_rgb(1.0, 0.55, 0.0), // orange
            "failed" => iced::Color::from_rgb(1.0, 0.0, 0.0), // red
            _ => iced::Color::from_rgb(0.7, 0.7, 0.7),        // gray (inactive)
        }
    }

    /// Sort key: (unit_file_state order, name).
    fn state_order(&self) -> u8 {
        match self.unit_file_state.as_str() {
            "enabled" => 0,
            "static" => 1,
            "disabled" => 2,
            "generated" => 3,
            _ => 99,
        }
    }
}

// ── Service types ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum UserServicesEvent {
    UnitsLoaded(Vec<UnitInfo>),
    UnitChanged(UnitInfo),
}

pub enum UserServicesCommand {
    ToggleUnit(String),
    Refresh,
}

#[derive(Debug, Clone)]
pub struct UserServicesService {
    pub units: Vec<UnitInfo>,
    conn: zbus::Connection,
}

impl UserServicesService {
    pub fn active_count(&self) -> usize {
        self.units.iter().filter(|u| u.is_active()).count()
    }
}

// ── ReadOnlyService ──────────────────────────────────────────────────

impl ReadOnlyService for UserServicesService {
    type UpdateEvent = UserServicesEvent;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            UserServicesEvent::UnitsLoaded(units) => {
                self.units = units;
            }
            UserServicesEvent::UnitChanged(changed) => {
                if let Some(existing) = self.units.iter_mut().find(|u| u.name == changed.name) {
                    *existing = changed;
                }
            }
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Subscription::run_with(TypeId::of::<Self>(), |_| {
            channel(100, async |mut output| {
                let mut state = State::Init;

                loop {
                    state = start_listening(state, &mut output).await;
                }
            })
        })
    }
}

// ── Service (mutable commands) ───────────────────────────────────────

impl Service for UserServicesService {
    type Command = UserServicesCommand;

    fn command(&mut self, command: Self::Command) -> iced::Task<ServiceEvent<Self>> {
        match command {
            UserServicesCommand::ToggleUnit(name) => {
                let conn = self.conn.clone();
                let is_active = self
                    .units
                    .iter()
                    .find(|u| u.name == name)
                    .is_some_and(|u| u.is_active());

                iced::Task::perform(
                    async move {
                        toggle_unit(&conn, &name, is_active).await;
                        list_user_units(&conn).await
                    },
                    |result| match result {
                        Ok(units) => ServiceEvent::Update(UserServicesEvent::UnitsLoaded(units)),
                        Err(_) => ServiceEvent::Error(()),
                    },
                )
            }
            UserServicesCommand::Refresh => {
                let conn = self.conn.clone();
                iced::Task::perform(async move { list_user_units(&conn).await }, |result| {
                    match result {
                        Ok(units) => ServiceEvent::Update(UserServicesEvent::UnitsLoaded(units)),
                        Err(_) => ServiceEvent::Error(()),
                    }
                })
            }
        }
    }
}

// ── State machine ────────────────────────────────────────────────────

enum State {
    Init,
    Active(zbus::Connection, HashSet<OwnedObjectPath>),
    Error,
}

async fn start_listening(
    state: State,
    output: &mut iced::futures::channel::mpsc::Sender<ServiceEvent<UserServicesService>>,
) -> State {
    match state {
        State::Init => match zbus::Connection::session().await {
            Ok(conn) => {
                // Subscribe so systemd sends us signals.
                if let Ok(manager) = Systemd1ManagerProxy::new(&conn).await
                    && let Err(e) = manager.subscribe().await
                {
                    warn!("systemd Subscribe failed (signals may not work): {e}");
                }

                match list_user_units(&conn).await {
                    Ok(units) => {
                        let tracked_paths: HashSet<OwnedObjectPath> =
                            units.iter().map(|u| u.object_path.clone()).collect();

                        let service = UserServicesService {
                            units: units.clone(),
                            conn: conn.clone(),
                        };
                        let _ = output.send(ServiceEvent::Init(service)).await;
                        let _ = output
                            .send(ServiceEvent::Update(UserServicesEvent::UnitsLoaded(units)))
                            .await;

                        State::Active(conn, tracked_paths)
                    }
                    Err(err) => {
                        error!("Failed to list user units: {err}");
                        let _ = output.send(ServiceEvent::Error(())).await;
                        State::Error
                    }
                }
            }
            Err(err) => {
                error!("Failed to connect to session bus for user_services: {err}");
                let _ = output.send(ServiceEvent::Error(())).await;
                State::Error
            }
        },
        State::Active(conn, tracked_paths) => {
            match listen_properties_changed(&conn, &tracked_paths, output).await {
                Ok(()) => State::Active(conn, tracked_paths),
                Err(err) => {
                    error!("Failed to listen for PropertiesChanged: {err}");
                    State::Error
                }
            }
        }
        State::Error => {
            let _ = pending::<u8>().next().await;
            State::Error
        }
    }
}

// ── PropertiesChanged listener ───────────────────────────────────────

async fn listen_properties_changed(
    conn: &zbus::Connection,
    tracked_paths: &HashSet<OwnedObjectPath>,
    output: &mut iced::futures::channel::mpsc::Sender<ServiceEvent<UserServicesService>>,
) -> anyhow::Result<()> {
    let rule = MatchRule::builder()
        .msg_type(MessageType::Signal)
        .sender("org.freedesktop.systemd1")?
        .interface("org.freedesktop.DBus.Properties")?
        .member("PropertiesChanged")?
        .path_namespace("/org/freedesktop/systemd1/unit")?
        .build();

    let stream = MessageStream::for_match_rule(rule, conn, None).await?;
    let mut chunks = stream.ready_chunks(10);

    while let Some(chunk) = chunks.next().await {
        // Collect unique tracked object paths from this batch of signals.
        let mut paths_to_update = HashSet::new();

        for msg in chunk.into_iter().flatten() {
            // Parse the signal body: (interface_name, changed_props, invalidated_props)
            let Ok(body) = msg.body().deserialize::<(
                String,
                std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
                Vec<String>,
            )>() else {
                continue;
            };

            let (iface_name, changed_props, _invalidated) = body;

            // Only care about org.freedesktop.systemd1.Unit interface.
            if iface_name != UNIT_IFACE {
                continue;
            }

            // Check if any interesting property changed.
            let dominated = changed_props
                .keys()
                .any(|k| INTERESTING_PROPS.contains(&k.as_str()));

            if !dominated {
                continue;
            }

            // Get the object path from the message header.
            let header = msg.header();
            let Some(path) = header.path() else {
                continue;
            };
            let path: OwnedObjectPath = path.to_owned().into();

            // Filter: only process signals for units we are tracking.
            if !tracked_paths.contains(&path) {
                continue;
            }

            paths_to_update.insert(path);
        }

        // One D-Bus read per unique tracked unit in this batch.
        for path in paths_to_update {
            if let Ok(info) = read_unit_from_path(conn, &path).await
                && info.name.ends_with(SERVICE_SUFFIX)
            {
                debug!("Unit changed: {} -> {}", info.name, info.active_state);
                let _ = output
                    .send(ServiceEvent::Update(UserServicesEvent::UnitChanged(info)))
                    .await;
            }
        }
    }

    Ok(())
}

// ── D-Bus helpers ────────────────────────────────────────────────────

/// Read full unit properties from a D-Bus object path.
async fn read_unit_from_path(
    conn: &zbus::Connection,
    path: &OwnedObjectPath,
) -> anyhow::Result<UnitInfo> {
    let proxy = Systemd1UnitProxy::builder(conn)
        .path(path.as_ref())?
        .build()
        .await?;

    Ok(UnitInfo {
        name: proxy.id().await?,
        description: proxy.description().await?,
        load_state: proxy.load_state().await?,
        active_state: proxy.active_state().await?,
        sub_state: proxy.sub_state().await?,
        unit_file_state: proxy.unit_file_state().await.unwrap_or_default(),
        object_path: path.clone(),
    })
}

/// Get the user unit directories to scan.
fn user_unit_dirs() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME") else {
        return Vec::new();
    };
    let home = PathBuf::from(home);
    vec![
        home.join(".config/systemd/user"),
        home.join(".local/share/systemd/user"),
    ]
}

/// List user-defined service units, replicating the Zig app's algorithm.
async fn list_user_units(conn: &zbus::Connection) -> anyhow::Result<Vec<UnitInfo>> {
    let manager = Systemd1ManagerProxy::new(conn).await?;
    let user_dirs = user_unit_dirs();

    // 1. Scan filesystem for user-defined unit file names.
    let mut user_unit_names = std::collections::HashSet::new();
    for dir in &user_dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.ends_with(SERVICE_SUFFIX) {
                    user_unit_names.insert(name.into_owned());
                }
            }
        }
    }

    // 2. Get unit file states from systemd, filtered to user dirs.
    let mut uf_states = std::collections::HashMap::new();
    if let Ok(unit_files) = manager.list_unit_files().await {
        for (file_path, state) in &unit_files {
            let Some(basename) = file_path.rsplit('/').next() else {
                continue;
            };
            if !basename.ends_with(SERVICE_SUFFIX) {
                continue;
            }
            let is_user = user_dirs
                .iter()
                .any(|dir| file_path.starts_with(dir.to_string_lossy().as_ref()));
            if !is_user {
                continue;
            }
            uf_states.insert(basename.to_owned(), state.clone());
        }
    }

    // 3. Get currently loaded units.
    let mut loaded = std::collections::HashMap::new();
    if let Ok(units) = manager.list_units().await {
        for (name, description, load_state, active_state, sub_state, _, object_path, _, _, _) in
            &units
        {
            if name.ends_with(SERVICE_SUFFIX) {
                loaded.insert(
                    name.clone(),
                    (
                        description.clone(),
                        load_state.clone(),
                        active_state.clone(),
                        sub_state.clone(),
                        object_path.clone(),
                    ),
                );
            }
        }
    }

    // 4. Merge: for each user unit file, build a UnitInfo.
    let mut result = Vec::with_capacity(uf_states.len());
    for (name, uf_state) in &uf_states {
        if let Some((description, load_state, active_state, sub_state, object_path)) =
            loaded.get(name)
        {
            // Unit is currently loaded — use runtime state.
            result.push(UnitInfo {
                name: name.clone(),
                description: description.clone(),
                load_state: load_state.clone(),
                active_state: active_state.clone(),
                sub_state: sub_state.clone(),
                unit_file_state: uf_state.clone(),
                object_path: object_path.clone(),
            });
        } else {
            // Not currently loaded — try LoadUnit to get an object path.
            let object_path = manager
                .load_unit(name)
                .await
                .unwrap_or_else(|_| OwnedObjectPath::default());

            result.push(UnitInfo {
                name: name.clone(),
                description: name.clone(),
                load_state: "not-found".to_owned(),
                active_state: "inactive".to_owned(),
                sub_state: "dead".to_owned(),
                unit_file_state: uf_state.clone(),
                object_path,
            });
        }
    }

    // 5. Sort by (state order, name).
    result.sort_by(|a, b| {
        a.state_order()
            .cmp(&b.state_order())
            .then_with(|| a.name.cmp(&b.name))
    });

    debug!("Found {} user service units", result.len());

    Ok(result)
}

/// Toggle a unit: start if inactive, stop if active.
async fn toggle_unit(conn: &zbus::Connection, name: &str, is_active: bool) {
    let Ok(manager) = Systemd1ManagerProxy::new(conn).await else {
        error!("Failed to create systemd manager proxy for toggle");
        return;
    };

    let result = if is_active {
        debug!("Stopping unit: {name}");
        manager.stop_unit(name, "replace").await
    } else {
        debug!("Starting unit: {name}");
        manager.start_unit(name, "replace").await
    };

    if let Err(e) = result {
        error!("Failed to toggle unit {name}: {e}");
    }
}
