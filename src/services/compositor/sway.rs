//! Sway backend. Sway speaks the i3 IPC protocol over the socket named by
//! `$SWAYSOCK`: a 14 byte header (`i3-ipc` magic, native endian payload length
//! and message type) followed by a JSON payload. `swayipc-types` provides the
//! message and reply types; the socket handling stays here so the backend runs
//! on tokio (`swayipc-async` would pull in the smol reactor).
//!
//! Every subscribed event triggers a full state resync, so event payloads are
//! never decoded: sway can add event variants without breaking this backend.

use super::types::{
    ActiveWindow, ActiveWindowSway, CompositorCommand, CompositorEvent, CompositorMonitor,
    CompositorService, CompositorState, CompositorWorkspace,
};
use crate::services::ServiceEvent;
use anyhow::{Context, Result, anyhow, bail};
use itertools::Itertools;
use serde::de::DeserializeOwned;
use std::{
    collections::{HashMap, HashSet},
    env,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};
use swayipc_types::{
    BindingState, CommandOutcome, CommandType, EventType, Input, MAGIC, Node, NodeType, Output,
    Success, Workspace,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
    sync::{broadcast, mpsc},
    task::JoinHandle,
    time::{Duration, Instant, timeout_at},
};

const HEADER_LEN: usize = MAGIC.len() + 4 + 4;
const EVENT_BIT: u32 = 0x8000_0000;

/// Refuse absurd payload lengths instead of trying to allocate them; the
/// largest real reply (GET_TREE) stays well below this.
const MAX_PAYLOAD_LEN: usize = 32 * 1024 * 1024;

/// A burst of events (opening a window moves focus, changes the workspace
/// contents and the title) is coalesced into a single resync.
const DEBOUNCE: Duration = Duration::from_millis(40);

/// Start of the id range handed to workspaces that cannot use their own number:
/// far beyond any plausible `workspace_names` list and any plausible workspace
/// number, so those workspaces keep rendering under their real name.
const NAMED_ID_BASE: i32 = 10_000;

pub fn is_available() -> bool {
    // A leftover SWAYSOCK from an exited session would otherwise shadow the
    // generic backend, so require the socket to still be there.
    socket_path().is_some_and(|path| path.exists())
}

fn socket_path() -> Option<PathBuf> {
    env::var_os("SWAYSOCK").map(PathBuf::from)
}

/// Maps sway workspaces onto the positive, stable ids the workspaces module
/// expects. A negative id marks a special workspace, and the module uses the id
/// as a 1-based index into `workspace_names`, so a numbered workspace keeps its
/// sway number as its id.
#[derive(Default)]
struct WorkspaceRegistry {
    ids: HashMap<String, i32>,
    names: HashMap<i32, String>,
}

impl WorkspaceRegistry {
    fn id_for(&mut self, name: &str, num: i32) -> i32 {
        if let Some(&id) = self.ids.get(name) {
            return id;
        }

        let id = if num >= 0 && !self.names.contains_key(&num) {
            num
        } else {
            // Named workspaces report `num == -1`, and two workspaces can share
            // a number ("3:web" and "3:mail" both report 3), so fall back to the
            // first free id in the high range.
            (NAMED_ID_BASE..i32::MAX)
                .find(|id| !self.names.contains_key(id))
                .unwrap_or(NAMED_ID_BASE)
        };

        self.ids.insert(name.to_owned(), id);
        self.names.insert(id, name.to_owned());
        id
    }

    fn name_for(&self, id: i32) -> Option<&str> {
        self.names.get(&id).map(String::as_str)
    }

    /// Forget workspaces that no longer exist, keeping the maps bounded and the
    /// ids dense across a long session.
    fn retain<'a>(&mut self, current: impl Iterator<Item = &'a str>) {
        let live: HashSet<&str> = current.collect();
        self.ids.retain(|name, _| live.contains(name.as_str()));
        let kept: HashSet<i32> = self.ids.values().copied().collect();
        self.names.retain(|id, _| kept.contains(id));
    }
}

