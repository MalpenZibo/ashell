---
sidebar_position: 4
---

# Full Configuration Example

This document shows every available configuration option with its default value.
Commented-out lines show the default; uncomment and change to customize.

```toml
log_level = "warn"
# language = "en-US"        # UI language; defaults to $LC_MESSAGES / $LANG
# region   = "it-IT"        # date format + unit system; defaults to $LC_TIME / $LANG
# outputs = "All"            # "All" (default), "Active", or { Targets = ["eDP-1"] }
position = "Top"             # "Top" (default) or "Bottom"
# layer = "Bottom"           # "Bottom" (default), "Top", or "Overlay"
# enable_esc_key = false     # Pressing Escape closes open menus

[modules]
left = [ [ "appLauncher", "Updates", "Workspaces" ] ]
center = [ "WindowTitle" ]
right = [ "SystemInfo", "MediaPlayer", [ "Tray", "Tempo", "Privacy", "Settings" ] ]

# Default module layout:
# left = [ "Workspaces" ]
# center = [ "WindowTitle" ]
# right = [ [ "Tempo", "Privacy", "Settings" ] ]

[updates]
check_cmd = "checkupdates; paru -Qua"
update_cmd = 'alacritty -e bash -c "paru; echo Done - Press enter to exit; read" &'
interval = 3600             # seconds; minimum enforced to 1

[workspaces]
visibility_mode = "All"           # "All" (default), "MonitorSpecific", "MonitorSpecificExclusive"
indicator_format = "Name"         # "Name" (default) or "NameAndIcons" to show app icons
group_by_monitor = false          # (default)
enable_workspace_filling = false  # (default)
# disable_special_workspaces = false  # (default) set true to hide special workspaces
# max_workspaces = 10               # (default: None) max number of workspaces when filling
# workspace_names = ["1", "2", "3"] # (default: []) custom names for workspaces
# enable_virtual_desktops = false   # (default) group workspaces into virtual desktops
# invert_scroll_direction = "All"   # (default: None) "All", "Mouse", or "Trackpad"

[[CustomModule]]
name = "appLauncher"
icon = "󱗼"
command = "walker"
# listen_cmd = "some-command"  # yields JSON lines: {"text": "...", "alt": "..."}
# icons = { "regex" = "icon" } # map regex on `alt` to icon
# alert = "regex"              # show alert dot when `alt` matches regex
# type = "Button"              # (default) "Button" or "Text"

[window_title]
mode = "Title"                   # "Title" (default), "Class", "InitialTitle", "InitialClass"
truncate_title_after_length = 150 # (default) 0 means no truncation; capped at 2048

[keyboard_layout]
# labels = { "English (US)" = "EN", "Italian" = "IT" }  # (default: {}) map layout names to short labels

[system_info]
indicators = [ "Cpu", "Memory", "Temperature" ]
# indicators = [ "Cpu", "Memory", "MemorySwap", "Temperature", "IpAddress", "DownloadSpeed", "UploadSpeed" ]
# indicators = [ { Disk = { Disk = "/dev/sda1", Name = "Root" } } ]
interval = 5

[system_info.cpu]
warn_threshold = 60
alert_threshold = 80
# format = "Percentage"   # (default) or "Frequency"

[system_info.memory]
warn_threshold = 70
alert_threshold = 85
# format = "Percentage"   # (default) or "Fraction"

[system_info.temperature]
# warn_threshold = 60     # (default: None, auto 60°C / 140°F based on unit system)
# alert_threshold = 80    # (default: None, auto 80°C / 176°F based on unit system)
# sensor = "Cpu"        # (default) type keyword: "Cpu", "Gpu", "Acpi", "Nvme" or exact label like "acpitz temp1"

[system_info.disk]
# warn_threshold = 80     # (default)
# alert_threshold = 90    # (default)
# format = "Percentage"   # (default) or "Fraction"
# mounts = ["/", "/home"] # (default: None = all non-removable disks)

[media_player]
# indicator_format = "IconAndText"  # (default), "Text", or "Icon"
# indicator_fields = ["Artist", "Title"] # (default), also supports "Album"
# max_text_length = 100         # (default)
# indicator_visualizer = "Background" # (default: None = disabled), "Before", or "After"
# menu_visualizer = false       # (default) bars behind the menu cards

[tray]
# blocklist = ["regex"]    # (default: []) hide tray items matching regex patterns
# right_click = "Menu"     # (default: None) "Open" or "Menu"; sets right-click action, left click gets the complement

[tempo]
clock_format = "%a %d %b %R"
# formats = [ "%a %d %b %R", "%Y-%m-%d %H:%M:%S", "%H:%M:%S" ]
# timezones = [ "UTC", "America/New_York", "Europe/London" ]
# weather_location = { City = "Rome" }
# weather_location = { Coordinates = [40.7128, -74.0060] }
# weather_location = "Current"
weather_indicator = "IconAndTemperature"  # (default), "Icon", or "None"
# wind_speed_unit = "Kmh"   # (default: None = derive from locale) "Kmh", "Mph", or "Ms"

[notifications]
format = "%H:%M"
show_timestamps = true
show_bodies = true
# grouped = false           # (default) group notifications by app
# toast = true              # (default) enable toast popups
# toast_position = "TopRight"  # (default) "TopLeft", "TopRight", "BottomLeft", "BottomRight"
# toast_timeout = 5000      # (default) milliseconds before auto-dismiss
# toast_limit = 5           # (default) max concurrent toasts
# toast_max_height = 150    # (default) max height of toast cards in pixels
# blocklist = ["regex"]     # (default: []) suppress notifications from apps matching regex

[settings]
# Optional: disable hover tooltips on status indicators
# enable_tooltips = false
lock_cmd = "playerctl --all-players pause; nixGL hyprlock &"
# shutdown_cmd = "shutdown now"                   # (default)
# suspend_cmd = "systemctl suspend"               # (default)
# hibernate_cmd = "systemctl hibernate"           # (default: None)
# reboot_cmd = "systemctl reboot"                 # (default)
# logout_cmd = "loginctl kill-user $(whoami)"     # (default)
audio_sinks_more_cmd = "pavucontrol -t 3"
audio_sources_more_cmd = "pavucontrol -t 4"
wifi_more_cmd = "nm-connection-editor"
vpn_more_cmd = "nm-connection-editor"
bluetooth_more_cmd = "blueberry"
battery_format = "IconAndPercentage"  # (default), "Icon", "Percentage", "Time", "IconAndTime"
# battery_hide_when_full = false  # (default)
# peripheral_indicators = "All"   # (default) or { Specific = ["Keyboard", "Mouse", "Headphones", "Gamepad"] }
peripheral_battery_format = "Icon"  # (default), "IconAndPercentage", "Percentage", etc.
# peripheral_expanded_by_default = false  # (default)
audio_indicator_format = "Icon"        # (default), "IconAndPercentage", "Percentage", etc.
microphone_indicator_format = "Icon"   # (default)
network_indicator_format = "Icon"      # (default), "IconAndPercentage", "Percentage", "Name", "IconAndName" (Name/IconAndName show the SSID/interface/VPN name)
bluetooth_indicator_format = "Icon"    # (default)
brightness_indicator_format = "Icon"   # (default)
volume_step = 5    # (default) step size for IPC volume up/down, range 1..=50
max_volume = 100   # (default) max volume level, range 1..=200 (>100 enables overdrive)
# remove_airplane_btn = false   # (default) set true to hide airplane mode button
# remove_idle_btn = false       # (default) set true to hide idle inhibitor button
indicators = [ "IdleInhibitor", "PowerProfile", "Audio", "Microphone", "Bluetooth", "Network", "Vpn", "Battery", "Brightness" ]
# indicators = [ "IdleInhibitor", "PowerProfile", "Audio", "Microphone", "Bluetooth", "Network", "Vpn", "Battery", "PeripheralBattery", "Brightness" ]

[[settings.CustomButton]]
name = "Virtual Keyboard"
icon = "⌨️"
command = "/path/to/toggle-keyboard.sh"
status_command = "/path/to/check-keyboard-status.sh"
tooltip = "Toggle On-Screen Keyboard"

[osd]
enabled = false   # (default)
timeout = 1500    # milliseconds
show_volume_percentage = false      # (default) show numeric volume value in the OSD
show_brightness_percentage = false  # (default) show numeric brightness value in the OSD

[animations]
enabled = false   # (default)

[appearance]
# font_name = "Sans"           # (default: None) custom font family
# scale_factor = 1.0           # (default) range: 0.0 < x <= 2.0
primary_color = "#7aa2f7"
success_color = "#9ece6a"
warning_color = "#e0af68"
danger_color = "#f7768e"
text_color = "#a9b1d6"
workspace_colors = [ "#7aa2f7", "#9ece6a" ]
# special_workspace_colors = [ "#7aa2f7", "#9ece6a" ]  # (default: None, falls back to workspace_colors)

[appearance.bar]
surface = "transparent"  # (default) or "solid"
# radius = "none"          # (default) none|sm|md|lg|xl, CSS border-radius shorthand (solid only)
# margin = "none"          # (default) none|xxs|xs|sm|md|lg|xl|xxl, CSS margin shorthand
# opacity = 1.0            # (default) range: 0.0 to 1.0

[appearance.menu]
# opacity = 1.0   # (default) menu background opacity
# backdrop = 0.0   # (default) backdrop blur amount

[appearance.background_color]
base = "#1a1b26"
weak = "#24273a"
strong = "#414868"
# weakest = "#000000"    # (default: None)
# weaker = "#000000"     # (default: None)
# neutral = "#000000"    # (default: None)
# stronger = "#000000"   # (default: None)
# strongest = "#000000"  # (default: None)
# text = "#ffffff"       # (default: None)
```
