# MenuWrapper

`src/components/menu_wrapper.rs`

## Purpose

A container widget that positions menu popup content relative to a triggering button, with a backdrop overlay that handles click-outside-to-close.

## How It Works

```
┌─────────────────────────────────────┐
│  Backdrop (transparent overlay)     │
│                                     │
│          ┌──────────────┐           │
│          │  Menu Content│           │
│          │  (positioned │           │
│          │   relative   │           │
│          │   to button) │           │
│          └──────────────┘           │
│                                     │
│  Click anywhere on backdrop = close │
└─────────────────────────────────────┘
```

The MenuWrapper:

1. Renders a fullscreen backdrop (optional color set with `backdrop()`)
2. Positions the menu content horizontally centered on the triggering button's x coordinate, clamped to stay 8px inside the screen edges
3. Positions the menu vertically at the top or bottom (`align_y()`, depending on bar position)
4. Emits the `on_click_outside()` message when the backdrop is clicked, to close the menu
5. Animates opening and closing (`open()`, `animated()`)

## Integration

The `App::menu_wrapper()` method (in `src/components/menu.rs`) wraps the content of the currently open menu; `App::view()` picks the content by `MenuType`:

```rust
// In App::view()
MenuType::Settings => self.menu_wrapper(
    id,
    self.settings
        .menu_view(id, use_theme(|t| t.bar_position))
        .map(Message::Settings),
    ui_ref,
),

// In components/menu.rs
pub fn menu_wrapper<'a>(
    &'a self,
    id: SurfaceId,
    content: Element<'a, app::Message>,
    button_ui_ref: ButtonUIRef,
) -> Element<'a, app::Message> {
    // ...
    components::MenuWrapper::new(button_ui_ref.position.x, menu_body)
        .padding(/* ... */)
        .align_y(match bar_position {
            Position::Top => Vertical::Top,
            Position::Bottom => Vertical::Bottom,
        })
        .backdrop(backdrop_color(menu_backdrop))
        .on_click_outside(app::Message::CloseMenu(id))
        .open(!self.outputs.menu_is_closing(id))
        .animated(use_theme(|t| t.animations_enabled))
        .into()
}
```

## Menu Sizes

Menu widths come from the `MenuSize` enum in `src/components/menu.rs`; each module sets its menu content width with it (e.g. `.width(MenuSize::Medium)`):

| Size | Width |
|------|-------|
| Small | 250px |
| Medium | 350px |
| Large | 450px |
| XLarge | 650px |
