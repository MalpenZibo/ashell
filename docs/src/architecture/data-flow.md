# Data Flow: Messages, Tasks, and Subscriptions

## The Message Enum

All events in ashell flow through a single `Message` enum defined in `src/app.rs`:

```rust
pub enum Message {
    // Config
    ConfigChanged(Box<Config>),

    // Menu management
    ToggleMenu(MenuType, Id, ButtonUIRef),
    CloseMenu(Id),
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
    Clock(modules::clock::Message),
    Tempo(modules::tempo::Message),
    Privacy(modules::privacy::Message),
    Settings(modules::settings::Message),
    MediaPlayer(modules::media_player::Message),

    // System events
    OutputEvent((OutputEvent, WlOutput)),
    ResumeFromSleep,
    ToggleVisibility,
    None,
}
```

Each module defines its own `Message` type (e.g., `modules::clock::Message`), which is wrapped in the top-level `Message` enum. This pattern keeps module logic self-contained while enabling centralized routing.

## Message Lifecycle

A typical message flows through the system like this:

```
1. External Event (D-Bus signal, timer tick, user click)
       │
2.     ▼ Subscription produces Message
   ServiceEvent::Update(data)
       │
3.     ▼ Module subscription maps to top-level Message
   Message::Settings(settings::Message::Audio(audio::Message::ServiceUpdate(event)))
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
    menu.close(),
    set_layer(id, Layer::Background),
])
```

**Subscriptions** are returned from `subscription()`:

```rust
// Example: timer-based subscription
every(Duration::from_secs(1)).map(|_| Message::Update)
```

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
CompositorService::subscribe()
    .map(|event| Message::Workspaces(workspaces::Message::CompositorEvent(event)))
```

## The Action Pattern

Some modules return an `Action` enum from their `update()` method instead of (or in addition to) a `Task`. Actions are interpreted by `App::update()` to perform cross-cutting operations:

```rust
// Example: settings module action
pub enum Action {
    None,
    Command(Task<Message>),
    CloseMenu,
    RequestKeyboard,
    ReleaseKeyboard,
    ReleaseKeyboardWithCommand(Task<Message>),
}
```

This pattern allows modules to request operations they can't perform themselves (like closing the menu or changing keyboard interactivity), while keeping the module decoupled from the `App` internals.

## Event Sources

ashell subscribes to many event sources:

| Source | Mechanism | Produces |
|--------|-----------|----------|
| Compositor (Hyprland/Niri) | IPC socket | Workspace changes, window focus, keyboard layout |
| PulseAudio | libpulse mainloop on dedicated thread | Volume changes, device hotplug |
| D-Bus (BlueZ, NM, UPower, etc.) | zbus signal watchers | Device state changes |
| Config file | inotify | `ConfigChanged` |
| System signals | signal-hook | `SIGUSR1` → `ToggleVisibility` |
| Timers | iced `time::every` | Periodic updates (clock, system info) |
| Wayland | Layer shell events | Output add/remove |
| systemd-logind | D-Bus | Sleep/wake (`ResumeFromSleep`) |
