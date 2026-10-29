# PositionButton

`src/components/position_button.rs`

## Purpose

A button that reports its screen position and size when pressed. This information is needed to position popup menus relative to the button that triggered them.

## Why Not a Regular Button?

iced's built-in `Button` widget emits a message on press, but it doesn't include any information about where the button is on screen. For menu positioning, ashell needs to know the exact screen coordinates of the clicked button.

## API

```rust
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl MouseButton {
    pub const ALL: [Self; 3];
    pub const fn index(self) -> usize;
}

pub enum PressHandler<'a, Message> {
    Message(Message),
    WithPosition(Box<dyn Fn(ButtonUIRef) -> Message + 'a>),
}

impl<'a, Message> PressHandler<'a, Message> {
    pub fn message(message: Message) -> Self;
    pub fn with_position(handler: impl Fn(ButtonUIRef) -> Message + 'a) -> Self;
}

impl PositionButton {
    // One method for every mouse button. PressHandler::WithPosition receives the
    // ButtonUIRef needed for menu placement.
    pub fn on(self, button: MouseButton, handler: PressHandler<'a, Message>) -> Self;

    // Scroll handlers are explicit up/down; there is no generic scroll event.
    pub fn on_scroll_up(self, msg: Message) -> Self;
    pub fn on_scroll_down(self, msg: Message) -> Self;

    // Hover handlers (used for tooltip popups).
    pub fn on_hover(self, handler: PressHandler<'a, Message>) -> Self;
    pub fn on_unhover(self, msg: Message) -> Self;

    // Layout and styling
    pub fn width(self, width: impl Into<Length>) -> Self;
    pub fn height(self, height: impl Into<Length>) -> Self;
    pub fn padding<P: Into<Padding>>(self, padding: P) -> Self;
    pub fn style(self, style: impl Fn(&Theme, Status) -> Style) -> Self;
}
```

Internally the press handlers live in a `[Option<PressHandler>; 3]` array indexed by `MouseButton::index()`, and the pressed state is a `[bool; 3]`. Adding a mouse button means adding an enum variant and one array slot, rather than another parallel field and match arm.

## Notes

- Scroll events are explicit: separate `on_scroll_up` and `on_scroll_down` handlers. There is no generic "scroll" event.
- For custom modules, these handlers map directly to `on_scroll_up` / `on_scroll_down` config fields.
- `PressHandler::WithPosition` builds a message from the button's on-screen position; `PressHandler::Message` is the plain-message case.
- Any mouse button can emit a message or toggle a menu, not just left-click.

## ButtonUIRef

```rust
#[derive(Debug, Clone, Copy)]
pub struct ButtonUIRef {
    pub position: Point, // Screen coordinates of the button's center
}
```

## Usage

```rust
// In modules/mod.rs, ModuleInteractions maps a ModuleAction onto a PressHandler.
// A menu toggle needs the button position, so it becomes WithPosition:
module_item(content).set_handler(
    MouseButton::Left,
    Some(PressHandler::with_position(move |button_ui_ref| {
        Message::ToggleMenu(menu_type.clone(), id, button_ui_ref)
    })),
)
```

The `button_ui_ref` is then used by `MenuWrapper` to position the popup relative to the button.
