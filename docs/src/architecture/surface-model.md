# Surface Model: Layer Shell and Multi-Monitor

## Wayland Layer Shell

ashell uses the [wlr-layer-shell](https://wayland.app/protocols/wlr-layer-shell-unstable-v1) protocol to position itself as a status bar. Key concepts:

- **Layer surface**: A special Wayland surface that lives in a specific layer (Background, Bottom, Top, Overlay).
- **Anchor**: Where the surface attaches (top, bottom, left, right edges).
- **Exclusive zone**: Space reserved by the bar that other windows won't overlap.

## Surface Architecture

For each monitor output, ashell creates **two** layer surfaces:

```
┌─────────────────────────────────────────┐
│              Monitor Output              │
│                                          │
│  ┌──────────────────────────────────┐   │
│  │     Main Layer Surface           │   │
│  │  (Top/Bottom layer, 34px high)   │   │
│  │  Namespace: "ashell-main-layer"  │   │
│  │  Exclusive zone: yes             │   │
│  │  Keyboard: None                  │   │
│  └──────────────────────────────────┘   │
│                                          │
│  ┌──────────────────────────────────┐   │
│  │     Menu Layer Surface           │   │
│  │  (Background ↔ Overlay layer)    │   │
│  │  Namespace: "ashell-menu-layer"  │   │
│  │  Exclusive zone: no              │   │
│  │  Keyboard: None ↔ OnDemand       │   │
│  └──────────────────────────────────┘   │
│                                          │
└─────────────────────────────────────────┘
```

- **Main surface**: Always visible, displays the bar content. Uses an exclusive zone so windows don't overlap it.
- **Menu surface**: Hidden by default (on Background layer). When a menu opens, it's promoted to Overlay layer. When closed, it's demoted back to Background.

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
pub struct Outputs(Vec<(String, Option<ShellInfo>, Option<WlOutput>)>);
```

Each entry is a tuple of:
- **Name**: Monitor name (e.g., `"eDP-1"`) or `"Fallback"` for the default
- **ShellInfo**: The layer surfaces and their state (if active)
- **WlOutput**: The Wayland output object (if known)

### Lifecycle

1. **Startup**: A fallback surface is created (not tied to any specific output).
2. **Output detected**: When Wayland reports a new output, ashell creates surfaces for it (if it matches the config filter).
3. **Output removed**: Surfaces for that output are destroyed.
4. **Config change**: The `sync` method reconciles surfaces with the new config.

## Menu Layer Switching

When a menu opens:

```rust
// In menu.rs
pub fn open(&mut self, ...) -> Task<Message> {
    self.menu_info.replace((menu_type, button_ui_ref));
    Task::batch(vec![
        set_layer(self.id, Layer::Overlay),           // Promote to top
        set_keyboard_interactivity(self.id, OnDemand), // Enable keyboard (if needed)
    ])
}

pub fn close(&mut self) -> Task<Message> {
    self.menu_info.take();
    Task::batch(vec![
        set_layer(self.id, Layer::Background),        // Hide
        set_keyboard_interactivity(self.id, None),    // Disable keyboard
    ])
}
```

This approach avoids creating and destroying surfaces on every menu toggle, which would be expensive.

## Bar Positioning

The `position` config field (default: `Bottom`) controls where the bar appears:

- `Top`: Anchored to top edge
- `Bottom`: Anchored to bottom edge

The `layer` config field (default: `Bottom`) controls the Wayland layer:

- `Top`: Bar appears above normal windows
- `Bottom`: Bar appears below floating windows (default preference)
- `Overlay`: Bar appears above everything

> **Note**: The `Bottom` layer default was a deliberate choice by the maintainer — the bar sits below floating windows. The `Top` layer option was added for users who prefer the bar always visible, especially in Niri's overview mode.
