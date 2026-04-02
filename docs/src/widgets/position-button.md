# PositionButton

`src/widgets/position_button.rs`

## Purpose

A button that reports its screen position and size when pressed. This information is needed to position popup menus relative to the button that triggered them.

## Why Not a Regular Button?

iced's built-in `Button` widget emits a message on press, but it doesn't include any information about where the button is on screen. For menu positioning, ashell needs to know the exact screen coordinates of the clicked button.

## API

```rust
pub fn position_button<'a>(content: impl Into<Element<'a, Message>>) -> PositionButton<'a, Message>;

impl PositionButton {
    // Standard click: callback receives ButtonUIRef with position info
    pub fn on_press_with_position(self, f: impl Fn(ButtonUIRef) -> Message) -> Self;

    // Standard click without position info
    pub fn on_press(self, msg: Message) -> Self;

    // Right-click handler
    pub fn on_right_press(self, msg: Message) -> Self;

    // Scroll handlers
    pub fn on_scroll_up(self, msg: Message) -> Self;
    pub fn on_scroll_down(self, msg: Message) -> Self;

    // Styling
    pub fn padding(self, padding: impl Into<Padding>) -> Self;
    pub fn height(self, height: impl Into<Length>) -> Self;
    pub fn style(self, style: impl Fn(&Theme, Status) -> button::Style) -> Self;
}
```

## ButtonUIRef

```rust
pub struct ButtonUIRef {
    pub position: Point,    // Screen coordinates of the button's top-left corner
    pub size: Size,         // Width and height of the button
}
```

## Usage

```rust
// In modules/mod.rs
position_button(content)
    .on_press_with_position(move |button_ui_ref| {
        Message::ToggleMenu(MenuType::Settings, output_id, button_ui_ref)
    })
```

The `button_ui_ref` is then used by `MenuWrapper` to position the popup relative to the button.