fn workspace_registry() -> &'static Mutex<WorkspaceRegistry> {
    static REGISTRY: OnceLock<Mutex<WorkspaceRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(Mutex::default)
}

/// Active output names, indexed by the `monitor_id` handed out in the state.
fn output_names() -> &'static Mutex<Vec<String>> {
    static NAMES: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    NAMES.get_or_init(Mutex::default)
}

/// One sway IPC connection. Sway replies to requests in order on the same
/// socket, so a connection is either used for requests or, once subscribed, for
/// events.
struct SwaySocket {
    stream: UnixStream,
}

impl SwaySocket {
    async fn connect() -> Result<Self> {
        let path = socket_path().ok_or_else(|| anyhow!("SWAYSOCK is not set"))?;
        let stream = UnixStream::connect(&path)
            .await
            .with_context(|| format!("failed to connect to the Sway socket {}", path.display()))?;
        Ok(Self { stream })
    }

    async fn read_frame(&mut self) -> Result<(u32, Vec<u8>)> {
        let mut header = [0u8; HEADER_LEN];
        self.stream
            .read_exact(&mut header)
            .await
            .context("failed to read the Sway IPC header")?;

        if header[..MAGIC.len()] != MAGIC {
            bail!(
                "invalid Sway IPC magic '{}'",
                String::from_utf8_lossy(&header[..MAGIC.len()])
            );
        }

        let payload_len = u32::from_ne_bytes(
            header[MAGIC.len()..MAGIC.len() + 4]
                .try_into()
                .expect("slice of 4 bytes"),
        ) as usize;
        let frame_type = u32::from_ne_bytes(
            header[MAGIC.len() + 4..]
                .try_into()
                .expect("slice of 4 bytes"),
        );

        if payload_len > MAX_PAYLOAD_LEN {
            bail!("Sway IPC payload of {payload_len} bytes is implausibly large");
        }

        let mut payload = vec![0u8; payload_len];
        if payload_len > 0 {
            self.stream
                .read_exact(&mut payload)
                .await
                .context("failed to read the Sway IPC payload")?;
        }

        Ok((frame_type, payload))
    }

    async fn request<T: DeserializeOwned>(
        &mut self,
        command: CommandType,
        payload: &[u8],
    ) -> Result<T> {
        self.stream
            .write_all(&command.encode_with(payload))
            .await
            .with_context(|| format!("failed to send {command:?}"))?;
        self.stream.flush().await?;

        // `decode` also rejects a reply whose type does not match the request.
        let frame = self.read_frame().await?;
        command
            .decode(frame)
            .with_context(|| format!("failed to decode the {command:?} reply"))
    }

    async fn subscribe(&mut self, events: &[EventType]) -> Result<()> {
        let payload = serde_json::to_vec(events)?;
        let reply: Success = self.request(CommandType::Subscribe, &payload).await?;
        if !reply.success {
            bail!("Sway refused the IPC event subscription");
        }
        Ok(())
    }
}

/// Request connection kept alive across resyncs: a sync issues five requests,
/// so reconnecting every time burns file descriptors for nothing. A dead socket
/// is replaced on the next request.
#[derive(Default)]
struct SwayRequests {
    socket: Option<SwaySocket>,
}

