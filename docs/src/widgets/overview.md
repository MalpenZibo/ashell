# Widgets Overview

ashell includes three custom iced widgets in `src/widgets/` that provide functionality not available in iced's built-in widget set.

## Custom Widgets

| Widget | File | Purpose |
|--------|------|---------|
| [Centerbox](centerbox.md) | `widgets/centerbox.rs` | Three-column layout that keeps the center truly centered |
| [PositionButton](position-button.md) | `widgets/position_button.rs` | Button that reports its screen position on press |
| [MenuWrapper](menu-wrapper.md) | `widgets/menu_wrapper.rs` | Menu container with backdrop and click-outside-to-close |

## ButtonUIRef

Defined in `widgets/mod.rs`, this type carries a button's screen position and size:

```rust
#[derive(Debug, Clone, Default)]
pub struct ButtonUIRef {
    pub position: Point,
    pub size: Size,
}
```

This is used by `PositionButton` to tell the menu system where to position popup menus relative to the button that triggered them.

## Why Custom Widgets?

iced provides a rich set of built-in widgets (buttons, text, rows, columns, containers, sliders, etc.), but a status bar has specific needs:

- **Centerbox**: iced's `Row` doesn't guarantee the center element stays centered when left/right content has different widths.
- **PositionButton**: Standard iced buttons don't report their screen position, which is needed for menu placement.
- **MenuWrapper**: No built-in support for modal overlays with backdrop click-to-close.
