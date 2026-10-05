# The Message Enum

The `Message` enum in `src/app/message.rs` (re-exported from `src/app.rs`) is the central event type for the entire application. Every state change flows through it.

## All Variants

```rust
pub enum Message {
    // Config file changed (hot-reload)
    ConfigChanged(Box<Config>),

    // Menu management
    ToggleMenu(MenuType, SurfaceId, ButtonUIRef), // Open/close a menu at a specific position
    CloseMenu(SurfaceId),                         // Start closing the menu on a specific output
    FinishCloseMenu(SurfaceId),                   // Finish closing (after the close animation)

    // Module-specific messages
    Custom(String, custom_module::Message),    // Custom module (keyed by name)
    Updates(modules::updates::Message),
    Workspaces(modules::workspaces::Message),
    WindowTitle(modules::window_title::Message),
    SystemInfo(modules::system_info::Message),
    KeyboardLayout(modules::keyboard_layout::Message),
    KeyboardSubmap(modules::keyboard_submap::Message),
    Tray(modules::tray::Message),
    Tempo(modules::tempo::Message),
    Privacy(modules::privacy::Message),
    Settings(modules::settings::Message),
    MediaPlayer(modules::media_player::Message),
    Notifications(modules::notifications::Message),
    Osd(osd::Message),

    // IPC volume/brightness/toggle commands (with optional OSD)
    IpcOsdCommand(IpcCommand),

    // System events
    OutputEvent(OutputEvent),                  // Wayland monitor added/removed
    CloseAllMenus,                             // Close menus on all outputs
    ResumeFromSleep,                           // System woke from sleep
    None,                                      // No-op
    ToggleVisibility,                          // SIGUSR1 signal or IPC toggle-visibility
}
```

## Routing Pattern

In `App::update()`, each message variant is matched and delegated to the appropriate handler:

```rust
pub fn update(&mut self, message: Message) -> Task<Message> {
    match message {
        Message::ConfigChanged(config) => {
            // ...
            tasks.push(self.refresh_config(config));
            Task::batch(tasks)
        }
        Message::Settings(message) => match self.settings.update(message) {
            modules::settings::Action::None => Task::none(),
            modules::settings::Action::Command(task) => task.map(Message::Settings),
            modules::settings::Action::CloseMenu(id) => { /* close menu */ }
            // ...
        },
        Message::Workspaces(msg) => self.workspaces.update(msg).map(Message::Workspaces),
        // ... one arm per variant
    }
}
```

## Special Messages

### ConfigChanged

Emitted by the config file watcher subscription when the TOML file is modified. Triggers a full config reload across all modules.

### OutputEvent

Emitted by iced's `output_events()` subscription when monitors are connected or disconnected (and when surfaces enter/leave outputs). Triggers creation or destruction of layer surfaces.

### ToggleVisibility

Emitted when the process receives a `SIGUSR1` signal or the `toggle-visibility` IPC command. Calls `Outputs::toggle_visibility()`, which shows or hides the bar (and closes any open menu when hiding).

### ResumeFromSleep

Emitted by the logind service when the system wakes from sleep. Re-syncs the outputs (layer surfaces) via `Outputs::sync()`.

### CloseAllMenus

Emitted when all menus should close (e.g., when the ESC key is pressed with `enable_esc_key = true`).
