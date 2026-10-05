# The App Struct

The `App` struct in `src/app.rs` is the central state container for the entire application. It owns all module instances, the general configuration, and the output/surface management.

## Fields

```rust
pub struct App {
    config_path: PathBuf,               // Path to the TOML config file
    logger: LoggerHandle,               // flexi_logger handle for runtime log level changes
    pub general_config: GeneralConfig,  // Extracted config subset (outputs, modules, layer)
    pub outputs: Outputs,               // Multi-monitor surface management

    // Module instances
    pub custom: HashMap<String, Custom>,     // User-defined custom modules
    pub updates: Option<Updates>,            // Package update checker (optional)
    pub workspaces: Workspaces,              // Workspace indicators
    pub window_title: WindowTitle,           // Active window display
    pub system_info: SystemInfo,             // CPU/RAM/disk/network stats
    pub keyboard_layout: KeyboardLayout,     // Keyboard layout indicator
    pub keyboard_submap: KeyboardSubmap,     // Hyprland submap display
    pub tray: TrayModule,                    // System tray
    pub tempo: Tempo,                        // Clock/calendar/weather
    pub privacy: Privacy,                    // Mic/camera/screenshare indicators
    pub settings: Settings,                  // Settings panel
    pub media_player: MediaPlayer,           // MPRIS media control
    pub notifications: Notifications,        // Notification center and toasts
    pub osd: Osd,                            // On-screen display overlay
}
```

The theme is not stored on `App`: it lives in a thread-local in `src/theme.rs`, set with `init_theme()` and read with `use_theme()`. Bar visibility (toggled via `SIGUSR1` or IPC) is tracked by `Outputs`, not by `App`.

## GeneralConfig

A subset of the config used at the App level:

```rust
pub struct GeneralConfig {
    outputs: config::Outputs,     // Which monitors to show the bar on
    pub modules: Modules,         // Left/center/right module layout
    pub layer: config::Layer,     // Wayland layer (Top/Bottom/Overlay)
    enable_esc_key: bool,         // Whether ESC closes menus
}
```

## Initialization

`App::new()` returns a closure that produces the initial state and a startup task:

```rust
pub fn new(
    (logger, config, config_path): (LoggerHandle, Config, PathBuf),
) -> impl FnOnce() -> (Self, Task<Message>) {
    move || {
        let mut outputs = Outputs::new(/* bar layout, position, layer, scale_factor */);

        // Initialize all modules from config
        let custom = config.custom_modules.clone().into_iter()
            .map(|o| (o.name.clone(), Custom::new(o)))
            .collect();

        init_theme(AshellTheme::new(config.position, &config.appearance, &config.animations));
        init_localizer(resolve_localizer(&config));

        // ...
        (App { /* all fields */ }, warm_icons)
    }
}
```

The startup task warms the XDG icon cache when the tray or icon-based workspace indicators are in use.

## Config Hot-Reload

When the config file changes, the `Message::ConfigChanged` handler in `App::update()` re-syncs outputs if needed, updates the logger level, and calls `App::refresh_config()` to propagate changes to all modules:

```rust
// In App::update(), Message::ConfigChanged(config):
if /* outputs, position, bar layout, scale factor or layer changed */ {
    tasks.push(self.outputs.sync(/* ... */)); // may create/destroy surfaces
}
self.logger.set_new_spec(get_log_spec(&config.logging.level));
tasks.push(self.refresh_config(config));

fn refresh_config(&mut self, config: Box<Config>) -> Task<Message> {
    // Update theme and localizer
    init_theme(AshellTheme::new(config.position, &config.appearance, &config.animations));
    init_localizer(resolve_localizer(&config));

    // Update general config
    self.general_config = GeneralConfig { /* ... */ };

    // Propagate to each module via ConfigReloaded messages
    self.workspaces.update(modules::workspaces::Message::ConfigReloaded(config.workspaces));
    self.settings.update(modules::settings::Message::ConfigReloaded(Box::new(config.settings)));
    // ... and so on for each module
}
```

This enables live editing of the config file without restarting ashell.
Note that only `logging.level` is re-applied on reload: `logging.target` and
`logging.directory` are read once at startup by `config::read_logging_config()`
and need a restart.
