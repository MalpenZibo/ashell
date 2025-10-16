---
sidebar_position: 12
---

# Settings

This module provides access to system settings like audio, network, bluetooth,  
battery, power profile and idle inhibitor.

It displays in the status bar indicators about:

- Audio volume
- Network status
- Bluetooth connection status
- Battery status
- Power profile
- Idle inhibitor status
- VPN connection status

And let you interact with these settings:

- Change audio and microphone volume
- Change audio output and input devices
- Toggle network connection
- Toggle VPN connection
- Toggle airplane mode
- Change brightness
- Toggle bluetooth
- Change power profile
- Toggle idle inhibitor
- Lock the screen
- Suspend, hibernate, logout, reboot, or shutdown the system

You can configure some function of this module.

With the `lock_cmd` option you can set a command to lock  
the system, if not set the related button will not appear.

With the `shutdown_cmd`, `suspend_cmd`, `hibernate_cmd`, `reboot_cmd`,
and `logout_cmd`, you can change the related commands to
shut down, suspend, hibernate, reboot, or log out of the system.  
These parameters are optional and have the following default values:

```toml
shutdown_cmd = "shutdown now"
suspend_cmd = "systemctl suspend"
hibernate_cmd = "systemctl hibernate"
reboot_cmd = "systemctl reboot"
logout_cmd = "loginctl kill-user $(whoami)"
```

With the `audio_sinks_more_cmd` and `audio_sources_more_cmd`  
options you can set commands to open the audio settings  
for sinks and sources, if not set the related buttons will not appear.

With the `wifi_more_cmd`, `vpn_more_cmd` and `bluetooth_more_cmd` options  
you can set commands to open the network, VPN and bluetooth settings.

With the `remove_airplane_btn` option you can remove the airplane mode button.

With the `remove_idle_btn` option you can remove the idle inhibitor button.

With the `battery_format` option you can customize the battery indicator format.

The possible values are:

- `Icon` - Show only the battery icon
- `Percentage` - Show only the battery percentage
- `IconAndPercentage` - Show both the battery icon and percentage (default)

In the same way it's possible to customize the peripheral battery indicator format.
The possible values are the same as above, but you need to use
the `peripheral_battery_format` option.
The default value is `Icon`.

With the `peripheral_indicators` you can decide which peripheral battery indicators
are shown in the status bar.

The possible values are:

- `All` - Show all peripheral battery indicators (default)
- `Specific` - Show only the peripheral battery indicators in the specified categories.
  The possible categories are:
  - `Keyboard`
  - `Mouse`
  - `Headphones`
  - `Gamepad`

```toml
[settings]
battery_format = "IconAndPercentage"
peripheral_battery_format = "Icon"
peripheral_indicators = { Specific = ["Gamepad", "Keyboard"] }
```

## Status Bar Indicators

With the `indicators` option you can customize which status indicators
are displayed in the status bar and in what order they appear.

Available indicators are:

- `IdleInhibitor` - Shows an icon when idle inhibitor is active
- `PowerProfile` - Shows the current power profile icon
- `Audio` - Shows the audio volume level icon
- `Network` - Shows the network connection status icon
- `Vpn` - Shows the VPN connection status icon
- `Bluetooth` - Shows a Bluetooth icon when connected to at least one device
- `Battery` - Shows the battery level and charging status
- `PeripheralBattery` - Shows the peripheral battery status

```toml
[settings]
# Customize which indicators to show and their order
indicators = ["Battery", "Bluetooth", "Network", "Audio"]

# The default value is the following, the items are shown in this order:
indicators = ["IdleInhibitor", "PowerProfile", "Audio", "Bluetooth", "Network", "Vpn", "Battery"]
```

## Example

In the following example we use:

- `hyprlock` to lock the screen
- `pavucontrol` to open the audio settings for sinks and sources  
  directly in the correct tab.
- `nm-connection-editor` to open the wifi and VPN settings
- `blueman-manager` to open the bluetooth settings

We also disable the airplane mode button and the idle inhibitor button.

```toml
[settings]
lock_cmd = "hyprlock &"
audio_sinks_more_cmd = "pavucontrol -t 3"
audio_sources_more_cmd = "pavucontrol -t 4"
wifi_more_cmd = "nm-connection-editor"
vpn_more_cmd = "nm-connection-editor"
bluetooth_more_cmd = "blueman-manager"
remove_airplane_btn = true
remove_idle_btn = true
# Only show battery, bluetooth, network and audio indicators
indicators = ["Battery", "Bluetooth", "Network", "Audio"]
```