impl SwayRequests {
    async fn request<T: DeserializeOwned>(
        &mut self,
        command: CommandType,
        payload: &[u8],
    ) -> Result<T> {
        let mut last_error = None;

        for _ in 0..2 {
            if self.socket.is_none() {
                match SwaySocket::connect().await {
                    Ok(socket) => self.socket = Some(socket),
                    Err(e) => {
                        last_error = Some(e);
                        continue;
                    }
                }
            }

            let Some(socket) = self.socket.as_mut() else {
                continue;
            };

            match socket.request(command, payload).await {
                Ok(value) => return Ok(value),
                Err(e) => {
                    // The connection is unusable once a frame went missing or
                    // half a request was written: drop it and try a fresh one.
                    self.socket = None;
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("{command:?} failed")))
    }
}

/// Everything a state update is built from.
struct SwaySnapshot {
    workspaces: Vec<Workspace>,
    outputs: Vec<Output>,
    /// Optional so the bar still shows workspaces when the tree cannot be read
    /// or deserialized (sway built without xwayland omits `urgent` on nodes).
    tree: Option<Node>,
    inputs: Vec<Input>,
    binding_state: Option<BindingState>,
}

async fn fetch_snapshot(requests: &mut SwayRequests) -> Result<SwaySnapshot> {
    let workspaces = requests.request(CommandType::GetWorkspaces, b"").await?;
    let outputs = requests.request(CommandType::GetOutputs, b"").await?;

    Ok(SwaySnapshot {
        workspaces,
        outputs,
        tree: optional(requests.request(CommandType::GetTree, b"").await),
        inputs: optional(requests.request(CommandType::GetInputs, b"").await).unwrap_or_default(),
        binding_state: optional(requests.request(CommandType::GetBindingState, b"").await),
    })
}

/// Degrade gracefully for the parts of the state that are not essential: a
/// missing window title is better than no workspaces at all.
fn optional<T>(result: Result<T>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(e) => {
            // A permanent failure (a field this sway build never sends, say)
            // would otherwise warn on every resync, so only report a change.
            static LAST_WARNING: Mutex<Option<String>> = Mutex::new(None);
            let message = e.to_string();
            let mut last = LAST_WARNING.lock().unwrap();
            if last.as_deref() != Some(message.as_str()) {
                log::warn!("Sway state is incomplete: {message}");
                *last = Some(message);
            }
            None
        }
    }
}

pub async fn run_listener(tx: &broadcast::Sender<ServiceEvent<CompositorService>>) -> Result<()> {
    let mut events = SwaySocket::connect().await?;
    events
        .subscribe(&[
            EventType::Workspace,
            EventType::Output,
            EventType::Mode,
            EventType::Window,
            EventType::Input,
        ])
        .await?;

    // Subscribe before the first snapshot so no change can slip through the gap.
    let mut requests = SwayRequests::default();
    send_state(tx, &build_state(&fetch_snapshot(&mut requests).await?));

    // The event socket is read from its own task: `read_exact` is not cancel
    // safe, so the read must never sit in a `select!` arm where a half-read
    // frame could be dropped and desync the stream.
    let (notify_tx, mut notify_rx) = mpsc::channel::<()>(1);
    let _reader = AbortOnDrop(tokio::spawn(async move {
        loop {
            match events.read_frame().await {
                Ok((frame_type, _)) => {
                    if frame_type & EVENT_BIT != 0 {
                        // Capacity one: a full channel already means "resync
                        // pending", so dropping the notification loses nothing.
                        if notify_tx.try_send(()).is_err() && notify_tx.is_closed() {
                            break;
                        }
                    } else {
                        log::debug!("Ignoring a reply frame on the Sway event socket");
                    }
                }
                Err(e) => {
                    log::warn!("Sway event stream ended: {e}");
                    break;
                }
            }
        }
    }));

    let mut last_sync = Instant::now();

    loop {
        if notify_rx.recv().await.is_none() {
            bail!("Sway event stream closed");
        }

        // Drain the rest of a burst before paying for a resync. The window is
        // measured from the previous sync, so an isolated change is not delayed
        // while a burst still costs a single sync.
        let deadline = last_sync + DEBOUNCE;
        while Instant::now() < deadline {
            match timeout_at(deadline, notify_rx.recv()).await {
                Ok(Some(())) => {}
                Ok(None) => bail!("Sway event stream closed"),
                Err(_) => break,
            }
        }

        match fetch_snapshot(&mut requests).await {
            Ok(snapshot) => send_state(tx, &build_state(&snapshot)),
            // A transient failure must not end the listener: nothing restarts
            // it, and the next event resyncs anyway.
            Err(e) => log::warn!("Sway state sync failed: {e}"),
        }

        last_sync = Instant::now();
    }
}

