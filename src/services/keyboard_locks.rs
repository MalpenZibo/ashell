use super::{ReadOnlyService, Service, ServiceEvent};
use evdev::{
    AttributeSet, Device, EventSummary, KeyCode, KeyEvent, LedCode, SynchronizationCode,
    SynchronizationEvent, uinput::VirtualDevice,
};
use iced::{
    Subscription, Task,
    futures::{SinkExt, StreamExt, channel::mpsc::Sender, stream::pending},
    stream::channel,
};
use log::{debug, info, warn};
use std::{
    any::TypeId,
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::{
    io::{Interest, unix::AsyncFd},
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};

/// Aggregate LED state across all keyboards.
///
/// A lock is considered "on" if any monitored keyboard reports it as on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyboardLocksData {
    pub caps_lock: bool,
    pub num_lock: bool,
    pub scroll_lock: bool,
}

/// One of the three keyboard locks we track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockKind {
    Caps,
    Num,
    Scroll,
}

#[derive(Debug, Clone)]
pub struct KeyboardLocksService {
    pub data: KeyboardLocksData,
    commander: UnboundedSender<KeyboardLocksCommand>,
}

#[derive(Debug, Clone)]
pub struct KeyboardLocksEvent(pub KeyboardLocksData);

#[derive(Debug, Clone)]
pub enum KeyboardLocksCommand {
    Toggle(LockKind),
}

impl ReadOnlyService for KeyboardLocksService {
    type UpdateEvent = KeyboardLocksEvent;
    type Error = ();

    fn update(&mut self, event: Self::UpdateEvent) {
        self.data = event.0;
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Subscription::run_with(TypeId::of::<Self>(), |_| {
            channel(100, async |mut output| {
                let mut state = State::Init;
                loop {
                    state = run(state, &mut output).await;
                }
            })
        })
    }
}

impl Service for KeyboardLocksService {
    type Command = KeyboardLocksCommand;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        let _ = self.commander.send(command);
        Task::none()
    }
}

/// Internal per-device update.
struct DeviceUpdate {
    path: PathBuf,
    locks: Option<KeyboardLocksData>,
}

enum State {
    Init,
    Active(UnboundedReceiver<KeyboardLocksCommand>),
    Error,
}

fn input_subsystem_monitor() -> anyhow::Result<AsyncFd<udev::MonitorSocket>> {
    let socket = udev::MonitorBuilder::new()?
        .match_subsystem("input")?
        .listen()?;
    Ok(AsyncFd::with_interest(
        socket,
        Interest::READABLE | Interest::WRITABLE,
    )?)
}

