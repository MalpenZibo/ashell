# Ashell

A ready to go Wayland status bar for Hyprland

> If you have an issue with the transparency you could try launching ashell with WGPU_BACKEND=gl. This env var forces wgpu to use OpenGL instead of Vulkan. It seems that wgpu has some issues with AMD GPU and Vulkan transparency.

### Does it only work on Hyprland?
While it's currently tailored for Hyprland, it could work with other compositors. 

However, it currently relies on [hyprland-rs](https://github.com/hyprland-community/hyprland-rs) 
to gather information about the active window and workspaces. I haven't implemented any 
feature flags to disable these functionalities or alternative methods to obtain this data.

## Install

[![Packaging status](https://repology.org/badge/vertical-allrepos/ashell.svg)](https://repology.org/project/ashell/versions)

### Arch Linux

You can get the official Arch Linux package from the AUR:

```
paru/yay -S ashell-git # to get the latest source version
```

### Nix

To install ashell using the nix package be sure to enable flakes and then run

```
nix profile install github:MalpenZibo/ashell
```

### NixOS

I haven't tested ashell on NixOS.

To enable this flake use (I'm not an expert and I haven't tested this configuration
but I'm quite sure that if you use NixOS you are smart enoght to add ashell to your configuration :D

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
but I'm quite sure that if you use NixOS you are smart enough to add ashell to your configuration :D


## Features

- Lancher button
- OS Updates indicator
- Hyprland Active Window
- Hyprland Workspaces
- System Information (CPU, RAM, Temperature)
- Date time
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

## Configuration
The configuration uses the yaml file format and is named `~/.config/ashell.yml`

``` yaml
# Ashell log level filter, possible values "DEBUG" | "INFO" | "WARNING" | "ERROR". Needs reload
logLevel: "INFO" # optional, default "INFO"
# Ashell bar position, possible values Top | Bottom. Needs reload
position: Top # optional, default Top
# App lancher commanda, it will be used to open the launcher,
# without a value the related button will not appear
appLauncherCmd: "~/.config/rofi/launcher.sh" # optional, default None 
# Update module configuration. 
# Without a value the related button will not appear.
updates: # optional, default None 
  # The check command will be used to retrieve the update list.
  # It should return something like `package_name version_from -> version_to\n`
  checkCmd: "checkupdates; paru -Qua" # required
  # The update command is used to init the OS update process
  updateCmd: "alacritty -e bash -c \"paru; echo Done - Press enter to exit; read\" &" # required
# Maximum number of chars that can be present in the window title
# after that the title will be truncated 
truncateTitleAfterLength: 150 # optional, default 150
# The system module configuration
system: 
  disabled: false # Enable or disable the system monitor module
  cpuWarnThreshold: 6O # cpu indicator warning level (default 60)
  cpuAlertThreshold: 8O # cpu indicator alert level (default 80)
  memWarnThreshold: 7O # mem indicator warning level (default 70)
  memAlertThreshold: 85 # mem indicator alert level (default 85)
  tempWarnThreshold: 6O # temperature indicator warning level (default 60)
  tempAlertThreshold: 8O # temperature indicator alert level (default 80)
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
```

### So, what's the purpose of this project?
While, I could have used [waybar](https://github.com/Alexays/Waybar) that's for sure is a 
a great project but I wanted something more sophisticated 
with submenus and other stuff.

I tried with other great projects like [eww](https://github.com/elkowar/eww) but
instead of writing or copy-paste eww configurations I prefered to create 
my Wayland bar.

So, I <del>copy-pasted from iced pop-os fork</del> created a layer to interact 
with wayland layer shell protocol from iced [Iced SCTK](https://github.com/MalpenZibo/iced_sctk) 
and I started to create this project.

So feel free to fork this project and customize it for your needs 
or you could try to compile and use it.

## Some screenshots

#### Main bar
![MainBar](https://raw.githubusercontent.com/MalpenZibo/ashell/main/screenshots/ashell.png)

#### Updates
![Updates](https://raw.githubusercontent.com/MalpenZibo/ashell/main/screenshots/updates-panel.png)

#### Settings
![Settings](https://raw.githubusercontent.com/MalpenZibo/ashell/main/screenshots/settings-panel.png)

#### Power menu
![PowerMenu](https://raw.githubusercontent.com/MalpenZibo/ashell/main/screenshots/power-menu.png)

#### Pulse Audio
![PulseAudio](https://raw.githubusercontent.com/MalpenZibo/ashell/main/screenshots/sinks-selection.png)

#### Network
![Network](https://raw.githubusercontent.com/MalpenZibo/ashell/main/screenshots/network-menu.png)

#### Bluetooth
![Bluetooth](https://raw.githubusercontent.com/MalpenZibo/ashell/main/screenshots/bluetooth-menu.png)


