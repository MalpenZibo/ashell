# Centerbox

`src/widgets/centerbox.rs`

## Purpose

The Centerbox is a three-column horizontal layout widget. Unlike iced's `Row`, it guarantees that the center element is truly centered on the screen, regardless of the widths of the left and right elements.

## How It Works

```
┌──────────────┬──────────────┬──────────────┐
│    Left      │    Center    │    Right     │
│  (shrink)    │  (centered)  │  (shrink)    │
└──────────────┴──────────────┴──────────────┘
```

The layout algorithm:
1. Measures the left and right children
2. Centers the middle child in the remaining space
3. Ensures the center stays at the true horizontal midpoint, even if left and right have different widths

## API

```rust
pub struct Centerbox<'a, Message, Theme, Renderer> {
    children: [Element<'a, Message, Theme, Renderer>; 3],
    // ...
}

impl Centerbox {
    pub fn new(children: [Element; 3]) -> Self;
    pub fn spacing(self, amount: impl Into<Pixels>) -> Self;
    pub fn padding(self, padding: impl Into<Padding>) -> Self;
    pub fn width(self, width: impl Into<Length>) -> Self;
    pub fn height(self, height: impl Into<Length>) -> Self;
    pub fn align_y(self, align: Alignment) -> Self;
}
```

## Usage in ashell

The Centerbox is used as the main bar layout:

```rust
// In App::view()
Centerbox::new(self.modules_section(id, &self.theme))
    .width(Length::Fill)
    .height(Length::Fill)
    .align_y(Alignment::Center)
```

Where `modules_section()` returns `[left_modules, center_modules, right_modules]`.
