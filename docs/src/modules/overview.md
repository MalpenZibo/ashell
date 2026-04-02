# Modules Overview

Modules are the UI building blocks of ashell. Each module is a self-contained component that renders content in the bar and optionally provides a popup menu.

## Available Modules

| Module | Config Name | Description | Has Menu |
|--------|-------------|-------------|----------|
| Workspaces | `"Workspaces"` | Workspace indicators and switching | No |
| WindowTitle | `"WindowTitle"` | Active window title/class display | No |
| SystemInfo | `"SystemInfo"` | CPU, RAM, disk, network, temperature | Yes |
| KeyboardLayout | `"KeyboardLayout"` | Keyboard layout indicator (click to cycle) | No |
| KeyboardSubmap | `"KeyboardSubmap"` | Hyprland submap display | No |
| Tray | `"Tray"` | System tray icons | Yes (per-app) |
| Clock | `"Clock"` | Simple time display (**deprecated**) | No |
| Tempo | `"Tempo"` | Advanced clock with calendar, weather, timezones | Yes |
| Privacy | `"Privacy"` | Microphone/camera/screenshare indicators | No |
| Settings | `"Settings"` | Settings panel (audio, network, bluetooth, etc.) | Yes |
| MediaPlayer | `"MediaPlayer"` | MPRIS media player control | Yes |
| Updates | `"Updates"` | Package update indicator | Yes |
| Custom | `"Custom:name"` | User-defined modules | No |

## Configuration

Modules are arranged in three bar sections via the config file:

```toml
[modules]
left = ["Workspaces"]
center = ["Tempo"]
right = [["SystemInfo", "Settings"], "Tray"]
```

### Grouping

Modules can be grouped using nested arrays:

```toml
right = [["SystemInfo", "Settings"], "Tray"]
#         └── group ──────────────┘   └── single
```

In the `Islands` bar style, grouped modules share a single background container. In `Solid`/`Gradient` styles, grouping has no visual effect.

The config uses the `ModuleDef` enum:

```rust
pub enum ModuleDef {
    Single(ModuleName),         // "Tempo"
    Group(Vec<ModuleName>),     // ["SystemInfo", "Settings"]
}
```

## Module vs Service

A key architectural distinction:

- **Modules** are UI components. They have a `view()` method that renders iced `Element`s.
- **Services** are backend integrations. They produce events and accept commands but have no UI.

Modules consume services through subscriptions. For example, the `Workspaces` module subscribes to `CompositorService` events to know about workspace changes.

## How Modules Are Rendered

The `modules_section()` method in `src/modules/mod.rs` builds the three bar sections:

```rust
pub fn modules_section(&self, id: Id, theme: &AshellTheme) -> [Element<Message>; 3] {
    // Returns [left_elements, center_elements, right_elements]
    // Each module is wrapped in a button (if interactive) or plain container
}
```

These three sections are placed into a `Centerbox` widget that keeps the center truly centered.
