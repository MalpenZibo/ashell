# Outputs and Surface Management

The output and surface management is defined in `src/outputs.rs`. It handles multi-monitor support and layer surface creation.

## The Outputs Struct

```rust
pub struct Outputs {
    entries: Vec<(OutputKey, Option<ShellInfo>, Option<OutputId>)>,
    toast: Option<OverlaySurface>,      // Notification toast surface (if shown)
    osd: Option<OverlaySurface>,        // OSD surface (if shown)
    // ...
    visibility: BarVisibility,          // Shown or Hidden (toggled via SIGUSR1/IPC)
}
```

Each entry in `entries` represents a known monitor:

| Field | Type | Description |
|-------|------|-------------|
| Key | `OutputKey` | Monitor `name` (e.g., `"eDP-1"`, or `"Fallback"`) and `description` (name + make + model, used for matching configured targets) |
| ShellInfo | `Option<ShellInfo>` | Layer surfaces for this output (if active) |
| OutputId | `Option<OutputId>` | Wayland output ID (if discovered) |

## ShellInfo

```rust
pub struct ShellInfo {
    pub id: SurfaceId,           // Main surface ID
    pub position: Position,      // Top or Bottom
    pub layer: config::Layer,    // Wayland layer
    pub layout: BarLayout,       // Surface mode + resolved outer margin
    pub menu: Menu,              // Menu surface state
    pub scale_factor: f64,
    pub output_logical_height: Option<u32>, // For computing toast input regions
}
```

## Surface Creation

Each output gets a bar layer surface created via `create_output_layers()`:

```rust
fn create_output_layers<Message: 'static>(
    layout: BarLayout,
    output_id: Option<OutputId>,
    position: Position,
    layer: config::Layer,
    scale_factor: f64,
    visibility: BarVisibility,
) -> (SurfaceId, Task<Message>) {
    // Main layer: "ashell-main-layer"
    //   - Anchored to top or bottom edge + left + right
    //   - Exclusive zone = bar height + margin (0 when hidden)
    //   - Keyboard interactivity: None
}
```

The geometry (anchor, size, exclusive zone, margin, input region) comes from `Outputs::bar_geometry()`. Menu surfaces are not created up front: a fullscreen `"ashell-menu-layer"` surface is created when a menu opens and destroyed when it closes (see [Menu System](menu-system.md)). The notification toast and OSD are separate overlay surfaces, created on demand by `show_toast_layer()` / `show_osd_layer()`.

## HasOutput Enum

Used in `App::view()` to determine what to render for a given surface ID:

```rust
pub enum HasOutput<'a> {
    Main,                       // Render the bar
    Menu(Option<&'a OpenMenu>), // Render the menu (if open)
    Toast,                      // Render notification toasts
    Osd,                        // Render the OSD
}
```

## Sync on Config Change

When the config changes, `Outputs::sync()` reconciles the current surfaces with the new configuration:

- Creates surfaces for newly targeted outputs
- Destroys surfaces for outputs no longer targeted
- Recreates surfaces whose layer changed
- Updates position, layout, and scale factor for existing surfaces

## Adding and Removing Outputs

When Wayland reports output events:

- **Output added**: If the output matches the config filter (All/Active/Targets), create surfaces for it.
- **Output removed**: Destroy the associated surfaces.
- **Fallback**: At startup a `"Fallback"` bar surface (not bound to an output) is used until real outputs are detected; the first added output replaces it. If removing an output leaves no bar surfaces, a new fallback surface is created.
