# Surface Model: Layer Shell and Multi-Monitor

## Wayland Layer Shell

ashell uses the [wlr-layer-shell](https://wayland.app/protocols/wlr-layer-shell-unstable-v1) protocol to position itself as a status bar. Key concepts:

- **Layer surface**: A special Wayland surface that lives in a specific layer (Background, Bottom, Top, Overlay).
- **Anchor**: Where the surface attaches (top, bottom, left, right edges).
- **Exclusive zone**: Space reserved by the bar that other windows won't overlap.

## Surface Architecture

For each monitor output, ashell creates a main bar surface, plus a menu surface while a menu is open:

```
┌─────────────────────────────────────────┐
│              Monitor Output              │
│                                          │
│  ┌──────────────────────────────────┐   │
│  │     Main Layer Surface           │   │
│  │  (configured layer, 34px high)   │   │
│  │  Namespace: "ashell-main-layer"  │   │
│  │  Exclusive zone: yes             │   │
│  │  Keyboard: None ↔ OnDemand       │   │
│  └──────────────────────────────────┘   │
│                                          │
│  ┌──────────────────────────────────┐   │
│  │     Menu Layer Surface           │   │
│  │  (Overlay layer, on demand)      │   │
│  │  Namespace: "ashell-menu-layer"  │   │
│  │  Exclusive zone: no              │   │
│  │  Keyboard: None/OnDemand/Excl.   │   │
│  └──────────────────────────────────┘   │
│                                          │
└─────────────────────────────────────────┘
```

- **Main surface**: Always present, displays the bar content. Uses an exclusive zone so windows don't overlap it. Its keyboard interactivity is switched to `OnDemand` while a menu is open if `enable_esc_key` is set, so Escape can close the menu.
- **Menu surface**: Created on the Overlay layer when a menu opens, anchored to all four edges, and destroyed when the menu closes. It requests `OnDemand` keyboard interactivity when `enable_esc_key` is set, and `Exclusive` while a dialog (e.g. the password prompt) needs keyboard input.

Two additional overlay surfaces are created on demand and are not tied to a specific output: the toast surface for notifications (`"ashell-toast-layer"`) and the OSD surface (`"ashell-osd-layer"`).

## Multi-Monitor Configuration

The `outputs` config field controls which monitors get a bar:

```toml
# Default: bar on all monitors
outputs = "All"

# Only on the active monitor
outputs = "Active"

# Specific monitors by name
outputs = { Targets = ["eDP-1", "HDMI-A-1"] }
```

### The Outputs Struct

`src/outputs.rs` defines the `Outputs` struct:

```rust
pub struct Outputs {
    entries: Vec<(OutputKey, Option<ShellInfo>, Option<OutputId>)>,
    toast: Option<OverlaySurface>,
    osd: Option<OverlaySurface>,
    // ...
}
```

Each entry is a tuple of:
- **OutputKey**: Monitor name (e.g., `"eDP-1"`, or `"Fallback"` for the default) and description, used to match `outputs` targets
- **ShellInfo**: The bar surface id, its menu, and layout state (if active)
- **OutputId**: The Wayland output identifier (if known)

### Lifecycle

1. **Startup**: The initial surface created by `.layer_shell()` in `main.rs` is used as a fallback (not tied to any specific output).
2. **Output detected**: When Wayland reports a new output, ashell creates surfaces for it (if it matches the config filter).
3. **Output removed**: Surfaces for that output are destroyed.
4. **Config change**: The `sync` method reconciles surfaces with the new config.

## Menu Surface Lifecycle

Menu surfaces are managed by `Menu` in `src/components/menu.rs`:

```rust
pub fn open(
    &mut self,
    menu_type: MenuType,
    button_ui_ref: ButtonUIRef,
    request_keyboard: bool,
    output_id: Option<OutputId>,
) -> Task<app::Message> {
    let (menu_id, task) = new_layer_surface(LayerShellSettings {
        namespace: "ashell-menu-layer".to_string(),
        layer: Layer::Overlay,
        keyboard_interactivity, // OnDemand if request_keyboard, else None
        output: output_id,
        anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
        // ...
    });
    // ...
}

/// Begin the close animation, firing `FinishCloseMenu` once it ends.
pub fn close(&mut self) -> Task<app::Message> { /* ... */ }

/// Destroy the surface after the close animation, opening any queued menu.
pub fn finish_close(&mut self) -> Task<app::Message> { /* ... */ }
```

Closing is two-step: `close()` starts the close animation (100 ms, skipped when animations are disabled) and emits `Message::FinishCloseMenu`, whose handler destroys the surface. Toggling a different menu while one is closing queues it until the close finishes.

## Bar Positioning

The `position` config field (default: `Top`) controls where the bar appears:

- `Top`: Anchored to top edge
- `Bottom`: Anchored to bottom edge

The `layer` config field (default: `Bottom`) controls the Wayland layer:

- `Top`: Bar appears above normal windows
- `Bottom`: Bar appears below floating windows (default preference)
- `Overlay`: Bar appears above everything

> **Note**: With the default `Bottom` layer the bar sits below floating windows. Use `Top` to keep the bar always visible, especially in Niri's overview mode.
