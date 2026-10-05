# Centerbox

`src/components/centerbox.rs`

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
3. Keeps the center at the true horizontal midpoint, even if left and right have different widths; only when that would make it overlap a side child does it fall back to centering in the space left between them
4. When `animated` is enabled (the default), changes of the center's x-position are animated (100ms ease-out) instead of snapping

## API

```rust
pub struct Centerbox<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    children: [Element<'a, Message, Theme, Renderer>; 3],
    // ...
}

impl Centerbox {
    pub fn new(children: [Element; 3]) -> Self;
    pub fn animated(self, animated: bool) -> Self;
    pub fn spacing(self, amount: impl Into<Pixels>) -> Self;
    pub fn padding<P: Into<Padding>>(self, padding: P) -> Self;
    pub fn width(self, width: impl Into<Length>) -> Self;
    pub fn height(self, height: impl Into<Length>) -> Self;
    pub fn align_items(self, align: Alignment) -> Self;
}
```

## Usage in ashell

The Centerbox is used as the main bar layout:

```rust
// In App::view()
let [left, center, right] = self.modules_section(id);
let centerbox = Centerbox::new([left, center, right])
    .animated(animations_enabled)
    .spacing(space.xxs)
    .width(Length::Fill)
    .align_items(Alignment::Center)
    .height((HEIGHT + bar_layout.vertical_padding()) as f32)
    .padding(bar_layout.padding());
```

Where `modules_section()` returns `[left_modules, center_modules, right_modules]`.
