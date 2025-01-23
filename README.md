# Ashell

A ready to go Wayland status bar for Hyprland.

Feel free to fork this project and customize it for your needs or just open an
issue to request a particular feature.

> If you have graphical issues like missing transparency or graphical artifact you could launch ashell with WGPU_BACKEND=gl. This env var forces wgpu to use OpenGL instead of Vulkan

### Does it only work on Hyprland?

While it's currently tailored for Hyprland, it could work with other compositors.

However, it currently relies on [hyprland-rs](https://github.com/hyprland-community/hyprland-rs)
to gather information about the active window and workspaces. I haven't implemented any
feature flags to disable these functionalities or alternative methods to obtain this data.

## Install

[![Packaging status](https://repology.org/badge/vertical-allrepos/ashell.svg)](https://repology.org/project/ashell/versions)

### Arch Linux

You can get the official Arch Linux package from the AUR:

#### Tagged release

```
paru/yay -S ashell
```

#### Main branch

```
paru/yay -S ashell-git
```

### ALT Linux

```
su -
apt-get install ashell
```

### Nix

To install ashell using the nix package be sure to enable flakes and then run

#### Tagged release

```
nix profile install github:MalpenZibo/ashell?ref=0.3.1
```

#### Main branch

```
nix profile install github:MalpenZibo/ashell
```

### NixOS

I haven't tested ashell on NixOS.

To enable this flake use

```nix
{ pkgs, ... }:

{
  environment.systemPackages = with pkgs; [
    (import (pkgs.callPackage (pkgs.fetchFromGitHub {
      owner = "MalpenZibo";
      repo = "ashell";
      rev = "refs/heads/main"; # Or specify the branch/tag you need
      sha256 = "sha256-PLACEHOLDER"; # Replace with the correct hash
    }) {}).defaultPackage.x86_64-linux)
  ];
}
```

> I'm not an expert and I haven't tested this configuration
> but I'm quite sure that if you use NixOS you are smart enough to add ashell to your configuration :D

## Features

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

## Configuration

The configuration uses the yaml file format and is named `~/.config/ashell.yml`

```yaml
# Ashell log level filter, possible values "DEBUG" | "INFO" | "WARNING" | "ERROR". Needs reload
logLevel: "WARN" # optional, default "WARN"
# Possible status bar outputs, values could be: All, Active, or Targets
# All: the status bar will be displayed on all the available outputs, example: outputs: All
# Active: the status bar will be displayed on the active output, example: outputs: Active
# Targets: the status bar will be displayed on the outputs listed here,
#   example:
#     outputs: !Targets
#       - DP-1
#       - eDP-1
# if the outputs is not available the bar will be displayed in the active output
outputs: All # optional, default all
# Bar position, possible values Top | Bottom.
position: Top # optional, default Top
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
#  - Settings
# optional, the following is the default configuration
modules:
  # The modules that will be displayed on the left side of the status bar
  left:
    - Workspaces
  # The modules that will be displayed in the center of the status bar
  center:
    - WindowTitle
  # The modules that will be displayed on the right side of the status bar
  right:
    - SystemInfo
    # This modules will form a group sharing the same element in the status bar
    - [Clock, Privacy, Settings]
# App launcher command, it will be used to open the launcher,
# without a value the related button will not appear
appLauncherCmd: "~/.config/rofi/launcher.sh" # optional, default None
# Clipboard command, it will be used to open the clipboard menu,
# without a value the related button will not appear
clipboardCmd: "cliphist-rofi-img | wl-copy" # optional, default None
# Update module configuration.
# Without a value the related button will not appear.
updates: # optional, default None
  # The check command will be used to retrieve the update list.
  # It should return something like `package_name version_from -> version_to\n`
  checkCmd: "checkupdates; paru -Qua" # required
  # The update command is used to init the OS update process
  updateCmd: 'alacritty -e bash -c "paru; echo Done - Press enter to exit; read" &' # required
# Maximum number of chars that can be present in the window title
# after that the title will be truncated
truncateTitleAfterLength: 150 # optional, default 150
# Workspaces module configuration, optional
workspaces:
  # The visibility mode of the workspaces, possible values are:
  # All: all the workspaces will be displayed
  # MonitorSpecific: only the workspaces of the related monitor will be displayed
  visibilityMode: All # optional, default All
  # Enable filling with empty workspaces
  # For example:
  # With this flag set to true if there are only 2 workspaces,
  # the workspace 1 and the workspace 4, the module will show also
  # two more workspaces, the workspace 2 and the workspace 3
  enableWorkspaceFilling: false # optional, default false
# The system module configuration
system:
  cpuWarnThreshold: 60 # cpu indicator warning level (default 60)
  cpuAlertThreshold: 80 # cpu indicator alert level (default 80)
  memWarnThreshold: 70 # mem indicator warning level (default 70)
  memAlertThreshold: 85 # mem indicator alert level (default 85)
  tempWarnThreshold: 60 # temperature indicator warning level (default 60)
  tempAlertThreshold: 80 # temperature indicator alert level (default 80)
# Clock module configuration
clock:
  # clock format see: https://docs.rs/chrono/latest/chrono/format/strftime/index.html
  format: "%a %d %b %R" # optional, default: %a %d %b %R
# Settings module configuration
settings:
  # command used for lock the system
  # without a value the related button will not appear
  lockCmd: "hyprlock &" # optional, default None
  # command used to open the sinks audio settings
  # without a value the related button will not appear
  audioSinksMoreCmd: "pavucontrol -t 3" # optional default None
  # command used to open the sources audio settings
  # without a value the related button will not appear
  audioSourcesMoreCmd: "pavucontrol -t 4" # optional, default None
  # command used to open the network settings
  # without a value the related button will not appear
  wifiMoreCmd: "nm-connection-editor" # optional, default None
  # command used to open the VPN settings
  # without a value the related button will not appear
  vpnMoreCmd: "nm-connection-editor" # optional, default None
  # command used to open the Bluetooth settings
  # without a value the related button will not appear
  bluetoothMoreCmd: "blueman-manager" # optional, default None
# Appearance config
# Each color could be a simple hex color like #228800 or an
# object that define a base hex color and two optional variant of that color (a strong one and a weak one)
# and the text color that should be used with that base color
# example:
# backgroundColor:
#   base: #448877
#   strong: #448888 -- optional default autogenerated from base color
#   weak: #448855 -- optional default autogenarated from base color
#   text: #ffffff -- optional default base text color
appearance:
  backgroundColor: "#1e1e2e" # used as a base background color for header module button
  primaryColor: "#fab387" # used as a accent color
  secondaryColor: "#11111b" # used for darker background color
  successColor: "#a6e3a1" # used for success message or happy state
  dangerColor: "#f38ba8" # used for danger message or danger state (the weak version is used for the warning state
  textColor: "#f38ba8" # base default text color
  # this is a list of color that will be used in the workspace module (one color for each monitor)
  workspaceColors:
    - "#fab387"
    - "#b4befe"
  # this is a list of color that will be used in the workspace module
  # for the special workspace (one color for each monitor)
  # optional, default None
  # without a value the workspaceColors list will be used
  specialWorkspaceColors:
    - "#a6e3a1"
    - "#f38ba8"
```

## Some screenshots

I will try my best to keep these screenshots as updated as possible but some details
could be different

<img src="https://raw.githubusercontent.com/MalpenZibo/ashell/main/screenshots/ashell.png"></img>

| ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/screenshots/updates-panel.png) | ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/screenshots/settings-panel.png)  |
| ------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/screenshots/power-menu.png)    | ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/screenshots/sinks-selection.png) |
| ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/screenshots/network-menu.png)  | ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/screenshots/bluetooth-menu.png)  |
| ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/screenshots/vpn-menu.png)      | ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/screenshots/airplane-mode.png)   |
