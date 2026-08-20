# Compositor Service and Abstraction Layer

The compositor service (`src/services/compositor/`) abstracts over multiple Wayland compositors, with dedicated backends for Hyprland, Niri, MangoWC, and Sway and a generic Wayland fallback for other compositors.

## Architecture

```
services/compositor/
├── mod.rs       # Service trait impl, backend detection, broadcast system
├── types.rs     # CompositorState, CompositorEvent, CompositorCommand, CompositorChoice
├── hyprland.rs  # Hyprland IPC integration
├── niri.rs      # Niri IPC integration
├── mangowc.rs   # MangoWC integration (via the `mmsg` IPC CLI)
├── sway.rs      # Sway integration (i3 IPC over $SWAYSOCK)
└── generic.rs   # Generic Wayland fallback (ext-workspace / wlr-foreign-toplevel)
```

## Backend Detection

The compositor is detected automatically, falling back to the generic backend
when no dedicated one matches:

```rust
fn detect_backend() -> Option<CompositorChoice> {
    if hyprland::is_available() {         // Checks HYPRLAND_INSTANCE_SIGNATURE
        Some(CompositorChoice::Hyprland)
    } else if niri::is_available() {      // Checks NIRI_SOCKET
        Some(CompositorChoice::Niri)
    } else if mangowc::is_available() {   // Probes the `mmsg` IPC CLI
        Some(CompositorChoice::Mango)
    } else if sway::is_available() {      // Checks SWAYSOCK
        Some(CompositorChoice::Sway)
    } else if generic::is_available() {   // ext-workspace / wlr-foreign-toplevel
        Some(CompositorChoice::Generic)
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

### MangoWC (`mangowc.rs`)

Drives MangoWC through its `mmsg` IPC CLI:
- Watches `mmsg -w` for change events and re-derives the full state on each one
- Maps MangoWC tags onto workspaces; since several tags can be active at once,
  it reports them all via `CompositorState::active_workspace_ids`
- Sends commands by shelling out to `mmsg -s`

### Sway (`sway.rs`)

Speaks the i3 IPC protocol directly over the socket named by `$SWAYSOCK`, using
the `swayipc-types` crate for the message and reply types (the socket handling
stays in the backend so it runs on tokio; `swayipc-async` is built on the smol
reactor):
- One connection is subscribed to events and read from a dedicated task, because
  `read_exact` is not cancel safe and a half-read frame would desync the stream
- Event payloads are never decoded: any event coalesces into a full resync, so
  new sway event variants cannot break the backend
- A second, long-lived connection issues the five requests a resync needs
  (`GET_WORKSPACES`, `GET_OUTPUTS`, `GET_TREE`, `GET_INPUTS`, `GET_BINDING_STATE`)
  and is reconnected on failure; a failed resync warns and waits for the next
  event instead of ending the listener
- Workspace ids come from a registry that keeps a numbered workspace's sway
  number as its id (the workspaces module uses the id to index
  `workspace_names`) and hands named workspaces an id from a high range
- Per-monitor visibility comes from `Output::current_workspace`, since
  `Workspace::focused` is global