/// Keeps the event reader tied to the listener instead of leaking it when the
/// listener returns early.
struct AbortOnDrop(JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub async fn execute_command(cmd: CompositorCommand) -> Result<()> {
    match cmd {
        CompositorCommand::FocusWorkspace(id) => {
            let name = {
                let registry = workspace_registry().lock().unwrap();
                registry
                    .name_for(id)
                    .ok_or_else(|| anyhow!("unknown Sway workspace id {id}"))?
                    .to_owned()
            };
            // Switch by name: `workspace number N` would pick whichever
            // workspace happens to share that number, and it is the only form
            // that works for named workspaces. `--no-auto-back-and-forth` keeps
            // a click on the current workspace a no-op.
            run_command(&format!(
                "workspace --no-auto-back-and-forth {}",
                quote(&name)
            ))
            .await
        }
        CompositorCommand::ScrollWorkspace(dir) => {
            let command = if dir > 0 {
                "workspace next_on_output"
            } else {
                "workspace prev_on_output"
            };
            run_command(command).await
        }
        CompositorCommand::FocusMonitor(id) => {
            let name = {
                let names = output_names().lock().unwrap();
                usize::try_from(id)
                    .ok()
                    .and_then(|idx| names.get(idx))
                    .ok_or_else(|| anyhow!("unknown Sway output id {id}"))?
                    .clone()
            };
            run_command(&format!("focus output {}", quote(&name))).await
        }
        CompositorCommand::FocusSpecialWorkspace(_)
        | CompositorCommand::ToggleSpecialWorkspace(_) => {
            Err(anyhow!("special workspaces are not supported on Sway"))
        }
        CompositorCommand::NextLayout => {
            run_command("input type:keyboard xkb_switch_layout next").await
        }
        CompositorCommand::CustomDispatch(action, args) => {
            let command = if args.is_empty() {
                action
            } else {
                format!("{action} {args}")
            };
            run_command(&command).await
        }
    }
}

/// Quote a sway command argument: workspace and output names can contain spaces
/// and, in principle, quotes.
fn quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

async fn run_command(command: &str) -> Result<()> {
    let mut socket = SwaySocket::connect().await?;
    let outcomes: Vec<CommandOutcome> = socket
        .request(CommandType::RunCommand, command.as_bytes())
        .await?;

    for outcome in outcomes {
        CommandOutcome::decode(outcome)
            .with_context(|| format!("Sway command '{command}' failed"))?;
    }

    Ok(())
}

fn send_state(tx: &broadcast::Sender<ServiceEvent<CompositorService>>, state: &CompositorState) {
    let _ = tx.send(ServiceEvent::Update(CompositorEvent::StateChanged(
        Box::new(state.clone()),
    )));
}

/// Only views are windows: a split container is a `con` as well, `app_id` is
/// absent for XWayland views and `window_properties` is absent for xdg-shell
/// ones, so neither identifies a view on its own. `shell` and `pid` are set for
/// every view and for nothing else.
fn is_view(node: &Node) -> bool {
    matches!(node.node_type, NodeType::Con | NodeType::FloatingCon)
        && (node.shell.is_some()
            || node.pid.is_some()
            || node.app_id.is_some()
            || node.window_properties.is_some())
}

fn window_class(node: &Node) -> Option<String> {
    node.app_id
        .clone()
        .or_else(|| {
            node.window_properties
                .as_ref()
                .and_then(|properties| properties.class.clone())
        })
        .filter(|class| !class.is_empty())
}

/// Window count and, when icons are enabled, the window classes of every
/// workspace in the tree, keyed by workspace name.
fn workspace_contents(
    tree: Option<&Node>,
    collect_classes: bool,
) -> HashMap<&str, (u16, Vec<String>)> {
    let Some(tree) = tree else {
        return HashMap::new();
    };

    tree.iter()
        .filter(|node| matches!(node.node_type, NodeType::Workspace))
        .filter_map(|workspace| {
            let name = workspace.name.as_deref()?;
            let mut windows: u16 = 0;
            let mut classes = Vec::new();

            for view in workspace.iter().filter(|node| is_view(node)) {
                windows = windows.saturating_add(1);
                if collect_classes && let Some(class) = window_class(view) {
                    classes.push(class);
                }
            }

            Some((name, (windows, classes)))
        })
        .collect()
}

fn build_state(snapshot: &SwaySnapshot) -> CompositorState {
    let collect_classes = super::should_collect_window_classes();

    let outputs: Vec<&str> = snapshot
        .outputs
        .iter()
        .filter(|output| output.active)
        .map(|output| output.name.as_str())
        .sorted()
        .collect();
    let output_ids: HashMap<&str, i128> = outputs
        .iter()
        .enumerate()
        .map(|(idx, name)| (*name, idx as i128))
        .collect();

    {
        let mut names = output_names().lock().unwrap();
        *names = outputs.iter().map(|name| (*name).to_owned()).collect();
    }

    let mut contents = workspace_contents(snapshot.tree.as_ref(), collect_classes);

    let mut registry = workspace_registry().lock().unwrap();
    registry.retain(snapshot.workspaces.iter().map(|ws| ws.name.as_str()));

    // `index` drives the bar ordering and has to stay small and dense, because
    // `enable_workspace_filling` fills every gap below the highest index. Sway
    // reports `num == -1` for workspaces whose name does not start with a
    // number; those are appended after the numbered ones in reply order.
    let highest_num = snapshot
        .workspaces
        .iter()
        .map(|ws| ws.num)
        .max()
        .unwrap_or(0)
        .max(0);
    let mut named = 0;

    let mut workspaces = Vec::with_capacity(snapshot.workspaces.len());
    for ws in &snapshot.workspaces {
        let index = if ws.num >= 0 {
            ws.num
        } else {
            named += 1;
            highest_num.saturating_add(named)
        };
        let (windows, window_classes) = contents.remove(ws.name.as_str()).unwrap_or_default();

        workspaces.push(CompositorWorkspace {
            id: registry.id_for(&ws.name, ws.num),
            index,
            name: ws.name.clone(),
            monitor: ws.output.clone(),
            monitor_id: output_ids.get(ws.output.as_str()).copied(),
            windows,
            is_special: false,
            has_urgent: ws.urgent,
            window_classes,
        });
    }
    drop(registry);

    workspaces.sort_by_key(|ws| (ws.monitor_id.unwrap_or(i128::MAX), ws.index));

    let workspace_id = |name: &str| workspaces.iter().find(|ws| ws.name == name).map(|ws| ws.id);

    let monitors: Vec<CompositorMonitor> = outputs
        .iter()
        .enumerate()
        .map(|(idx, name)| {
            // The workspace shown on this output. `focused` is global, so it
            // would leave every unfocused output without a visible workspace.
            let visible = snapshot
                .outputs
                .iter()
                .find(|output| output.name == *name)
                .and_then(|output| output.current_workspace.as_deref())
                .or_else(|| {
                    snapshot
                        .workspaces
                        .iter()
                        .find(|ws| ws.visible && ws.output == *name)
                        .map(|ws| ws.name.as_str())
                });

            CompositorMonitor {
                id: idx as i128,
                name: (*name).to_owned(),
                active_workspace_id: visible.and_then(workspace_id).unwrap_or(-1),
                special_workspace_id: -1,
            }
        })
        .collect();

    // Sway focuses exactly one workspace at a time.
    let active_workspace_ids = snapshot
        .workspaces
        .iter()
        .filter(|ws| ws.focused)
        .filter_map(|ws| workspace_id(&ws.name))
        .collect();

    // Follow the focus chain instead of looking for `focused`: while one of
    // ashell's own menus holds keyboard focus no node is focused, and blanking
    // the title module then would just be a flicker.
    let active_window = snapshot
        .tree
        .as_ref()
        .and_then(|tree| tree.find_focused_as_ref(is_view))
        .map(|node| {
            ActiveWindow::Sway(ActiveWindowSway {
                title: node.name.clone().unwrap_or_default(),
                class: window_class(node).unwrap_or_default(),
                address: node.id.to_string(),
            })
        });

    // Power buttons, lid switches and virtual keyboards are reported as
    // keyboards too, usually with a single default layout; prefer the device
    // that actually has layouts configured, since that is the one being
    // switched. `min_by_key` keeps the first device on a tie.
    let keyboard_layout = snapshot
        .inputs
        .iter()
        .filter(|input| input.input_type == "keyboard")
        .filter_map(|input| {
            let layout = input.xkb_active_layout_name.as_deref()?;
            (!layout.is_empty()).then_some((input.xkb_layout_names.len(), layout))
        })
        .min_by_key(|(layouts, _)| std::cmp::Reverse(*layouts))
        .map_or_else(|| "Unknown".to_string(), |(_, layout)| layout.to_owned());

    let submap = snapshot
        .binding_state
        .as_ref()
        .map(|state| state.name.as_str())
        .filter(|name| *name != "default")
        .map(str::to_owned);

    CompositorState {
        workspaces,
        monitors,
        active_workspace_ids,
        active_window,
        keyboard_layout,
        submap,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECT: &str = r#"{"x":0,"y":0,"width":1920,"height":1080}"#;

    fn node(id: i64, name: &str, node_type: &str, extra: &str, children: &str) -> String {
        format!(
            r#"{{"id":{id},"name":"{name}","type":"{node_type}","border":"pixel",
            "current_border_width":2,"layout":"splith","orientation":"horizontal","percent":1.0,
            "rect":{RECT},"window_rect":{RECT},"deco_rect":{RECT},"geometry":{RECT},
            "urgent":false,"focused":false,"floating":null,"floating_nodes":[],"sticky":false,
            "nodes":[{children}]{extra}}}"#
        )
    }

    /// eDP-1 shows workspace "1" (focused, one firefox view inside a split
    /// container), HDMI-A-1 shows named workspace "chat" (visible, one xdg-shell
    /// view without app_id plus one floating XWayland view).
    fn tree() -> String {
        let firefox = node(
            100,
            "Mozilla Firefox",
            "con",
            r#","focus":[],"pid":10,"shell":"xdg_shell","app_id":"firefox""#,
            "",
        );
        let split = node(90, "", "con", r#","focus":[100]"#, &firefox);
        let ws1 = node(10, "1", "workspace", r#","focus":[90],"num":1"#, &split);

        let no_app_id = node(
            200,
            "some dialog",
            "con",
            r#","focus":[],"pid":20,"shell":"xdg_shell""#,
            "",
        );
        let xwayland = node(
            201,
            "xterm",
            "floating_con",
            r#","focus":[],"pid":21,"shell":"xwayland",
            "window":42,"window_properties":{"title":"xterm","class":"XTerm","instance":"xterm",
            "window_role":null,"window_type":"normal","transient_for":null}"#,
            "",
        );
        let ws_chat = format!(
            r#"{{"id":11,"name":"chat","type":"workspace","border":"none","current_border_width":0,
            "layout":"splith","orientation":"horizontal","percent":null,"rect":{RECT},
            "window_rect":{RECT},"deco_rect":{RECT},"geometry":{RECT},"urgent":false,
            "focused":false,"focus":[200],"sticky":false,"num":-1,
            "nodes":[{no_app_id}],"floating_nodes":[{xwayland}]}}"#
        );

        let edp = node(2, "eDP-1", "output", r#","focus":[10]"#, &ws1);
        let hdmi = node(3, "HDMI-A-1", "output", r#","focus":[11]"#, &ws_chat);
        node(
            1,
            "root",
            "root",
            r#","focus":[2,3]"#,
            &format!("{edp},{hdmi}"),
        )
    }

    fn workspaces_json() -> &'static str {
        r#"[
          {"id":10,"num":1,"name":"1","visible":true,"focused":true,"urgent":false,
           "representation":"H[firefox]","rect":{"x":0,"y":0,"width":1920,"height":1080},
           "output":"eDP-1","focus":[90]},
          {"id":11,"num":-1,"name":"chat","visible":true,"focused":false,"urgent":true,
           "representation":null,"rect":{"x":1920,"y":0,"width":1920,"height":1080},
           "output":"HDMI-A-1","focus":[200]},
          {"id":12,"num":3,"name":"3:web","visible":false,"focused":false,"urgent":false,
           "representation":null,"rect":{"x":0,"y":0,"width":1920,"height":1080},
           "output":"eDP-1","focus":[]},
          {"id":13,"num":3,"name":"3:mail","visible":false,"focused":false,"urgent":false,
           "representation":null,"rect":{"x":0,"y":0,"width":1920,"height":1080},
           "output":"eDP-1","focus":[]}
        ]"#
    }

    fn outputs_json() -> &'static str {
        r#"[
          {"id":2,"name":"eDP-1","make":"m","model":"x","serial":"s","active":true,"primary":false,
           "scale":1.0,"subpixel_hinting":"unknown","transform":"normal","current_workspace":"1",
           "modes":[],"current_mode":null,"rect":{"x":0,"y":0,"width":1920,"height":1080},
           "focused":true,"dpms":true,"power":true},
          {"id":3,"name":"HDMI-A-1","make":"m","model":"x","serial":"s","active":true,
           "primary":false,"scale":1.0,"subpixel_hinting":"unknown","transform":"normal",
           "current_workspace":"chat","modes":[],"current_mode":null,
           "rect":{"x":1920,"y":0,"width":1920,"height":1080},"focused":false,"dpms":true,
           "power":true},
          {"id":null,"name":"DP-9","make":"m","model":"x","serial":"s","active":false,
           "primary":false,"scale":null,"subpixel_hinting":null,"transform":null,
           "current_workspace":null,"modes":[],"current_mode":null,"focused":false}
        ]"#
    }

    fn inputs_json() -> &'static str {
        r#"[
          {"identifier":"0:0:Power_Button","name":"Power Button","type":"keyboard",
           "xkb_active_layout_name":"English (US)","xkb_layout_names":["English (US)"],
           "xkb_active_layout_index":0,"vendor":0,"product":1,"libinput":null},
          {"identifier":"1:1:AT_keyboard","name":"AT Translated Set 2 keyboard","type":"keyboard",
           "xkb_active_layout_name":"German","xkb_layout_names":["English (US)","German"],
           "xkb_active_layout_index":1,"vendor":1,"product":1,"libinput":null},
          {"identifier":"2:2:mouse","name":"mouse","type":"pointer","vendor":2,"product":2,
           "libinput":null}
        ]"#
    }

    fn snapshot() -> SwaySnapshot {
        SwaySnapshot {
            workspaces: serde_json::from_str(workspaces_json()).expect("workspaces"),
            outputs: serde_json::from_str(outputs_json()).expect("outputs"),
            tree: Some(serde_json::from_str(&tree()).expect("tree")),
            inputs: serde_json::from_str(inputs_json()).expect("inputs"),
            binding_state: Some(serde_json::from_str(r#"{"name":"resize"}"#).expect("binding")),
        }
    }

    fn find<'a>(state: &'a CompositorState, name: &str) -> &'a CompositorWorkspace {
        state
            .workspaces
            .iter()
            .find(|ws| ws.name == name)
            .unwrap_or_else(|| panic!("workspace {name} missing"))
    }

    #[test]
    fn maps_a_multi_monitor_sway_state() {
        super::super::set_collect_window_classes(true);
        let state = build_state(&snapshot());

        // numbered workspaces keep their sway number as id and index
        assert_eq!((find(&state, "1").id, find(&state, "1").index), (1, 1));
        assert_eq!(
            (find(&state, "3:web").id, find(&state, "3:web").index),
            (3, 3)
        );

        // two workspaces sharing a number still get distinct positive ids
        let dup = find(&state, "3:mail");
        assert!(dup.id >= NAMED_ID_BASE, "got {}", dup.id);
        assert_eq!(dup.index, 3);

        // a named workspace sorts after the numbered ones, with a dense index
        let chat = find(&state, "chat");
        assert!(chat.id >= NAMED_ID_BASE, "got {}", chat.id);
        assert_eq!(chat.index, 4);

        // monitor ids follow the sorted list of *active* outputs, so the
        // uppercase HDMI-A-1 comes before eDP-1 and the disabled DP-9 is skipped
        assert_eq!(find(&state, "1").monitor_id, Some(1));
        assert_eq!(chat.monitor_id, Some(0));
        assert_eq!(
            state
                .monitors
                .iter()
                .map(|m| m.name.as_str())
                .collect::<Vec<_>>(),
            vec!["HDMI-A-1", "eDP-1"],
        );

        // every output reports the workspace visible on it, not just the focused one
        let hdmi = &state.monitors[0];
        let edp = &state.monitors[1];
        assert_eq!(hdmi.active_workspace_id, chat.id);
        assert_eq!(edp.active_workspace_id, 1);

        // only the focused workspace is "active"
        assert_eq!(state.active_workspace_ids, vec![1]);

        // urgency is carried over
        assert!(chat.has_urgent);
        assert!(!find(&state, "1").has_urgent);

        // views are counted through split containers; a split con is not a window
        assert_eq!(find(&state, "1").windows, 1);
        assert_eq!(
            find(&state, "1").window_classes,
            vec!["firefox".to_string()]
        );
        // an xdg-shell view without app_id counts, and an XWayland view falls
        // back to window_properties.class
        assert_eq!(chat.windows, 2);
        assert_eq!(chat.window_classes, vec!["XTerm".to_string()]);
        assert_eq!(find(&state, "3:web").windows, 0);

        // the focused view is found through the focus chain
        let window = state.active_window.expect("active window");
        assert_eq!(window.title(), "Mozilla Firefox");
        assert_eq!(window.class(), "firefox");

        // the power button does not win over the real keyboard
        assert_eq!(state.keyboard_layout, "German");
        assert_eq!(state.submap.as_deref(), Some("resize"));
    }

    #[test]
    fn ids_stay_stable_and_commands_resolve() {
        let first = build_state(&snapshot());
        let second = build_state(&snapshot());
        assert_eq!(
            first
                .workspaces
                .iter()
                .map(|ws| (ws.name.clone(), ws.id))
                .collect::<Vec<_>>(),
            second
                .workspaces
                .iter()
                .map(|ws| (ws.name.clone(), ws.id))
                .collect::<Vec<_>>(),
        );

        // every id maps back to the name a `workspace` command needs
        let registry = workspace_registry().lock().unwrap();
        for ws in &second.workspaces {
            assert_eq!(registry.name_for(ws.id), Some(ws.name.as_str()));
        }
        drop(registry);

        assert_eq!(quote(r#"we"ird\ws"#), r#""we\"ird\\ws""#);
    }

    #[test]
    fn survives_a_missing_tree_and_binding_state() {
        let mut snapshot = snapshot();
        snapshot.tree = None;
        snapshot.binding_state = None;
        snapshot.inputs = Vec::new();

        let state = build_state(&snapshot);
        assert_eq!(state.workspaces.len(), 4);
        assert_eq!(state.active_workspace_ids, vec![1]);
        assert!(state.active_window.is_none());
        assert!(state.submap.is_none());
        assert_eq!(state.keyboard_layout, "Unknown");
        assert!(state.workspaces.iter().all(|ws| ws.windows == 0));
    }

    #[test]
    fn a_node_without_urgent_still_parses_as_a_frame_payload() {
        // sway built without xwayland omits `urgent`; the tree then fails to
        // deserialize, which must stay a warning rather than a dead listener.
        let without_urgent = tree().replace(r#""urgent":false,"#, "");
        assert!(serde_json::from_str::<Node>(&without_urgent).is_err());
        assert!(optional::<Node>(Err(anyhow!("boom"))).is_none());
    }

    #[test]
    fn workspace_registry_prunes_dead_workspaces() {
        let mut registry = WorkspaceRegistry::default();
        assert_eq!(registry.id_for("1", 1), 1);
        let chat = registry.id_for("chat", -1);
        assert_eq!(chat, NAMED_ID_BASE);

        registry.retain(["1"].into_iter());
        assert_eq!(registry.name_for(chat), None);
        assert_eq!(registry.name_for(1), Some("1"));
        // the freed id is handed out again
        assert_eq!(registry.id_for("later", -1), NAMED_ID_BASE);
    }
}
