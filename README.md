# Ashell

A un-customizable Wayland bar for Hyprland compositor and Arch linux distributions. 

WIP, highly unstable

### Does it only works on Hyprland and Arch linux?
While it's currently tailored for Hyprland and Arch Linux, 
it could potentially work with other compositors. 

However, at present, it relies on [hyprland-rs](https://github.com/hyprland-community/hyprland-rs) 
to gather information about the active window and workspaces and I haven't implemented any 
feature flags to disable these functionalities or alternative methods to obtain this data.

At the moment, `ashell` is functional only on Arch Linux due to the lack of customization options. 
The issue lies with the `update` indicator, which I plan to address. 
I intend to create a more generic solution to fetch the update list or provide an option 
to disable this feature.

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

Almost nothing but it could be useful if you want to create your own bar 
or if you have to read some examples on how to get information from `dbus`.

So feel free to fork this project and customize it for your needs.

## Features
The main inspiration is the gnome shell bar. 
I'm using the catpuccin mocha color palette.

- Lancher button
- Arch linux OS Update
- Hyprland Active Window
- Hyprland Workspaces
- System Informations (cpu, ram, temperature)
- Date time
- Settings panel
    - power menu
    - battery information
    - pulse audio sources and sinks
    - screen brightness
    - network stuff
    - VPN
    - bluetooth

## Requirements
- a `~/.config/rofi/launcher.sh` script to open the app lancher
- `nerd fonts icons`
- `hyprlock` to lock the session
- `checkupdates` and `paru` to get the list of updates
- probably other stuff to avoid "unexpected" crashes :D

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

