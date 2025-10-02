<div align="center">
  <a href="https://malpenzibo.github.io/ashell/">
    <img src="https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/logo_header_dark.svg" alt="ashell" height="140"/>
  </a>
</div>

## What is ashell?

ashell is a ready to go Wayland status bar for Hyprland.

Feel free to fork this project and customize it for your needs or just open an
issue to request a particular feature.

## 🚀 Getting Started

Refer to the [Getting Started](https://malpenzibo.github.io/ashell/docs/intro)
page on website

## ✨ Features

- App Launcher button
- Сlipboard button
- OS Updates indicator
- Hyprland Active Window
- Hyprland Workspaces
- System Information (CPU, RAM, Temperature)
- Hyprland Keyboard Layout
- Hyprland Keyboard Submap
- Tray
- Date time
- Privacy (check microphone, camera and screenshare usage)
- Media Player
- Settings panel
  - Power menu
  - Battery information
  - Audio sources and sinks
  - Screen brightness
  - Network stuff
  - VPN
  - Bluetooth
  - Power profiles
  - Idle inhibitor
  - Airplane mode
- Custom Modules
  - Simple (execute command on click)
  - Advanced (update UI with command output)

## 🛠️ Install

[![Packaging status](https://repology.org/badge/vertical-allrepos/ashell.svg)](https://repology.org/project/ashell/versions)

Refer to the [Installation](https://malpenzibo.github.io/ashell/docs/installation)
page for more details.

## ⚙️ Configuration

ashell comes with a default configuration that should work out of the box.

If you want to customize it you can refer to
the [Configuration](https://malpenzibo.github.io/ashell/docs/configuration)
page for more details.

```bash
ashell --config-path /path/to/config.toml
```

```toml
# Ashell log level filter, possible values "debug" | "info" | "warn" | "error". Needs reload
log_level = "warn"
# Possible status bar outputs, values could be: All, Active, or a list of outputs
# All: the status bar will be displayed on all the available outputs, example: outputs = "All"
# active: the status bar will be displayed on the active output, example: outputs = "Active"
# list of outputs: the status bar will be displayed on the outputs listed here, example: outputs = { Targets = ["DP-1", "eDP-1"] }
# if the outputs is not available the bar will be displayed in the active output
outputs = "All"
# Bar position, possible values Top | Bottom.
position = "Top"
# App launcher command, it will be used to open the launcher,
# without a value the related button will not appear
# optional, default None
app_launcher_cmd = "~/.config/rofi/launcher.sh"
# Clipboard command, it will be used to open the clipboard menu,
# without a value the related button will not appear
# optional, default None
clipboard_cmd = "cliphist-rofi-img | wl-copy"

# Declare which modules should be used and in which position in the status bar.
# This is the list of all possible modules
#  - AppLauncher
#  - Updates
#  - Clipboard
#  - Workspaces
#  - WindowTitle
#  - SystemInfo
#  - KeyboardLayout
#  - KeyboardSubmap
#  - Tray
#  - Clock
#  - Privacy
#  - MediaPlayer
#  - Settings
# optional, the following is the default configuration
[modules]
# The modules that will be displayed on the left side of the status bar
left = [ "Workspaces" ]
# The modules that will be displayed in the center of the status bar
center = [ "WindowTitle" ]
# The modules that will be displayed on the right side of the status bar
# The nested modules array will form a group sharing the same element in the status bar
# You can also use custom modules to extend the normal set of options, see configuration below
right = [ "SystemInfo", [ "Clock", "Privacy", "Settings" ], "CustomNotifications" ]

# Update module configuration.
# Without a value the related button will not appear.
# optional, default None
[updates]
# The check command will be used to retrieve the update list.
# It should return something like `package_name version_from -> version_to\n`
check_cmd = "checkupdates; paru -Qua"
# The update command is used to init the OS update process
update_cmd = 'alacritty -e bash -c "paru; echo Done - Press enter to exit; read" &'

# Workspaces module configuration, optional
[workspaces]
# The visibility mode of the workspaces, possible values are:
# All: all the workspaces will be displayed
# MonitorSpecific: only the workspaces of the related monitor will be displayed
# optional, default All
visibility_mode = "All"

# Enable filling with empty workspaces
# For example:
# With this flag set to true if there are only 2 workspaces,
# the workspace 1 and the workspace 4, the module will show also
# two more workspaces, the workspace 2 and the workspace 3
# optional, default false
enable_workspace_filling = false

# If you want to see more workspaces prefilled, set the number here:
# max_workspaces = 6
# In addition to the 4 workspaces described above it will also show workspaces 5 and 6
# Only works with `enable_workspace_filling = true`

# WindowTitle module configuration, optional
[window_title]
# The information to get from your active window.
# Possible modes are:
# - Title
# - Class
# optional, default Title
mode = "Title"

# Maximum number of chars that can be present in the window title
# after that the title will be truncated
# optional, default 150
truncate_title_after_length = 150


# keyboardLayout module configuration
# optional
# Maps layout names to arbitrary labels, which can be any text, including unicode symbols as shown below
# If using Hyprland the names can be found in `hyprctl devices | grep "active keymap"`
[keyboard_layout.labels]
"English (US)" = "🇺🇸"
"Russian" = "🇷🇺"

# The system module configuration
# optional
[system]
# System information shown in the status bar
# The possible values are:
#  - Cpu
#  - Memory
#  - MemorySwap
#  - Temperature
#  - { disk = "path" }
#  - IpAddress
#  - DownloadSpeed
#  - UploadSpeed
# optional, the following is the default configuration
# If for example you want to dispay the usage of the root and home partition
# you can use the following configuration
# systemInfo = [ { disk = "/" }, { disk = "/home" } ]
indicators = [ "Cpu", "Memory", "Temperature" ]

# CPU indicator thresholds
# optional
[system.cpu]
# cpu indicator warning level (default 60)
warn_threshold = 60
# cpu indicator alert level (default 80)
alert_threshold = 80

# Memory indicator thresholds
# optional
[system.memory]
# mem indicator warning level (default 70)
warn_threshold = 70
# mem indicator alert level (default 85)
alert_threshold = 85

# Memory swap indicator thresholds
# optional
[system.temperature]
# temperature indicator warning level (default 60)
warn_threshold = 60
# temperature indicator alert level (default 80)
alert_threshold = 80

# Disk indicator thresholds
# optional
[system.disk]
# disk indicator warning level (default 80)
warn_threshold = 80
# disk indicator alert level (default 90)
alert_threshold = 90

# Clock module configuration
[clock]
# clock format see: https://docs.rs/chrono/latest/chrono/format/strftime/index.html
format = "%a %d %b %R"

# Media player module configuration
[media_player]
# optional, default 100
max_title_length = 100

# Custom modules configuration (you can have multiple)
[[CustomModule]]
# The name will link the module in your left/center/right definition
name = "CustomNotifications"
# The default icon for this custom module
icon = ""
# The command that will be executed on click
command = "swaync-client -t -sw"
# You can optionally configure your custom module to update the UI using another command
# The output right now follows the waybar json-style output, using the `alt` and `text` field
# E.g. `{"text": "3", "alt": "notification"}`
listen_cmd = "swaync-client -swb"
# You can define behavior for the `text` and `alt` fields
# Any number of regex can be used to change the icon based on the alt field
icons.'dnd.*' = ""
# Another regex can optionally show a red "alert" dot on the icon
alert = ".*notification"

# Settings module configuration
[settings]
# command used for lock the system
# without a value the related button will not appear
# optional, default None
lock_cmd = "hyprlock &"
# commands used to respectively shutdown, suspend, reboot and logout
# all optional, without values the defaults shown here will be used
shutdown_cmd = "shutdown now"
suspend_cmd = "systemctl suspend"
reboot_cmd = "systemctl reboot"
logout_cmd = "loginctl kill-user $(whoami)"
# command used to open the sinks audio settings
# without a value the related button will not appear
# optional default None
audio_sinks_more_cmd = "pavucontrol -t 3"
# command used to open the sources audio settings
# without a value the related button will not appear
# optional, default None
audio_sources_more_cmd = "pavucontrol -t 4"
# command used to open the network settings
# without a value the related button will not appear
# optional, default None
wifi_more_cmd = "nm-connection-editor"
# command used to open the VPN settings
# without a value the related button will not appear
# optional, default None
vpn_more_cmd = "nm-connection-editor"
# command used to open the Bluetooth settings
# without a value the related button will not appear
# optional, default None
bluetooth_more_cmd = "blueman-manager"
# option to remove the airtplane button
# optional, default false
remove_airplane_btn = true

# Custom buttons in settings panel
[[settings.CustomButton]]
name = "Virtual Keyboard"
icon = "⌨️"
command = "/path/to/toggle-keyboard.sh"
status_command = "/path/to/check-keyboard-status.sh"
tooltip = "Toggle On-Screen Keyboard"

[[settings.CustomButton]]
name = "Terminal"
icon = ""
command = "alacritty"
tooltip = "Open Terminal"

# Appearance config
# Each color could be a simple hex color like #228800 or an
# object that define a base hex color and two optional variant of that color (a strong one and a weak one)
# and the text color that should be used with that base color
# example:
# [appearance.background_color]
# base = "#448877"
# strong = "#448888" # optional default autogenerated from base color
# weak = "#448855" # optional default autogenarated from base color
# text = "#ffffff" # optional default base text color
[appearance]
# optional, default iced.rs font
font_name = "Comic Sans MS"
# The style of the main bar, possible values are: Islands | Solid | Gradient
# optional, default Islands
style = "Islands"
# The opacity of the main bar, possible values are: 0.0 to 1.0
# optional, default 1.0
opacity = 0.7
# used as a base background color for header module button
background_color = "#1e1e2e"
# used as a accent color
primary_color = "#fab387"
# used for darker background color
secondary_color = "#11111b"
# used for success message or happy state
success_color = "#a6e3a1"
# used for danger message or danger state (the weak version is used for the warning state
danger_color = "#f38ba8"
# base default text color
text_color = "#f38ba8"
# this is a list of color that will be used in the workspace module (one color for each monitor)
workspace_colors = [ "#fab387", "#b4befe" ]
# this is a list of color that will be used in the workspace module
# for the special workspace (one color for each monitor)
# optional, default None
# without a value the workspaceColors list will be used
special_workspace_colors = [ "#a6e3a1", "#f38ba8" ]

# menu options
[appearance.menu]
# The opacity of the menu, possible values are: 0.0 to 1.0
# optional, default 1.0
opacity = 0.7
# The backdrop of the menu, possible values are: 0.0 to 1.0
# optional, default 0.0
backdrop = 0.3
```

## 📷 Screenshots

I will try my best to keep these screenshots as updated as possible but some details
could be different

#### default style

<img src="https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/ashell.png"></img>

#### solid style

<img src="https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/ashell-solid.png"></img>

#### gradient style

<img src="https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/ashell-gradient.png"></img>

#### opacity settings

<img src="https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/opacity.png"></img>

| ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/updates-panel.png) | ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/settings-panel.png)  |
| ---------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/power-menu.png)    | ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/sinks-selection.png) |
| ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/network-menu.png)  | ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/bluetooth-menu.png)  |
| ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/vpn-menu.png)      | ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/airplane-mode.png)   |
