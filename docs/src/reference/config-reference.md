# Configuration Reference

Complete reference for all configuration options in `~/.config/ashell/config.toml`.

## Top-Level Options

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | Option\<String\> | auto | UI language (BCP-47 or POSIX, e.g. `"en-US"`). Auto-detected from `$LC_MESSAGES` / `$LANG` when unset |
| `region` | Option\<String\> | auto | Regional formatting — dates and unit defaults (e.g. `"it-IT"`). Auto-detected from `$LC_TIME` / `$LANG` when unset |
| `position` | `"Top"` \| `"Bottom"` | `"Top"` | Bar position on screen |
| `layer` | `"Top"` \| `"Bottom"` \| `"Overlay"` | `"Bottom"` | Wayland layer (Bottom = below floating windows) |
| `outputs` | `"All"` \| `"Active"` \| `{ Targets = [...] }` | `"All"` | Which monitors show the bar |
| `enable_esc_key` | bool | `false` | Whether ESC key closes menus |
| `osd.enabled` | bool | `false` | Show OSD overlay for IPC volume/microphone/brightness/airplane/idle-inhibitor commands |
| `osd.timeout` | u64 | `1500` | OSD auto-hide delay in milliseconds |
| `osd.show_volume_percentage` | bool | `false` | Show the percentage in the volume/microphone OSD |
| `osd.show_brightness_percentage` | bool | `false` | Show the percentage in the brightness OSD |
| `animations.enabled` | bool | `false` | Master toggle for UI animations (bar widths, menu open/close, toast slides, etc.) |
| `logging.level` | String | `"warn"` | Log level ([env_logger syntax](https://docs.rs/env_logger)) |
| `logging.target` | `"file"` \| `"stdout"` \| `"stderr"` | `"file"` | Where logs are written |
| `logging.directory` | Option\<PathBuf\> | `$XDG_RUNTIME_DIR` (fallback `/tmp/ashell`) | Log directory, only used when `logging.target = "file"`. Supports `~` and `$VAR` |

## Module Layout

```toml
[modules]
left = ["Workspaces"]
center = ["Tempo"]
right = [["SystemInfo", "Settings"], "Tray"]
```

Module names: `"Workspaces"`, `"WindowTitle"`, `"SystemInfo"`, `"KeyboardLayout"`, `"KeyboardSubmap"`, `"Tray"`, `"Notifications"`, `"Tempo"`, `"Privacy"`, `"Settings"`, `"MediaPlayer"`, `"Updates"`. Any other string is the `name` of a custom module.

## Appearance

```toml
[appearance]
font_name = "JetBrains Mono"   # Optional custom font
font_weight = "normal"          # Optional face weight (thin ... black)
scale_factor = 1.0              # DPI scale factor
opacity = 1.0                   # 0.0-1.0, every surface ashell draws
blur = "auto"                   # auto|always|never, compositor blur

# `opacity` may instead be a table, replacing the scalar above. Any key may be
# omitted and falls back to `default`. Keep it after every `[appearance]`
# scalar, or those scalars become keys of this table.
# [appearance.opacity]
# default = 0.8
# bar = 1.0
# menu = 0.9
# osd = 0.6
# notifications = 0.9

[appearance.bar]
surface = "transparent"         # "transparent" or "solid"
radius = "none"                 # none|sm|md|lg|xl, CSS border-radius shorthand (solid only)
margin = "none"                 # none|xxs..xxl|number(pixels), CSS margin shorthand
                                # always screen pixels, not scaled by scale_factor
padding = "xxs"                 # (default) same values, inset inside the bar
                                # drawn inside the bar, so it does scale
```

### Colors

```toml
# Simple hex color
[appearance]
text_color = "#cdd6f4"

# Complete color with variants
[appearance.primary_color]
base = "#cba6f7"
strong = "#dbbcff"
weak = "#a385d8"
text = "#1e1e2e"
```

Available color fields: `background_color`, `primary_color`, `success_color`, `warning_color`, `danger_color`, `text_color`.

`background_color` in its complete form takes `base`, `weakest`, `weaker`, `weak`, `neutral`, `strong`, `stronger`, `strongest` and `text` instead of the `strong`/`weak`/`text` variants above.

### Menu Appearance

```toml
[appearance.menu]
backdrop = 0.3                  # darkening drawn behind an open menu
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
indicator_format = "Name"            # "Name" or "NameAndIcons"
group_by_monitor = false
enable_workspace_filling = false     # Fill empty workspace slots
disable_special_workspaces = false
max_workspaces = 10                  # Optional: limit workspace count
workspace_names = ["1", "2", "3"]    # Optional: custom names
enable_virtual_desktops = false
invert_scroll_direction = "All"      # Optional: "All", "Mouse" or "Trackpad"; omit to keep the normal direction
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
# "Cpu", "Memory", "MemorySwap", "Temperature", "IpAddress",
# "DownloadSpeed", "UploadSpeed", or { Disk = "/path", Name = "label" }
indicators = ["Cpu", "Memory", "Temperature"]
interval = 5                         # Refresh interval in seconds

# CPU thresholds
[system_info.cpu]
warn_threshold = 60
alert_threshold = 80
format = "Percentage"                # "Percentage" or "Frequency"

# Memory thresholds
[system_info.memory]
warn_threshold = 70
alert_threshold = 85
format = "Percentage"                # "Percentage", "Amount" or "Fraction"

# Temperature thresholds (default 60/80 °C, converted when units is Fahrenheit)
[system_info.temperature]
warn_threshold = 60
alert_threshold = 80
sensor = "Cpu"
# units = "Celsius"

# Disk thresholds
[system_info.disk]
warn_threshold = 80
alert_threshold = 90
format = "Percentage"                # "Percentage" or "Fraction"
# mounts = ["/", "/home"]
```

**Dependencies:**
- Temperature monitoring reads the kernel `hwmon` sysfs interface directly (no extra package required). The `sensor` option is either a type keyword (`"Cpu"`, `"Gpu"`, `"Acpi"`, `"Nvme"`) for auto-detection or an exact hwmon label (e.g. `"acpitz temp1"`) — run `sensors` (from `lm_sensors`) to find the right name. The displayed unit follows the locale / unit system unless `units` (`"Celsius"` / `"Fahrenheit"`) overrides it.
- CPU, memory, disk, and network info use standard kernel interfaces and do not need extra packages.

## Tempo Module

```toml
[tempo]
clock_format = "%a %d %b %R"           # chrono format string
formats = ["%R", "%a %d %b %R"]        # Optional: formats to cycle through, overrides clock_format
timezones = ["America/New_York", "Europe/London"]
weather_location = { City = "Rome" }   # "Current", { City = "..." } or { Coordinates = [lat, lon] }; omit to disable weather
weather_indicator = "IconAndTemperature"  # "IconAndTemperature", "Icon" or "None"
wind_speed_unit = "Kmh"                # Optional: "Kmh", "Mph" or "Ms"; defaults from the unit system
```

## Notifications Module

```toml
[notifications]
format = "%H:%M"                       # Timestamp format (chrono)
show_timestamps = true
show_bodies = true
grouped = false
toast = true                           # Show popup toasts
toast_position = "TopRight"            # "TopLeft", "TopRight", "TopCenter", "BottomLeft", "BottomRight", "BottomCenter"
toast_timeout = 5000                   # Milliseconds
toast_limit = 5
toast_max_height = 150
blocklist = ["^Spotify$"]              # Regexes matched against the app name
```

## Tray Module

```toml
[tray]
blocklist = ["^nm-applet$"]            # Regexes matched against the tray item name
right_click = "Menu"                   # Optional: "Open" or "Menu"
```

## Settings Module

```toml
[settings]
lock_cmd = "hyprlock"                  # Optional
shutdown_cmd = "shutdown now"
suspend_cmd = "systemctl suspend"
hibernate_cmd = "systemctl hibernate"  # Optional
reboot_cmd = "systemctl reboot"
logout_cmd = "loginctl kill-user $(whoami)"

# "IdleInhibitor", "PowerProfile", "Audio", "Microphone", "Network", "Vpn",
# "Bluetooth", "Battery", "PeripheralBattery", "Brightness"
indicators = ["IdleInhibitor", "PowerProfile", "Audio", "Microphone", "Bluetooth", "Network", "Vpn", "Battery"]

# Indicator formats: "Icon", "Percentage", "IconAndPercentage", "Time", "IconAndTime",
# "Name", "IconAndName", "PercentageAndTime", "IconAndPercentageAndTime"
battery_format = "IconAndPercentage"
battery_hide_when_full = false
peripheral_indicators = "All"          # or { Specific = [...] }
peripheral_battery_format = "Icon"
peripheral_expanded_by_default = false
battery_health = true                  # battery health and charge cycles
audio_indicator_format = "Icon"
microphone_indicator_format = "Icon"
network_indicator_format = "Icon"
bluetooth_indicator_format = "Icon"
brightness_indicator_format = "Icon"

volume_step = 5                        # 1-50
max_volume = 100                       # 1-200

# Optional commands for the "more" buttons and after switching audio device
# audio_sinks_more_cmd, audio_sources_more_cmd, audio_sink_post_switch_cmd,
# audio_source_post_switch_cmd, wifi_more_cmd, vpn_more_cmd, bluetooth_more_cmd

remove_airplane_btn = false
remove_idle_btn = false

# Enable/disable hover tooltips on status indicators (audio, bluetooth, wifi, battery)
enable_tooltips = true

# Show a slider for the built-in keyboard backlight, under the brightness slider
keyboard_backlight_slider = false

# Custom buttons in the settings panel
[[settings.CustomButton]]
name = "VPN"
icon = "\u{f023}"
command = "vpn-toggle"
status_command = "vpn-status"          # Optional
tooltip = "Toggle VPN"                 # Optional
```

**Sub-module dependencies:** The Settings module requires `systemd-logind` for shutdown/reboot/sleep actions.

| Sub-module | Required Package |
|------------|-----------------|
| Audio (volume) | PulseAudio or PipeWire-Pulse |
| Bluetooth | `bluez` |
| Brightness | systemd-logind (usually present) |
| Network | `networkmanager` or `iwd` |
| Power (battery) | `upower` |
| Keyboard backlight | `upower` (built-in keyboard only) |

## Media Player Module

```toml
[media_player]
indicator_format = "IconAndText"     # "Text", "IconAndText", or "Icon"
indicator_fields = ["Artist", "Title"]
max_text_length = 100
indicator_visualizer = "Background"  # "Background", "Before", or "After"; omit to disable
menu_visualizer = false              # bars behind the menu cards; cava runs only while the menu is open
visualizer_framerate = 30            # cava frames per second, clamped to 1-144

[media_player.indicator_controls]
left = "Menu"                        # "Menu", "Prev", "PlayPause", "Next", or "None"
middle = "None"
right = "None"
scroll = "None"                      # "Volume" or "None"
```

**Dependencies:** Any MPRIS-compatible media player (e.g., Spotify, Firefox, VLC, Strawberry). No extra system package is needed. The visualizer additionally needs `cava` on `$PATH`.

## Custom Modules

Custom modules allow you to create arbitrary modules with custom commands and icons.

```toml
[[CustomModule]]
name = "volume"
type = "Button"                        # "Text" or "Button"
icon = "\u{f026}"                      # Nerd Font icon
command = "pactl get-sink-volume @DEFAULT_SINK@"  # Left-click command
on_right_click = "pactl set-sink-mute @DEFAULT_SINK@ toggle"
on_middle_click = "pavucontrol"
on_scroll_up = "pactl set-sink-volume @DEFAULT_SINK@ +5%"
on_scroll_down = "pactl set-sink-volume @DEFAULT_SINK@ -5%"
listen_cmd = "pactl subscribe"         # Optional: stream JSON updates
```

Custom module fields:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | String | Yes | Unique identifier |
| `type` | `"Text"` \| `"Button"` | No | Display mode (default: `"Button"`) |
| `icon` | String | No | Nerd Font icon character |
| `command` | String | No | Command to execute on left-click (Button type) |
| `on_right_click` | String | No | Command on right-click |
| `on_middle_click` | String | No | Command on middle-click |
| `on_scroll_up` | String | No | Command on scroll up |
| `on_scroll_down` | String | No | Command on scroll down |
| `listen_cmd` | String | No | Command that outputs JSON lines for dynamic updates |
| `icons` | Map | No | Regex → icon mapping for dynamic icons |
| `alert` | String (regex) | No | Regex to show alert indicator |

The `listen_cmd` output must be JSON lines with `text` and `alt` fields, and
an optional `tooltip` (plain text, `\n` for new lines) shown on hover:
```json
{"text": "50%", "alt": "volume", "tooltip": "Speakers: 50%"}
```

Reference a custom module in the layout by its `name`:

```toml
[modules]
right = ["volume", "Settings"]
```

A name that matches no built-in module and no `[[CustomModule]]` is skipped
without any error, so a typo (or a `"Custom:"` prefix) makes the module silently
disappear.
