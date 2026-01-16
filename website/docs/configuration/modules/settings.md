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

## Indicator Format Options

With the format options you can customize how different indicators are displayed in the status bar.

All format options support the same values:

- `Icon` - Show only the icon
- `Percentage` (or `Value`) - Show only the numeric value (percentage, count, or strength)
- `IconAndPercentage` (or `IconAndValue`) - Show both the icon and the numeric value (default)

### Battery Format

With the `battery_format` option you can customize the battery indicator format.

The default value is `IconAndPercentage`.

```toml
[settings]
battery_format = "IconAndPercentage"
```

### Peripheral Battery Format

In the same way it's possible to customize the peripheral battery indicator format.
The possible values are the same as above, but you need to use
the `peripheral_battery_format` option.
The default value is `Icon`.

```toml
[settings]
peripheral_battery_format = "Icon"
```

### Audio Format

With the `audio_indicator_format` option you can customize the audio volume indicator format.

The default value is `Icon`.

```toml
[settings]
audio_indicator_format = "IconAndPercentage"
```

### Network Format

With the `network_indicator_format` option you can customize the network connection indicator format.
For WiFi connections, this shows the signal strength as a percentage.

The default value is `Icon`.

```toml
[settings]
network_indicator_format = "IconAndPercentage"
```

### Bluetooth Format

With the `bluetooth_indicator_format` option you can customize the bluetooth indicator format.
When devices are connected, this shows the number of connected devices.

The default value is `Icon`.

```toml
[settings]
bluetooth_indicator_format = "IconAndValue"
```

### Brightness Format

With the `brightness_indicator_format` option you can customize the brightness indicator format.

The default value is `Icon`.

```toml
[settings]
brightness_indicator_format = "IconAndPercentage"
```

## Peripheral Indicators

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
audio_indicator_format = "Icon"
network_indicator_format = "Icon"
bluetooth_indicator_format = "Icon"
brightness_indicator_format = "Icon"
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
- `Brightness` - Shows the current brightness level

```toml
[settings]
# Customize which indicators to show and their order
indicators = ["Battery", "Bluetooth", "Network", "Audio"]

# The default value is the following, the items are shown in this order:
indicators = ["IdleInhibitor", "PowerProfile", "Audio", "Bluetooth", "Network", "Vpn", "Battery", "Brightness"]
```

## Custom Buttons

You can add custom buttons to the settings panel using the `CustomButton` configuration.
These buttons can execute commands when clicked.

### Button Behavior

- If `status_command` is provided, the button acts as a **toggle** with visual state tracking
- If `status_command` is not provided, the button acts as a **launcher** (simple command execution)

### Configuration

| Field            | Required | Description                                                 |
| ---------------- | -------- | ----------------------------------------------------------- |
| `name`           | Yes      | Display name of the button                                  |
| `icon`           | Yes      | Icon to display (Unicode emoji or Nerd Font symbol)         |
| `command`        | Yes      | Shell command to execute when button is clicked             |
| `status_command` | No       | Command to check if toggle is active (exit code 0 = active) |
| `tooltip`        | No       | Tooltip text shown on hover                                 |

#### Icon Support

The `icon` field accepts:

- **Unicode emoji**: `⌨️`, `🖥️`, `📁`, `🌐`, etc.
- **Nerd Font symbols**: `󰌓`, ``, ``, etc. (requires Nerd Font installed)

Both are rendered using the `Symbols Nerd Font` and will display correctly in the UI.

#### Command Execution

Both `command` and `status_command` are executed through **bash shell** (`bash -c`), which means you can use:

- Shell features: pipes (`|`), redirects (`>`), logical operators (`&&`, `||`)
- Environment variables: `$HOME`, `$USER`, etc.
- Globs: `*.txt`, `~/Documents/*`

:::warning Security Note
Commands are executed with your user privileges. Be careful with commands from untrusted sources, as they have full shell access.
:::

#### Status Command Timeout

Each `status_command` has a **1 second timeout**. If the command doesn't complete within this time:

- The button state will be shown as "unknown" (grayed out)
- The process will be killed automatically
- An error will be logged for debugging

```toml
# Toggle button example (with status_command)
[[settings.CustomButton]]
name = "Virtual Keyboard"
icon = "⌨️"
command = "/path/to/toggle-keyboard.sh"
status_command = "/path/to/check-keyboard-status.sh"
tooltip = "Toggle On-Screen Keyboard"

# Launcher button example (without status_command)
[[settings.CustomButton]]
name = "Terminal"
icon = ""
command = "alacritty"
tooltip = "Open Terminal"
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
indicators = ["Battery", "Bluetooth", "Network", "Audio", "Brightness"]

[[settings.CustomButton]]
name = "Virtual Keyboard"
icon = "⌨️"
command = "toggle-onscreen-keyboard.sh"
status_command = "pgrep -x onboard"
tooltip = "Toggle On-Screen Keyboard"

[[settings.CustomButton]]
name = "File Manager"
icon = ""
command = "nautilus"
tooltip = "Open Files"
```
