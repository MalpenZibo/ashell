# MenuWrapper

`src/widgets/menu_wrapper.rs`

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

1. Renders a fullscreen backdrop (semi-transparent or transparent)
2. Positions the menu content horizontally aligned with the triggering button
3. Positions the menu vertically above or below the bar (depending on bar position)
4. Handles click events on the backdrop to close the menu

## Integration

The `App::menu_wrapper()` method creates the MenuWrapper for the currently open menu:

```rust
fn menu_wrapper(&self, output: &ShellInfo, menu_type: &MenuType, button_ui_ref: &ButtonUIRef)
    -> Element<Message>
{
    let content = match menu_type {
        MenuType::Settings => self.settings.menu_view(&self.theme),
        MenuType::Updates => self.updates.menu_view(&self.theme),
        // ...
    };

    MenuWrapper::new(content, button_ui_ref, bar_position, menu_size)
        .on_backdrop_press(Message::CloseMenu(output.id))
}
```

## Menu Sizes

The wrapper uses predefined size categories for menu width:

| Size | Width |
|------|-------|
| Small | 250px |
| Medium | 350px |
| Large | 450px |
| XLarge | 650px |
