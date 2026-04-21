# Configuration Reference

Complete reference for all configuration options in `~/.config/ashell/config.toml`.

## Top-Level Options

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `log_level` | String | `"warn"` | Log level ([env_logger syntax](https://docs.rs/env_logger)) |
| `position` | `"Top"` \| `"Bottom"` | `"Bottom"` | Bar position on screen |
| `layer` | `"Top"` \| `"Bottom"` \| `"Overlay"` | `"Bottom"` | Wayland layer (Bottom = below floating windows) |
| `outputs` | `"All"` \| `"Active"` \| `{ Targets = [...] }` | `"All"` | Which monitors show the bar |
| `enable_esc_key` | bool | `false` | Whether ESC key closes menus |

## Module Layout

```toml
[modules]
left = ["Workspaces"]
center = ["Tempo"]
right = [["SystemInfo", "Settings"], "Tray"]
```

Module names: `"Workspaces"`, `"WindowTitle"`, `"SystemInfo"`, `"KeyboardLayout"`, `"KeyboardSubmap"`, `"Tray"`, `"Clock"`, `"Tempo"`, `"Privacy"`, `"Settings"`, `"MediaPlayer"`, `"Updates"`, `"Custom:name"`.

## Appearance

```toml
[appearance]
style = "Islands"               # "Islands", "Solid", or "Gradient"
opacity = 0.9                   # 0.0-1.0
font_name = "JetBrains Mono"   # Optional custom font
scale_factor = 1.0              # DPI scale factor
```

### Colors

```toml
# Simple hex color
[appearance]
background = "#1e1e2e"

# Complete color with variants
[appearance.primary]
base = "#cba6f7"
strong = "#dbbcff"
weak = "#a385d8"
text = "#1e1e2e"
```

Available color fields: `background`, `text`, `primary`, `secondary`, `success`, `danger`.

### Menu Appearance

```toml
[appearance.menu]
opacity = 0.95
backdrop_blur = true
```

### Workspace Colors

```toml
[appearance]
workspace_colors = ["#cba6f7", "#f38ba8", "#a6e3a1", "#89b4fa"]
special_workspace_colors = ["#fab387"]
```

## Updates Module

```toml
[updates]
check_cmd = "checkupdates | wc -l"    # Command to check for updates
update_cmd = "foot -e sudo pacman -Syu" # Command to run updates
interval = 3600                         # Check interval in seconds
```

If the `[updates]` section is omitted entirely, the Updates module is disabled.

## Workspaces Module

```toml
[workspaces]
visibility_mode = "All"              # "All", "MonitorSpecific", "MonitorSpecificExclusive"
group_by_monitor = false
enable_workspace_filling = false     # Fill empty workspace slots
disable_special_workspaces = false
max_workspaces = 10                  # Optional: limit workspace count
workspace_names = ["1", "2", "3"]    # Optional: custom names
enable_virtual_desktops = false
```

## Window Title Module

```toml
[window_title]
mode = "Title"                       # "Title", "Class", "InitialTitle", "InitialClass"
truncate_title_after_length = 150
```

## Keyboard Layout Module

```toml
[keyboard_layout]
labels = { "English (US)" = "EN", "Italian" = "IT" }
```

## System Info Module

```toml
[system_info]
# CPU thresholds
[system_info.cpu]
warn_threshold = 60
alert_threshold = 80

# Memory thresholds
[system_info.memory]
warn_threshold = 60
alert_threshold = 80

# Temperature thresholds
[system_info.temperature]
warn_threshold = 60
alert_threshold = 80

# Disk thresholds
[system_info.disk]
warn_threshold = 60
alert_threshold = 80
```

**Dependencies:**
- Temperature monitoring reads the kernel `hwmon` sysfs interface directly (no extra package required). The `sensor` label (e.g. `"acpitz temp1"`) must match a hwmon device on your system — run `sensors` (from `lm_sensors`) to find the right name.
- CPU, memory, disk, and network info use standard kernel interfaces and do not need extra packages.

## Clock Module (Deprecated)

```toml
[clock]
format = "%H:%M"    # chrono format string
```

## Tempo Module

```toml
[tempo]
format = "%H:%M"
date_format = "%A, %B %d"
timezones = ["America/New_York", "Europe/London"]
weather_location = "Rome"              # Or coordinates
weather_format = "{temp}°C"
```

## Settings Module

```toml
[settings]
# Custom buttons in the settings panel
[[settings.custom_buttons]]
icon = "\u{f023}"
label = "VPN"
status_cmd = "vpn-status"
on_click = "vpn-toggle"
```

**Sub-module dependencies:** The Settings module requires `systemd-logind` for shutdown/reboot/sleep actions.

| Sub-module | Required Package |
|------------|-----------------|
| Audio (volume) | PulseAudio or PipeWire-Pulse |
| Bluetooth | `bluez` |
| Brightness | systemd-logind (usually present) |
| Network | `networkmanager` or `iwd` |
| Power (battery) | `upower` |

## Media Player Module

```toml
[media_player]
format = "{artist} - {title}"
```

**Dependencies:** Any MPRIS-compatible media player (e.g., Spotify, Firefox, VLC, Strawberry). No extra system package is needed.

## Custom Modules

```toml
[[CustomModule]]
name = "mymodule"
type = "Text"                   # "Text" or "Button"
cmd = "echo Hello"
interval = 5
format = "Result: {}"

[[CustomModule]]
name = "launcher"
type = "Button"
icon = "\u{f0e7}"
on_click = "rofi -show drun"
```

Custom module fields:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | String | Yes | Unique identifier |
| `type` | `"Text"` \| `"Button"` | No | Display mode |
| `cmd` | String | No | Command to execute for display text |
| `on_click` | String | No | Command on click (Button type) |
| `icon` | String | No | Icon character |
| `interval` | u64 | No | Refresh interval in seconds |
| `format` | String | No | Output format string |

Reference a custom module in the layout as `"Custom:name"`:

```toml
[modules]
right = ["Custom:mymodule", "Settings"]
```
