# Menu System

The menu system is defined in `src/menu.rs`. It manages popup menus that appear when users click on modules in the bar.

## MenuType

Each module that supports a popup menu has a corresponding `MenuType`:

```rust
pub enum MenuType {
    Updates,
    Settings,
    Tray(String),     // Tray menus are identified by app name
    MediaPlayer,
    SystemInfo,
    Tempo,
}
```

## Menu Struct

```rust
pub struct Menu {
    pub id: Id,                                       // Layer surface ID
    pub menu_info: Option<(MenuType, ButtonUIRef)>,   // Currently open menu + button position
}
```

- When `menu_info` is `None`, no menu is open and the surface is on the Background layer.
- When `menu_info` is `Some(...)`, the menu is open, positioned relative to the button, and the surface is on the Overlay layer.

## Menu Lifecycle

### Open

```rust
pub fn open(&mut self, menu_type, button_ui_ref, request_keyboard) -> Task<Message> {
    self.menu_info.replace((menu_type, button_ui_ref));
    Task::batch(vec![
        set_layer(self.id, Layer::Overlay),               // Make visible
        // Optionally enable keyboard for text input (e.g., WiFi password)
    ])
}
```

### Close

```rust
pub fn close(&mut self) -> Task<Message> {
    self.menu_info.take();
    Task::batch(vec![
        set_layer(self.id, Layer::Background),            // Hide
        set_keyboard_interactivity(self.id, None),        // Disable keyboard
    ])
}
```

### Toggle

```rust
pub fn toggle(&mut self, menu_type, button_ui_ref, request_keyboard) -> Task<Message> {
    match self.menu_info.as_mut() {
        None => self.open(menu_type, button_ui_ref, request_keyboard),
        Some((current, _)) if *current == menu_type => self.close(),
        Some((current, ref_)) => {
            // Switch to a different menu type without close/open cycle
            *current = menu_type;
            *ref_ = button_ui_ref;
            Task::none()
        }
    }
}
```

## Menu Positioning

Menus are positioned relative to the button that triggered them. The `ButtonUIRef` carries the button's screen position and size:

```rust
pub struct ButtonUIRef {
    pub position: Point,
    pub size: Size,
}
```

In `App::menu_wrapper()`, the menu content is wrapped in a `MenuWrapper` widget that:

1. Positions the content relative to the button (aligned to the button's horizontal center).
2. Renders a backdrop overlay behind the menu.
3. Handles click-outside-to-close.

## Menu Sizes

Menus use predefined width categories:

```rust
pub enum MenuSize {
    Small,   // 250px
    Medium,  // 350px
    Large,   // 450px
    XLarge,  // 650px
}
```

## Keyboard Interactivity

By default, layer surfaces have keyboard interactivity set to `None` (performance optimization — Wayland doesn't need to track keyboard focus for the bar). When a menu needs text input (e.g., WiFi password entry), keyboard interactivity is set to `OnDemand`.
