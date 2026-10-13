# Data Flow: Messages, Tasks, and Subscriptions

## The Message Enum

All events in ashell flow through a single `Message` enum defined in `src/app/message.rs`:

```rust
pub enum Message {
    // Config
    ConfigChanged(Box<Config>),

    // Menu management
    ToggleMenu(MenuType, SurfaceId, ButtonUIRef),
    CloseMenu(SurfaceId),
    FinishCloseMenu(SurfaceId),
    CloseAllMenus,

    // Module messages (one variant per module)
    Custom(String, custom_module::Message),
    Updates(modules::updates::Message),
    Workspaces(modules::workspaces::Message),
    WindowTitle(modules::window_title::Message),
    SystemInfo(modules::system_info::Message),
    KeyboardLayout(modules::keyboard_layout::Message),
    KeyboardSubmap(modules::keyboard_submap::Message),
    Tray(modules::tray::Message),
    Tempo(modules::tempo::Message),
    Privacy(modules::privacy::Message),
    Settings(modules::settings::Message),
    MediaPlayer(modules::media_player::Message),
    Notifications(modules::notifications::Message),

    // OSD and IPC
    Osd(osd::Message),
    IpcOsdCommand(IpcCommand),

    // System events
    OutputEvent(OutputEvent),
    ResumeFromSleep,
    ToggleVisibility,
    None,
}
```

Each module defines its own `Message` type (e.g., `modules::tempo::Message`), which is wrapped in the top-level `Message` enum. This pattern keeps module logic self-contained while enabling centralized routing.

## Message Lifecycle

A typical message flows through the system like this:

```
1. External Event (D-Bus signal, timer tick, user click)
       │
2.     ▼ Subscription produces Message
   ServiceEvent::Update(data)
       │
3.     ▼ Module subscription maps to top-level Message
   Message::Settings(settings::Message::Audio(audio::Message::Event(event)))
       │
4.     ▼ App::update() matches on Message variant
   Delegates to self.settings.update(msg)
       │
5.     ▼ Module update() processes the message
   Returns Action or Task
       │
6.     ▼ App interprets the Action
   May produce more Tasks (e.g., close menu, send command to service)
```

## Tasks vs. Subscriptions

| Concept | Purpose | Lifetime | Example |
|---------|---------|----------|---------|
| **Task** | One-shot side effect | Runs once, produces one Message | Setting brightness, switching workspace |
| **Subscription** | Ongoing event stream | Runs for the lifetime of the app | Watching for compositor events, timer ticks |

**Tasks** are returned from `update()`:

```rust
// Example: batching multiple tasks
Task::batch(vec![
    task.map(Message::Settings),
    self.outputs.request_keyboard(id),
])
```

**Subscriptions** are returned from `subscription()`:

```rust
// Example: sampler threads keyed on config (system_info module)
Subscription::batch([
    Subscription::run_with(
        (self.config.interval, self.config.temperature.sensor.clone()),
        stats_sampler,
    ),
    Subscription::run_with(
        (self.config.interval, self.config.disk.mounts.clone()),
        disks_sampler,
    ),
])
```

`run_with` identifies a subscription by its key, so the sampler threads keep running across
config reloads and restart only when the key changes. `system_info` samples on dedicated threads
because sysinfo calls block: `statvfs` on an unreachable network mount can hang for minutes, and
anything that blocks in `update()` or `view()` freezes the whole bar.

## The ServiceEvent Pattern

Services communicate with modules through a standard `ServiceEvent<S>` enum:

```rust
pub enum ServiceEvent<S: ReadOnlyService> {
    Init(S),                    // Initial state when service starts
    Update(S::UpdateEvent),     // Incremental state update
    Error(S::Error),            // Service error
}
```

A module's subscription typically looks like:

```rust
// In the workspaces module; App maps the result into Message::Workspaces
CompositorService::subscribe().map(|event| Message::ServiceEvent(Box::new(event)))
```

## The Action Pattern

Some modules return an `Action` enum from their `update()` method instead of (or in addition to) a `Task`. Actions are interpreted by `App::update()` to perform cross-cutting operations:

```rust
// Example: settings module action
pub enum Action {
    None,
    Command(Task<Message>),
    CloseMenu(SurfaceId),
    RequestKeyboardWithCommand(SurfaceId, Task<Message>),
    ReleaseKeyboard(SurfaceId),
    ReleaseKeyboardWithCommand(SurfaceId, Task<Message>),
    OpenTooltipMenu(SurfaceId, MenuType, ButtonUIRef),
    CloseTooltipMenu(SurfaceId, MenuType),
}
```

This pattern allows modules to request operations they can't perform themselves (like closing the menu or changing keyboard interactivity), while keeping the module decoupled from the `App` internals.

## Event Sources

ashell subscribes to many event sources:

| Source | Mechanism | Produces |
|--------|-----------|----------|
| Compositor (Hyprland/Niri) | IPC socket | Workspace changes, window focus, keyboard layout |
| Compositor (MangoWC) | `mmsg` IPC CLI | Workspace (tag) changes, window focus, keyboard layout |
| Compositor (generic fallback) | Wayland protocols (`ext-workspace-v1`, `wlr-foreign-toplevel-management`) on a dedicated thread | Workspace changes, active window |
| PulseAudio | libpulse mainloop on dedicated thread | Volume changes, device hotplug |
| D-Bus (BlueZ, NM, UPower, etc.) | zbus signal watchers | Device state changes |
| Config file | inotify | `ConfigChanged` |
| System signals | signal-hook | `SIGUSR1` → `ToggleVisibility` |
| Timers | iced `time::every`, `tokio::time::interval` | Periodic updates (Tempo, system info) |
| Wayland | `iced::output_events()` | Output add/remove (`OutputEvent`) |
| systemd-logind | D-Bus | Sleep/wake (`ResumeFromSleep`) |
