---
sidebar_position: 4
---

# Full Configuration Example

```toml
log_level = "warn"
outputs = { Targets = ["eDP-1"] }
position = "Top"
app_launcher_cmd = "walker"

[modules]
left = [ [ "appLauncher", "Updates", "Workspaces" ] ]
center = [ "WindowTitle" ]
right = [ "SystemInfo", [  "Tray", "Clock", "Privacy", "Settings" ] ]

[updates]
check_cmd = "checkupdates; paru -Qua"
update_cmd = 'alacritty -e bash -c "paru; echo Done - Press enter to exit; read" &'

[workspaces]
enable_workspace_filling = true

[[CustomModule]]
name = "appLauncher"
icon = "󱗼"
command = "walker"

[window_title]
truncate_title_after_length = 100

[settings]
lock_cmd = "playerctl --all-players pause; nixGL hyprlock &"
audio_sinks_more_cmd = "pavucontrol -t 3"
audio_sources_more_cmd = "pavucontrol -t 4"
wifi_more_cmd = "nm-connection-editor"
vpn_more_cmd = "nm-connection-editor"
bluetooth_more_cmd = "blueberry"

[appearance]
style = "Islands"

primary_color = "#7aa2f7"
success_color = "#9ece6a"
text_color = "#a9b1d6"
workspace_colors = [ "#7aa2f7", "#9ece6a" ]
special_workspace_colors = [ "#7aa2f7", "#9ece6a" ]

[appearance.danger_color]
base = "#f7768e"
weak = "#e0af68"

[appearance.background_color]
base = "#1a1b26"
weak = "#24273a"
strong = "#414868"

[appearance.secondary_color]
base = "#0c0d14"

```
