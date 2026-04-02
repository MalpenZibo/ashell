# Compositor Service and Abstraction Layer

The compositor service (`src/services/compositor/`) abstracts over multiple Wayland compositors, currently supporting Hyprland and Niri.

## Architecture

```
services/compositor/
├── mod.rs       # Service trait impl, backend detection, broadcast system
├── types.rs     # CompositorState, CompositorEvent, CompositorCommand, CompositorChoice
├── hyprland.rs  # Hyprland IPC integration
└── niri.rs      # Niri IPC integration
```

## Backend Detection

The compositor is detected automatically via environment variables:

```rust
fn detect_backend() -> Option<CompositorChoice> {
    if hyprland::is_available() {         // Checks HYPRLAND_INSTANCE_SIGNATURE
        Some(CompositorChoice::Hyprland)
    } else if niri::is_available() {      // Checks NIRI_SOCKET
        Some(CompositorChoice::Niri)
    } else {
        None
    }
}
```

The detected backend is stored in a global `OnceLock` and never changes during the process lifetime.

## Broadcast Pattern

Unlike other services that use direct channels, the compositor service uses a **broadcast** pattern via `tokio::sync::broadcast`:

```rust
static BROADCASTER: OnceCell<broadcast::Sender<ServiceEvent<CompositorService>>> =
    OnceCell::const_new();
```

This allows multiple subscribers (e.g., Workspaces, WindowTitle, KeyboardLayout modules) to receive the same compositor events without duplication.

### Flow

```
Compositor IPC Socket
    │
    ▼ (single listener thread)
broadcaster_event_loop()
    │
    ▼ broadcast::Sender::send()
    ├── Subscriber 1 (Workspaces module)
    ├── Subscriber 2 (WindowTitle module)
    ├── Subscriber 3 (KeyboardLayout module)
    └── Subscriber 4 (KeyboardSubmap module)
```

Each call to `CompositorService::subscribe()` creates a new `broadcast::Receiver`, getting all events from that point forward.

## CompositorState

The unified state across both backends:

```rust
pub struct CompositorState {
    pub workspaces: Vec<Workspace>,
    pub active_window: Option<WindowInfo>,
    pub keyboard_layout: Option<String>,
    pub keyboard_submap: Option<String>,
    pub monitors: Vec<Monitor>,
}
```

## CompositorEvent

```rust
pub enum CompositorEvent {
    StateChanged(Box<CompositorState>),    // Full state update
    ActionPerformed,                        // Command completed successfully
}
```

## CompositorCommand

Commands that can be sent to the compositor:

```rust
pub enum CompositorCommand {
    FocusWorkspace(WorkspaceId),
    ScrollWorkspace(ScrollDirection),
    ToggleSpecialWorkspace(String),
    NextLayout,
    CustomDispatch(String),
}
```

## Backend Implementations

### Hyprland (`hyprland.rs`)

Uses the `hyprland` crate for IPC communication:
- Connects to Hyprland's Unix domain socket
- Listens for events (workspace changes, window focus, layout changes)
- Sends commands via the dispatcher

### Niri (`niri.rs`)

Uses the `niri-ipc` crate:
- Connects to Niri's IPC socket (path from `NIRI_SOCKET` env var)
- Listens for events and translates them to the common `CompositorEvent` format
- Sends commands via the IPC protocol
