# The Message Enum

The `Message` enum in `src/app.rs` is the central event type for the entire application. Every state change flows through it.

## All Variants

```rust
pub enum Message {
    // Config file changed (hot-reload)
    ConfigChanged(Box<Config>),

    // Menu management
    ToggleMenu(MenuType, Id, ButtonUIRef),   // Open/close a menu at a specific position
    CloseMenu(Id),                            // Close menu on a specific output
    CloseAllMenus,                            // Close menus on all outputs

    // Module-specific messages
    Custom(String, custom_module::Message),    // Custom module (keyed by name)
    Updates(modules::updates::Message),
    Workspaces(modules::workspaces::Message),
    WindowTitle(modules::window_title::Message),
    SystemInfo(modules::system_info::Message),
    KeyboardLayout(modules::keyboard_layout::Message),
    KeyboardSubmap(modules::keyboard_submap::Message),
    Tray(modules::tray::Message),
    Clock(modules::clock::Message),
    Tempo(modules::tempo::Message),
    Privacy(modules::privacy::Message),
    Settings(modules::settings::Message),
    MediaPlayer(modules::media_player::Message),

    // System events
    OutputEvent((OutputEvent, WlOutput)),      // Wayland monitor added/removed
    ResumeFromSleep,                           // System woke from sleep
    ToggleVisibility,                          // SIGUSR1 signal received
    None,                                      // No-op
}
```

## Routing Pattern

In `App::update()`, each message variant is matched and delegated to the appropriate handler:

```rust
fn update(&mut self, message: Message) -> Task<Message> {
    match message {
        Message::ConfigChanged(config) => {
            self.refesh_config(config);
            // ...
        }
        Message::Settings(msg) => {
            match self.settings.update(msg) {
                settings::Action::None => Task::none(),
                settings::Action::CloseMenu => { /* close menu */ }
                settings::Action::Command(task) => task,
                // ...
            }
        }
        Message::Workspaces(msg) => {
            self.workspaces.update(msg)
        }
        // ... one arm per variant
    }
}
```

## Special Messages

### ConfigChanged

Emitted by the config file watcher subscription when the TOML file is modified. Triggers a full config reload across all modules.

### OutputEvent

Emitted by Wayland when monitors are connected or disconnected. Triggers creation or destruction of layer surfaces.

### ToggleVisibility

Emitted when the process receives a `SIGUSR1` signal. Toggles the `visible` field, which controls whether the bar is shown or hidden.

### ResumeFromSleep

Emitted by the logind service when the system wakes from sleep. Used to refresh stale data (e.g., re-check network status, update clock).

### CloseAllMenus

Emitted when all menus should close (e.g., when the ESC key is pressed with `enable_esc_key = true`).
