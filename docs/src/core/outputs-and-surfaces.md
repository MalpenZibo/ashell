# Outputs and Surface Management

The output and surface management is defined in `src/outputs.rs`. It handles multi-monitor support and layer surface creation.

## The Outputs Struct

```rust
pub struct Outputs(Vec<(String, Option<ShellInfo>, Option<WlOutput>)>);
```

Each entry in the vector represents a known monitor:

| Field | Type | Description |
|-------|------|-------------|
| Name | `String` | Monitor name (e.g., `"eDP-1"`) or `"Fallback"` |
| ShellInfo | `Option<ShellInfo>` | Layer surfaces for this output (if active) |
| WlOutput | `Option<WlOutput>` | Wayland output object (if discovered) |

## ShellInfo

```rust
pub struct ShellInfo {
    pub id: Id,                  // Main surface window ID
    pub position: Position,      // Top or Bottom
    pub layer: config::Layer,    // Wayland layer
    pub style: AppearanceStyle,  // Bar style
    pub menu: Menu,              // Menu surface state
    pub scale_factor: f64,
}
```

## Surface Creation

Each output gets two layer surfaces created via `create_output_layers()`:

```rust
pub fn create_output_layers(
    style: AppearanceStyle,
    wl_output: Option<WlOutput>,
    position: Position,
    layer: config::Layer,
    scale_factor: f64,
) -> (Id, Id, Task<Message>) {
    // Main layer: "ashell-main-layer"
    //   - Anchored to top or bottom edge + left + right
    //   - Exclusive zone = bar height (reserves screen space)
    //   - Keyboard interactivity: None

    // Menu layer: "ashell-menu-layer"
    //   - Anchored to all edges (fullscreen)
    //   - No exclusive zone
    //   - Starts on Background layer (invisible)
    //   - Keyboard interactivity: None (until menu opens)
}
```

## HasOutput Enum

Used in `App::view()` to determine what to render for a given window ID:

```rust
pub enum HasOutput<'a> {
    Main,                                        // Render the bar
    Menu(Option<&'a (MenuType, ButtonUIRef)>),   // Render the menu (if open)
}
```

## Sync on Config Change

When the config changes, `Outputs::sync()` reconciles the current surfaces with the new configuration:

- Creates surfaces for newly targeted outputs
- Destroys surfaces for outputs no longer targeted
- Updates position, layer, and style for existing surfaces

## Adding and Removing Outputs

When Wayland reports output events:

- **Output added**: If the output matches the config filter (All/Active/Targets), create surfaces for it.
- **Output removed**: Destroy the associated surfaces.
- **Fallback**: If no specific outputs match, the fallback surface is used.
