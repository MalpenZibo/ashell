---
sidebar_position: 1
---

# 🚪 Main

This page contains the base configuration options for Ashell.

It allows you to configure things like the log level, the monitor(s) used to
render the status bar, and the bar's position.

All these configurations are defined in the root of the `toml` file.

## Logging

The `[logging]` section controls log level, destination, and file location.

By default, ashell logs at `warn` level to a file directly inside
`$XDG_RUNTIME_DIR`, for example `/run/user/1000/ashell_rCURRENT.log`. When
`$XDG_RUNTIME_DIR` is unset or is not a private per-user directory, the
fallback is `/tmp/ashell/`. Log files are rotated daily or when they reach
10 MB, and the last 7 files are kept.

See [log levels](https://docs.rs/env_logger/latest/env_logger/#enabling-logging)
for the full filter syntax.

### Options

- `level` — Log verbosity: `"error"`, `"warn"`, `"info"`, or `"debug"` (default: `"warn"`).
- `target` — Where to write logs: `"file"` (default), `"stdout"`, or `"stderr"`.
- `directory` — Custom log directory (only used when `target = "file"`).
  Supports `~` and environment variable expansion, and is created if missing.
  Defaults to `$XDG_RUNTIME_DIR`.

:::warning

Changing the `[logging]` section requires restarting ashell to take effect.
The log destination is set once at startup and cannot be changed via hot-reload.

:::

:::caution

On multi-user systems, avoid setting `directory` to a shared path like `/tmp/ashell`.
The default `$XDG_RUNTIME_DIR` is per-user and avoids permission conflicts when
multiple users run ashell on the same machine; ashell only falls back to
`/tmp/ashell` when `$XDG_RUNTIME_DIR` is unusable.

:::

### Examples

```toml
[logging]
level = "debug"
```

```toml
[logging]
level = "warn,ashell=info,ashell::services::network=debug"
```

```toml
[logging]
target = "stdout"
```

```toml
[logging]
target = "file"
directory = "~/.local/log/ashell"
```

The `level` option supports fine-grained control per Rust module, e.g.
`"warn,ashell::services::network=debug"`. To understand all possible module
names, check the [source code](https://github.com/MalpenZibo/ashell). The `src`
folder is the root of the `ashell` module, and every directory or file under it
declares a module or submodule. For example, `src/modules/media_player.rs` maps
to `ashell::modules::media_player`.

:::warning

Don't confuse Ashell features (called "modules") with Rust modules
(defined with `mod.rs` or in files). In this configuration, we're
referring to Rust modules.

:::

## Language & Region

Ashell supports localization through two independent root-level options.

- `language` controls the language used for translated UI strings.
- `region` controls locale-dependent formatting: the date/time format and the
  unit system (metric vs. imperial), which in turn affects things like the
  temperature unit and the default wind speed unit.

Both accept a BCP-47 / POSIX-style locale identifier (e.g. `"en-US"`, `"it-IT"`).
They are optional and fall back to your environment: `language` resolves from
`$LC_ALL`, then `$LC_MESSAGES`, then `$LANG`, and `region` resolves from
`$LC_ALL`, then `$LC_TIME`, then `$LANG`. The unit system additionally honors
`$LC_MEASUREMENT` when set. If nothing matches, ashell defaults to `en-US`.

Individual modules can opt out of the unit system: see
[`system_info.temperature.units`](./modules/system_info.md#temperature) and
[`tempo.wind_speed_unit`](./modules/tempo.md).

```toml
language = "en-US"   # UI language
region   = "it-IT"   # date format + unit system
```

## Outputs

You can configure which monitor(s) should display the status bar.

It can render on all monitors, only on the active one
(the focused monitor when Ashell starts), or on a list of specified monitors.

### Output Examples

Render the status bar on all monitors:

```toml
outputs = "All"
```

Render the status bar on the active monitor:

```toml
outputs = "Active"
```

Render the status bar on a specific list of monitors:

```toml
outputs = { Targets = ["DP-1", "eDP-1"] }
```

## Position & Layer

Configure the bar position and Wayland layer.

### Position Options

- `"Top"` - Bar at top of screen (default)
- `"Bottom"` - Bar at bottom of screen

### Layer Options

- `"Overlay"` - Above everything including fullscreen
- `"Top"` - Above everything excluding fullscreen
- `"Bottom"` - Above background, below windows (default)

### Examples

```toml
position = "Top"
layer = "Overlay"
```

```toml
position = "Bottom"
layer = "Bottom"
```

## Close menu with esc

You can enable the use of the `Esc` key to close the menu.

:::warning

With these features enabled, ashell will use the keyboard
in an exclusive way when a menu is open.

This means other applications will not be able to use
the keyboard when the menu is open.

:::

```toml
enable_esc_key = true
```

## Visibility Toggle

You can toggle the visibility of the status bar using the built-in IPC socket:

```bash
# Toggle ashell visibility
ashell msg toggle-visibility
```

This is the recommended approach for keybind-based toggling or scripting.

Alternatively, you can still use a `SIGUSR1` signal:

```bash
kill -SIGUSR1 $(pidof ashell)
```

## OSD (On-Screen Display)

Ashell can show a transient overlay when volume, microphone, brightness, airplane mode
or idle inhibitor changes via IPC commands. This is useful for binding compositor keys to ashell:

```bash
# Volume
ashell msg volume-up
ashell msg volume-down
ashell msg volume-toggle-mute

# Microphone
ashell msg microphone-up
ashell msg microphone-down
ashell msg microphone-toggle-mute

# Brightness
ashell msg brightness-up
ashell msg brightness-down

# Airplane mode
ashell msg toggle-airplane-mode

# Idle Inhibitor
ashell msg toggle-idle-inhibitor
```

The OSD appears at center-bottom and auto-hides after a timeout. To suppress
it for a specific command, add `--no-osd`:

```bash
ashell msg volume-up --no-osd
```

### OSD Configuration

```toml
[osd]
enabled = true   # Disabled by default; set to true to enable the OSD overlay
timeout = 1500   # Auto-hide delay in milliseconds
show_volume_percentage = true    # Show percentage text next to volume/mic bar
show_brightness_percentage = true # Show percentage text next to brightness bar
```

## Animations

Ashell ships a master toggle for UI animations (bar module width transitions,
menu open/close, toast slides, sub-menu accordion, toggle color fades, etc.).
It defaults to `false`, so ashell keeps its historical static behavior unless
you opt in:

```toml
[animations]
enabled = true
```
