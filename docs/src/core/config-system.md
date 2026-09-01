# Configuration System

The configuration system is defined in `src/config.rs`. ashell uses a TOML file for all user-facing settings.

## Config File Location

Default path: `~/.config/ashell/config.toml`

Override with the `--config-path` CLI flag:

```bash
ashell --config-path /path/to/config.toml
```

## The Config Struct

```rust
pub struct Config {
    pub logging: LoggingConfig,                     // Level, target and directory
    pub position: Position,                         // Top or Bottom
    pub layer: Layer,                               // Top, Bottom, or Overlay
    pub outputs: Outputs,                           // All, Active, or Targets
    pub modules: Modules,                           // Left/center/right module layout
    pub custom_modules: Vec<CustomModuleDef>,        // [[CustomModule]] array
    pub updates: Option<UpdatesModuleConfig>,
    pub workspaces: WorkspacesModuleConfig,
    pub window_title: WindowTitleConfig,
    pub system_info: SystemInfoModuleConfig,
    pub clock: ClockModuleConfig,
    pub tempo: TempoModuleConfig,
    pub settings: SettingsModuleConfig,
    pub appearance: Appearance,
    pub media_player: MediaPlayerModuleConfig,
    pub keyboard_layout: KeyboardLayoutModuleConfig,
    pub animations: AnimationsConfig,               // Master toggle for UI animations
    pub enable_esc_key: bool,                       // Default: false
}
```

Every field has a serde `#[serde(default)]` attribute, so **an empty config file is valid** — the bar works with zero configuration.

## Logging Bootstrap

The logger must exist before `get_config()` runs, but its destination lives in
the config file. `config::read_logging_config()` solves this by parsing **only**
the `[logging]` table into a throwaway wrapper struct, before the logger is
initialized:

```rust
pub fn read_logging_config(path: Option<&PathBuf>) -> LoggingConfig
```

Because it runs before the logger exists it must not use `log::*`; problems are
reported on `stderr` and it falls back to `LoggingConfig::default()`. Unknown
top-level keys are skipped without being deserialized, so a broken `[modules]`
section does not prevent the `[logging]` table from being read. Typos are still
reported a moment later by the full `Config` parse.

Only `logging.level` is re-applied on hot-reload; `logging.target` and
`logging.directory` are fixed at startup.

## Module Layout

Modules are arranged in three sections:

```toml
[modules]
left = ["Workspaces"]
center = ["Tempo"]
right = [["SystemInfo", "Settings"]]  # Inner array = grouped modules
```

The `ModuleDef` enum handles both individual modules and groups:

```rust
pub enum ModuleDef {
    Single(ModuleName),         // A single module
    Group(Vec<ModuleName>),     // Multiple modules grouped in one "island"
}
```

Groups are rendered together in a single container, which is especially visible with the `transparent` (islands) bar surface.

## Hot-Reload

Config changes are detected via [inotify](https://docs.rs/inotify) file watching:

1. The `config::subscription()` function watches the config file's **parent directory** for `CREATE`, `MODIFY`, `DELETE`, and `MOVE` events.
2. Events are batched using `ready_chunks(10)` to handle editors that perform atomic saves (write to temp file, then rename).
3. `DELETE` events include a 500ms delay before re-reading, to handle atomic save patterns where the file is briefly absent.
4. On change, the new config is parsed and sent as `Message::ConfigChanged(Box<Config>)`.

The subscription uses `TypeId::of::<Config>()` as its ID to ensure only one watcher runs.

## Module-Specific Configs

Each module has its own config struct. Examples:

```toml
# Workspace visibility mode
[workspaces]
visibility_mode = "MonitorSpecific"
enable_workspace_filling = true

# Tempo clock format
[tempo]
format = "%H:%M"
date_format = "%A, %B %d"

# System info thresholds
[system_info.cpu]
warn_threshold = 60
alert_threshold = 80

# Updates checker
[updates]
check_cmd = "checkupdates | wc -l"
update_cmd = "foot -e sudo pacman -Syu"
interval = 3600
```

## Appearance Config

```toml
[appearance]
font_name = "JetBrains Mono"
scale_factor = 1.0

[appearance.bar]
surface = "transparent"    # transparent or solid
radius = "md"              # none|sm|md|lg|xl, CSS border-radius shorthand
margin = "sm"              # none|xxs..xxl, CSS margin shorthand

[appearance.opacity]
default = 0.9
bar = 1.0

[appearance.background]
base = "#1e1e2e"

[appearance.menu]
backdrop = 0.3
```

See the [Configuration Reference](../reference/config-reference.md) for a complete list of all configuration options.
