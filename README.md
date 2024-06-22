# Ashell

Barely customizable Wayland status bar for Hyprland compositor. 

WIP, highly unstable

### Does it only work on Hyprland?
While it's currently tailored for Hyprland, it could work with other compositors. 

However, at present, it relies on [hyprland-rs](https://github.com/hyprland-community/hyprland-rs) 
to gather information about the active window and workspaces and I haven't implemented any 
feature flags to disable these functionalities or alternative methods to obtain this data.

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
logLevel: "INFO" # possible values "DEBUG" | "INFO" | "WARNING" | "ERROR" (default value "INFO")
appLauncherCmd: "~/.config/rofi/launcher.sh" # This command will be used to open the launcher, default value None, the button will not appear 
# Update module config. The check command will be used to retrieve the update list.
# It should return something like `package_name version_from -> version_to\n`
# The update command is used to init the OS update process
updates:
  checkCmd: "checkupdates; paru -Qua"
  updateCmd: "alacritty -e bash -c \"paru; echo Done - Press enter to exit; read\" &"
system:
  disabled: false # Enable or disable the system monitor module
settings:
  lockCmd: "hyprlock &" # command used for lock the system
  audioSinksMoreCmd: "pavucontrol -t 3" # command used to open the  sinks audio settings (default none -> the button "More" will not appear)
  audioSourcesMoreCmd: "pavucontrol -t 4" # command used to open the sources audio settings (default none -> the button "More" will not appear) 
  wifiMoreCmd: "nm-connection-editor" # command used to open the network settings (default none -> the button "More" will not appear) 
  vpnMoreCmd: "nm-connection-editor" # command used to open the VPN settings (default none -> the button "More" will not appear) 
  bluetoothMoreCmd: "blueman-manager" # command used to open the Bluetooth settings (default none -> the button "More" will not appear) 
```

### So, what's the purpose of this project?
While, I could have used [waybar](https://github.com/Alexays/Waybar) that's for sure is a 
a great project but I wanted something more sophisticated 
with submenus and other stuff.

I tried with other great projects like [eww](https://github.com/elkowar/eww) but
instead of writing or copy-paste eww configurations I prefered to create 
my Wayland bar.

So, using the pop-os fork of [iced](https://github.com/pop-os/iced), I started to 
create this project.

In the end, what can this project do for you? 

Almost nothing but it could be useful if you want to create your own status bar 
or if you have to read some examples on how to get information from `dbus`.

So feel free to fork this project and customize it for your needs.

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

#### Power Profile, Idle inhibitor
![PowerProfileIdleInhibitor](https://raw.githubusercontent.com/MalpenZibo/ashell/main/screenshots/power-profile-idle-indicator.png)

