# Menu System

The menu system is defined in `src/components/menu.rs`. It manages popup menus that appear when users click on modules in the bar.

## MenuType

Each module that supports a popup menu has a corresponding `MenuType`:

```rust
pub enum MenuType {
    Updates,
    Settings,
    Notifications,
    Tray(String),     // Tray menus are identified by app name
    MediaPlayer,
    SystemInfo,
    Tempo,
    // Tooltip menus (opened on hover, not click)
    AudioTooltip,
    BluetoothTooltip,
    WifiTooltip,
    VpnTooltip,
    BatteryTooltip,
    PeripheralBatteryTooltip(usize),
}
```

## Menu Struct

```rust
pub struct Menu {
    pub open: Option<OpenMenu>,          // Currently open menu, if any
    closing: bool,                       // Close animation in progress
    pending_open: Option<PendingOpen>,   // Menu queued to open after the close animation
    animations_enabled: bool,
}

pub struct OpenMenu {
    pub id: SurfaceId,                   // Layer surface ID of the menu
    pub menu_type: MenuType,
    pub button_ui_ref: ButtonUIRef,      // Position of the button that opened it
}
```

- When `open` is `None`, no menu is open and no menu surface exists.
- When `open` is `Some(...)`, a full-output layer surface on the Overlay layer exists, and the menu content is positioned relative to the button.

## Menu Lifecycle

### Open

```rust
pub fn open(&mut self, menu_type, button_ui_ref, request_keyboard, output_id) -> Task<app::Message> {
    // Create a new layer surface anchored to all edges, on the Overlay layer,
    // with OnDemand keyboard interactivity if requested (None otherwise)
    let (menu_id, task) = new_layer_surface(LayerShellSettings { /* ... */ });
    // Destroy any surface still alive before reusing the slot
    // ...
    self.open = Some(OpenMenu { id: menu_id, menu_type, button_ui_ref });
    Task::batch(vec![destroy, task])
}
```

### Close

Closing is split in two steps so the close animation can play:

```rust
/// Begin the close animation, firing `FinishCloseMenu` once it ends.
pub fn close(&mut self) -> Task<app::Message> {
    // Sets `closing`, then emits Message::FinishCloseMenu(id) after
    // ANIMATION_DURATION (immediately when animations are disabled)
}

/// Destroy the surface after the close animation, opening any queued menu.
pub fn finish_close(&mut self) -> Task<app::Message> {
    // Opens `pending_open` if set, otherwise destroy_layer_surface(open.id)
}
```

### Toggle

```rust
pub fn toggle(&mut self, menu_type, button_ui_ref, request_keyboard, output_id) -> Task<app::Message> {
    // While closing: same type cancels the close, a different type is queued
    match &mut self.open {
        None => self.open(menu_type, button_ui_ref, request_keyboard, output_id),
        Some(open) if open.menu_type == menu_type => {
            // Tooltips just update their position; other menus close
        }
        // A tooltip never replaces an open non-tooltip menu
        Some(open) => {
            // Switch to a different menu type without close/open cycle
            open.menu_type = menu_type;
            open.button_ui_ref = button_ui_ref;
            Task::none()
        }
    }
}
```

## Menu Positioning

Menus are positioned relative to the button that triggered them. The `ButtonUIRef` (in `src/components/position_button.rs`) carries the button's center point and the viewport size:

```rust
pub struct ButtonUIRef {
    pub position: Point,
    pub viewport: (f32, f32),
}
```

In `App::menu_wrapper()` (also in `src/components/menu.rs`), the menu content is wrapped in a `MenuWrapper` widget that:

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

Bar surfaces have keyboard interactivity set to `None` (Wayland doesn't need to track keyboard focus for the bar). A menu surface is created with `OnDemand` keyboard interactivity when `enable_esc_key` is set (so ESC can close it), and `None` otherwise. When a menu needs text input (e.g., WiFi password entry), `Menu::request_keyboard()` switches it to `Exclusive` so keystrokes reach the dialog immediately; `Menu::release_keyboard()` sets it back to `None`.
