# PositionButton

`src/components/position_button.rs`

## Purpose

A button that reports its screen position and size when pressed. This information is needed to position popup menus relative to the button that triggered them.

## Why Not a Regular Button?

iced's built-in `Button` widget emits a message on press, but it doesn't include any information about where the button is on screen. For menu positioning, ashell needs to know the exact screen coordinates of the clicked button.

## API

```rust
pub fn position_button<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> PositionButton<'a, Message, Theme, Renderer>;

impl PositionButton {
    // Standard click: callback receives ButtonUIRef with position info
    pub fn on_press_with_position(self, f: impl Fn(ButtonUIRef) -> Message) -> Self;

    // Standard click without position info
    pub fn on_press(self, msg: Message) -> Self;

    // Right-click handlers
    pub fn on_right_press(self, msg: Message) -> Self;
    pub fn on_right_press_with_position(self, f: impl Fn(ButtonUIRef) -> Message) -> Self;

    // Middle-click handlers
    pub fn on_middle_press(self, msg: Message) -> Self;
    pub fn on_middle_press_with_position(self, f: impl Fn(ButtonUIRef) -> Message) -> Self;

    // Scroll handlers (explicit up/down, not a generic scroll event)
    pub fn on_scroll_up(self, msg: Message) -> Self;
    pub fn on_scroll_down(self, msg: Message) -> Self;

    // Hover handlers (used for tooltip popups)
    pub fn on_hover(self, msg: Message) -> Self;
    pub fn on_hover_with_position(self, f: impl Fn(ButtonUIRef) -> Message) -> Self;
    pub fn on_unhover(self, msg: Message) -> Self;

    // Layout and styling
    pub fn width(self, width: impl Into<Length>) -> Self;
    pub fn height(self, height: impl Into<Length>) -> Self;
    pub fn padding<P: Into<Padding>>(self, padding: P) -> Self;
    pub fn clip(self, clip: bool) -> Self;
    pub fn style(self, style: impl Fn(&Theme, Status) -> Style) -> Self;
}
```

## Notes

- Scroll events are explicit: separate `on_scroll_up` and `on_scroll_down` handlers. There is no generic "scroll" event.
- For custom modules, these handlers map directly to `on_scroll_up` / `on_scroll_down` config fields.
- Middle-click (`on_middle_press`) is primarily used by `CustomAction` (custom modules without menus). `ToggleMenuWithExtra` does not include middle-click support.

## ButtonUIRef

```rust
#[derive(Debug, Clone, Copy)]
pub struct ButtonUIRef {
    pub position: Point,    // Screen coordinates of the button's center
    pub viewport: (f32, f32), // Width and height of the screen
}
```

## Usage

```rust
// In modules/mod.rs (module_item wraps its content in a position_button)
module_item(content)
    .on_press_with_position(move |button_ui_ref| {
        Message::ToggleMenu(menu_type.clone(), id, button_ui_ref)
    })
```

The `button_ui_ref` is then used by `MenuWrapper` to position the popup relative to the button.