fn enumerate_event_devices() -> io::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in std::fs::read_dir("/dev/input")? {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str())
            && name.starts_with("event")
        {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

/// Try opening the device, retrying briefly to avoid racing with kernel/udev.
fn open_device_with_retry(path: &Path) -> io::Result<Option<Device>> {
    let mut last_err: Option<io::Error> = None;
    for attempt in 0..3 {
        match Device::open(path) {
            Ok(device) => return Ok(Some(device)),
            Err(err) if err.kind() == io::ErrorKind::PermissionDenied => {
                // Don't retry on EACCES — it won't change.
                return Err(err);
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                // Device disappeared between enumeration and open.
                return Ok(None);
            }
            Err(err) => {
                last_err = Some(err);
                if attempt < 2 {
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| io::Error::other("failed to open device")))
}

/// Returns true if the device exposes at least one of the locks we track.
fn has_lock_leds(device: &Device) -> bool {
    device.supported_leds().is_some_and(|leds| {
        leds.contains(LedCode::LED_CAPSL)
            || leds.contains(LedCode::LED_NUML)
            || leds.contains(LedCode::LED_SCROLLL)
    })
}

fn read_lock_state(device: &Device) -> io::Result<KeyboardLocksData> {
    let leds = device.get_led_state()?;
    Ok(KeyboardLocksData {
        caps_lock: leds.contains(LedCode::LED_CAPSL),
        num_lock: leds.contains(LedCode::LED_NUML),
        scroll_lock: leds.contains(LedCode::LED_SCROLLL),
    })
}

/// Spawn a task watching a single keyboard for LED changes.
///
/// Returns `Some(handle)` if the device qualifies (exposes at least one
/// tracked LED) and was opened successfully. Returns `None` if it doesn't
/// qualify or is otherwise uninteresting. On permission errors, logs and
/// returns `None`.
fn spawn_device_task(path: PathBuf, tx: UnboundedSender<DeviceUpdate>) -> Option<JoinHandle<()>> {
    let device = match open_device_with_retry(&path) {
        Ok(Some(device)) => device,
        Ok(None) => return None,
        Err(err) if err.kind() == io::ErrorKind::PermissionDenied => {
            debug!("permission denied opening {}: {err}", path.display());
            return None;
        }
        Err(err) => {
            debug!("failed to open {}: {err}", path.display());
            return None;
        }
    };

    if !has_lock_leds(&device) {
        return None;
    }

    let initial = match read_lock_state(&device) {
        Ok(state) => state,
        Err(err) => {
            debug!("failed to read LED state for {}: {err}", path.display());
            return None;
        }
    };

    let mut stream = match device.into_event_stream() {
        Ok(stream) => stream,
        Err(err) => {
            debug!("failed to start event stream for {}: {err}", path.display());
            return None;
        }
    };

    if tx
        .send(DeviceUpdate {
            path: path.clone(),
            locks: Some(initial),
        })
        .is_err()
    {
        return None;
    }

    let mut current = initial;
    let handle = tokio::spawn(async move {
        loop {
            match stream.next_event().await {
                Ok(event) => {
                    if !matches!(event.destructure(), EventSummary::Led(_, _, _)) {
                        continue;
                    }
                    let device = stream.device();
                    let Ok(new_state) = read_lock_state(device) else {
                        continue;
                    };
                    if new_state != current {
                        current = new_state;
                        if tx
                            .send(DeviceUpdate {
                                path: path.clone(),
                                locks: Some(new_state),
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                }
                Err(err) => {
                    debug!(
                        "event stream error for {}: {err}; closing task",
                        path.display()
                    );
                    let _ = tx.send(DeviceUpdate {
                        path: path.clone(),
                        locks: None,
                    });
                    break;
                }
            }
        }
    });

    Some(handle)
}

fn aggregate(per_device: &HashMap<PathBuf, KeyboardLocksData>) -> KeyboardLocksData {
    let mut acc = KeyboardLocksData::default();
    for state in per_device.values() {
        acc.caps_lock |= state.caps_lock;
        acc.num_lock |= state.num_lock;
        acc.scroll_lock |= state.scroll_lock;
    }
    acc
}

async fn run(state: State, output: &mut Sender<ServiceEvent<KeyboardLocksService>>) -> State {
    match state {
        State::Init => match init().await {
            Ok(initial) => {
                let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                let _ = output
                    .send(ServiceEvent::Init(KeyboardLocksService {
                        data: initial,
                        commander: tx,
                    }))
                    .await;
                State::Active(rx)
            }
            Err(err) => {
                warn!("Keyboard locks service disabled: {err}");
                State::Error
            }
        },
        State::Active(rx) => {
            supervise(output, rx).await;
            // If supervise returns, give up.
            State::Error
        }
        State::Error => {
            let _ = pending::<u8>().next().await;
            State::Error
        }
    }
}

/// Verify that at least one qualifying keyboard is accessible.
///
/// Returns the initial aggregate state on success. Fails (and disables the
/// service) only when *no* device could be opened (all-EACCES or no input
/// nodes found).
async fn init() -> anyhow::Result<KeyboardLocksData> {
    let paths = enumerate_event_devices()?;
    let mut accessible_any = false;
    let mut aggregate_state = KeyboardLocksData::default();

    for path in &paths {
        match open_device_with_retry(path) {
            Ok(Some(device)) => {
                accessible_any = true;
                if !has_lock_leds(&device) {
                    continue;
                }
                if let Ok(state) = read_lock_state(&device) {
                    aggregate_state.caps_lock |= state.caps_lock;
                    aggregate_state.num_lock |= state.num_lock;
                    aggregate_state.scroll_lock |= state.scroll_lock;
                }
            }
            Ok(None) => {}
            Err(err) if err.kind() == io::ErrorKind::PermissionDenied => {
                debug!("permission denied opening {}: {err}", path.display());
            }
            Err(err) => {
                debug!("failed to probe {}: {err}", path.display());
            }
        }
    }

    if !accessible_any {
        return Err(anyhow::anyhow!(
            "no readable input devices (is the user in the `input` group?)"
        ));
    }

    Ok(aggregate_state)
}

/// Run the supervisor: spawn per-device tasks, react to hotplug, emit
/// aggregated updates, and process toggle commands.
async fn supervise(
    output: &mut Sender<ServiceEvent<KeyboardLocksService>>,
    mut commands: UnboundedReceiver<KeyboardLocksCommand>,
) {
    let mut monitor = match input_subsystem_monitor() {
        Ok(monitor) => monitor,
        Err(err) => {
            warn!("Failed to set up input udev monitor: {err}");
            return;
        }
    };

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<DeviceUpdate>();
    let mut per_device: HashMap<PathBuf, KeyboardLocksData> = HashMap::new();
    let mut tasks: HashMap<PathBuf, JoinHandle<()>> = HashMap::new();
    let mut current_aggregate = KeyboardLocksData::default();
    let mut emitter = ToggleEmitter::new();

    // Spawn tasks for already-present devices.
    if let Ok(paths) = enumerate_event_devices() {
        for path in paths {
            if let Some(handle) = spawn_device_task(path.clone(), tx.clone()) {
                tasks.insert(path, handle);
            }
        }
    }

    info!(
        "Keyboard locks service listening (tracking {} devices)",
        tasks.len()
    );

    loop {
        tokio::select! {
            update = rx.recv() => {
                let Some(update) = update else { break };
                handle_device_update(update, &mut per_device, &mut tasks);
                let new_aggregate = aggregate(&per_device);
                if new_aggregate != current_aggregate {
                    current_aggregate = new_aggregate;
                    if output
                        .send(ServiceEvent::Update(KeyboardLocksEvent(current_aggregate)))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
            cmd = commands.recv() => {
                let Some(cmd) = cmd else { break };
                match cmd {
                    KeyboardLocksCommand::Toggle(kind) => emitter.toggle(kind),
                }
            }
            guard = monitor.writable_mut() => {
                match guard {
                    Ok(mut guard) => {
                        let events: Vec<_> = guard.get_inner().iter().collect();
                        for evt in events {
                            handle_udev_event(&evt, &tx, &mut tasks);
                        }
                        guard.clear_ready();
                    }
                    Err(err) => {
                        warn!("input udev monitor failed: {err}");
                        break;
                    }
                }
            }
        }
    }

    // Best-effort cleanup of remaining device tasks.
    for (_, handle) in tasks {
        handle.abort();
    }
}

fn handle_device_update(
    update: DeviceUpdate,
    per_device: &mut HashMap<PathBuf, KeyboardLocksData>,
    tasks: &mut HashMap<PathBuf, JoinHandle<()>>,
) {
    match update.locks {
        Some(state) => {
            per_device.insert(update.path, state);
        }
        None => {
            per_device.remove(&update.path);
            if let Some(handle) = tasks.remove(&update.path) {
                handle.abort();
            }
        }
    }
}

fn handle_udev_event(
    evt: &udev::Event,
    tx: &UnboundedSender<DeviceUpdate>,
    tasks: &mut HashMap<PathBuf, JoinHandle<()>>,
) {
    let Some(devnode) = evt.device().devnode().map(|p| p.to_path_buf()) else {
        return;
    };
    let Some(name) = devnode.file_name().and_then(|n| n.to_str()) else {
        return;
    };
    if !name.starts_with("event") {
        return;
    }

    match evt.event_type() {
        udev::EventType::Add => {
            if tasks.contains_key(&devnode) {
                return;
            }
            if let Some(handle) = spawn_device_task(devnode.clone(), tx.clone()) {
                tasks.insert(devnode, handle);
            }
        }
        udev::EventType::Remove => {
            if let Some(handle) = tasks.remove(&devnode) {
                handle.abort();
            }
            // The per-device task's drop send may not arrive (the channel
            // is unbuffered enough that aborts can race); explicitly signal
            // removal so the aggregator forgets this device.
            let _ = tx.send(DeviceUpdate {
                path: devnode,
                locks: None,
            });
        }
        _ => {}
    }
}

/// Lazily-created uinput emitter for toggling lock keys.
///
/// The virtual device is created on the first toggle request. If creation
/// fails (typically because `/dev/uinput` is not accessible), the emitter
/// permanently disables itself and logs a warning once. Read-only LED
/// monitoring keeps working in either case.
struct ToggleEmitter {
    state: EmitterState,
}

enum EmitterState {
    Uninitialized,
    Ready(VirtualDevice),
    Disabled,
}

impl ToggleEmitter {
    fn new() -> Self {
        Self {
            state: EmitterState::Uninitialized,
        }
    }

    fn toggle(&mut self, kind: LockKind) {
        if matches!(self.state, EmitterState::Uninitialized) {
            match build_virtual_device() {
                Ok(device) => {
                    info!("Keyboard locks: uinput virtual device ready");
                    self.state = EmitterState::Ready(device);
                }
                Err(err) => {
                    warn!(
                        "Keyboard locks: cannot toggle locks: failed to open /dev/uinput: {err}. \
                         The user likely needs write access to /dev/uinput \
                         (e.g. via a udev rule giving the `input` group access)."
                    );
                    self.state = EmitterState::Disabled;
                }
            }
        }

        let EmitterState::Ready(device) = &mut self.state else {
            return;
        };

        let key = match kind {
            LockKind::Caps => KeyCode::KEY_CAPSLOCK,
            LockKind::Num => KeyCode::KEY_NUMLOCK,
            LockKind::Scroll => KeyCode::KEY_SCROLLLOCK,
        };

        let press = *KeyEvent::new(key, 1);
        let release = *KeyEvent::new(key, 0);
        let syn = *SynchronizationEvent::new(SynchronizationCode::SYN_REPORT, 0);

        if let Err(err) = device.emit(&[press, syn, release, syn]) {
            warn!("Keyboard locks: failed to emit {kind:?} toggle: {err}");
        }
    }
}

fn build_virtual_device() -> io::Result<VirtualDevice> {
    let mut keys = AttributeSet::<KeyCode>::new();
    keys.insert(KeyCode::KEY_CAPSLOCK);
    keys.insert(KeyCode::KEY_NUMLOCK);
    keys.insert(KeyCode::KEY_SCROLLLOCK);

    VirtualDevice::builder()?
        .name("ashell keyboard locks")
        .with_keys(&keys)?
        .build()
}
