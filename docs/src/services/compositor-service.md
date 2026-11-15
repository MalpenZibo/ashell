# Compositor Service and Abstraction Layer

The compositor service (`src/services/compositor/`) abstracts over multiple Wayland compositors, with dedicated backends for Hyprland, Niri, MangoWC, and Wayfire and a generic Wayland fallback for other compositors.

## Architecture

```
services/compositor/
├── mod.rs       # Service trait impl, backend detection, broadcast system
├── types.rs     # CompositorState, CompositorEvent, CompositorCommand, CompositorChoice
├── hyprland.rs  # Hyprland IPC integration
├── niri.rs      # Niri IPC integration
├── mangowc.rs   # MangoWC integration (via the `mmsg` IPC CLI)
├── wayfire.rs   # Wayfire IPC integration (length-prefixed JSON socket)
└── generic.rs   # Generic Wayland fallback (ext-workspace / wlr-foreign-toplevel)
```

## Backend Detection

The compositor is detected automatically, falling back to the generic backend
when no dedicated one matches:

```rust
fn detect_backend() -> Option<CompositorChoice> {
    if hyprland::is_available() {         // Checks HYPRLAND_INSTANCE_SIGNATURE
        Some(CompositorChoice::Hyprland)
    } else if niri::is_available() {      // Checks NIRI_SOCKET / NIRI_SOCKET_PATH
        Some(CompositorChoice::Niri)
    } else if mangowc::is_available() {   // Probes `mmsg get version`
        Some(CompositorChoice::Mango)
    } else if wayfire::is_available() {   // Checks WAYFIRE_SOCKET
        Some(CompositorChoice::Wayfire)
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
    ├── Subscriber 3 (Minimap module)
    ├── Subscriber 4 (KeyboardLayout module)
    └── Subscriber 5 (KeyboardSubmap module)
```

Each call to `CompositorService::subscribe()` creates a new `broadcast::Receiver`, getting all events from that point forward.

## CompositorState

The unified state across all backends:

```rust
pub struct CompositorState {
    pub workspaces: Vec<CompositorWorkspace>,
    pub monitors: Vec<CompositorMonitor>,
    pub active_workspace_ids: Vec<i32>,
    pub focused_workspace_id: Option<i32>,  // None where there is no global focus
    pub active_window: Option<ActiveWindow>,
    pub windows: Vec<CompositorWindow>,     // per-window layout, filled by Niri only
    pub keyboard_layout: String,
    pub submap: Option<String>,
}
```

## CompositorEvent

```rust
pub enum CompositorEvent {
    ActionPerformed,                        // Command completed successfully
    StateChanged(Box<CompositorState>),    // Full state update
}
```

## CompositorCommand

Commands that can be sent to the compositor:

```rust
pub enum CompositorCommand {
    FocusWorkspace(i32),
    FocusNamedWorkspace(String),
    ToggleSpecialWorkspace(String),
    CustomDispatch(String, String), // For "vdesk"
    NextLayout,
    FocusAppWindow {
        pid: Option<u32>,
        apps: Vec<String>,
        title: Option<String>,
    },
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
- Watches `mmsg watch all-monitors` for change events and rebuilds the full state from each one
- Maps MangoWC tags onto workspaces; since several tags can be active at once,
  it reports them all via `CompositorState::active_workspace_ids`
- Sends commands by shelling out to `mmsg dispatch`

### Wayfire (`wayfire.rs`)

Talks to Wayfire's own IPC socket (`WAYFIRE_SOCKET`, needs the `ipc` and
`ipc-rules` plugins, with `ipc` listed first; workspace switching also needs
`vswitch`). Requires Wayfire 0.9+; the keyboard layout additionally needs
`wayfire/get-keyboard-state`, available since 0.10. The protocol is
length-prefixed (4-byte little-endian) JSON:

- One connection subscribes to events via `window-rules/events/watch`; a burst of
  events is coalesced before the state is re-derived, because Wayfire emits
  several events per user action
- A second, long-lived connection serves the method calls of a state refresh
  (`list-outputs`, `list-views`, `get-focused-output`, `get-focused-view`,
  `wayfire/get-keyboard-state`)
- Workspaces are a per-output 2D grid, so each (output, grid cell) pair becomes
  one ashell workspace. Wayfire has no per-workspace view list: views are
  bucketed into cells from their geometry, which it reports in output-local
  coordinates relative to the output's current workspace

### Generic Wayland (`generic.rs`)

Fallback for compositors without a dedicated backend, using standard Wayland protocols on a single connection:
- `wl_output` for monitors, `ext-workspace-v1` for workspaces, and `wlr-foreign-toplevel-management` for the active window
- Rebuilds the full `CompositorState` on every change
- Only `FocusWorkspace` is supported as a command; others return an error
