---
sidebar_position: 4
---

# Full Configuration Example

```toml
log_level = "warn"
#outputs = { Targets = ["eDP-1"] }
position = "Top"

[modules]
left = [ [ "appLauncher", "Updates", "Workspaces" ] ]
center = [ "WindowTitle" ]
right = [ "SystemInfo", "MediaPlayer", [ "Tray", "Tempo", "Privacy", "Settings" ] ]

[updates]
check_cmd = "checkupdates; paru -Qua"
update_cmd = 'alacritty -e bash -c "paru; echo Done - Press enter to exit; read" &'
interval = 3600

[workspaces]
visibility_mode = "All"
group_by_monitor = false
enable_workspace_filling = true

[[CustomModule]]
name = "appLauncher"
icon = "󱗼"
command = "walker"

[window_title]
mode = "Title"
truncate_title_after_length = 100

[system_info]
indicators = [ "Cpu", "Memory", "Temperature" ]
interval = 5

[system_info.cpu]
warn_threshold = 60
alert_threshold = 80

[system_info.memory]
warn_threshold = 70
alert_threshold = 85

[system_info.temperature]
warn_threshold = 60
alert_threshold = 80
sensor = "acpitz temp1"

[tempo]
clock_format = "%a %d %b %R"
# formats = [ "%a %d %b %R", "%Y-%m-%d %H:%M:%S", "%H:%M:%S" ]
# timezones = [ "UTC", "America/New_York", "Europe/London" ]
weather_location = { City = "Rome" }
# weather_location = { Coordinates = [40.7128, -74.0060] }
# weather_location = "Current"
weather_indicator = "IconAndTemperature"

[notifications]
format = "%H:%M"
show_timestamps = true
max_notifications = 10
show_bodies = true

[settings]
lock_cmd = "playerctl --all-players pause; nixGL hyprlock &"
audio_sinks_more_cmd = "pavucontrol -t 3"
audio_sources_more_cmd = "pavucontrol -t 4"
wifi_more_cmd = "nm-connection-editor"
vpn_more_cmd = "nm-connection-editor"
bluetooth_more_cmd = "blueberry"
battery_format = "IconAndPercentage"
peripheral_battery_format = "Icon"
audio_indicator_format = "Icon"
microphone_indicator_format = "Icon"
network_indicator_format = "Icon"
bluetooth_indicator_format = "Icon"
brightness_indicator_format = "Icon"
indicators = [ "IdleInhibitor", "PowerProfile", "Audio", "Microphone", "Bluetooth", "Network", "Vpn", "Battery", "Brightness" ]

[[settings.CustomButton]]
name = "Virtual Keyboard"
icon = "⌨️"
command = "/path/to/toggle-keyboard.sh"
status_command = "/path/to/check-keyboard-status.sh"
tooltip = "Toggle On-Screen Keyboard"

[osd]
enabled = true   # disabled by default
timeout = 1500

[appearance]
style = "Islands"

primary_color = "#7aa2f7"
success_color = "#9ece6a"
warning_color = "#e0af68"
danger_color = "#f7768e"
text_color = "#a9b1d6"
workspace_colors = [ "#7aa2f7", "#9ece6a" ]
special_workspace_colors = [ "#7aa2f7", "#9ece6a" ]

[appearance.background_color]
base = "#1a1b26"
weak = "#24273a"
strong = "#414868"

```
