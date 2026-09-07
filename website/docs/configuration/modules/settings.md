---
sidebar_position: 7
---

# Settings

This module provides access to system settings like audio, network, bluetooth,  
battery, power profile and idle inhibitor.

It displays in the status bar indicators about:

- Audio volume
- Microphone volume
- Network status
- Bluetooth connection status
- Battery status
- Peripheral battery status
- Screen brightness
- Power profile
- Idle inhibitor status
- VPN connection status

And lets you interact with these settings:

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

You can configure this module.

With the `lock_cmd` option you can set a command to lock  
the system, if not set the related button will not appear.

With the `shutdown_cmd`, `suspend_cmd`, `reboot_cmd`, and `logout_cmd`,
you can change the related commands to
shut down, suspend, reboot, or log out of the system.  
These parameters are optional and have the following default values:

```toml
shutdown_cmd = "shutdown now"
suspend_cmd = "systemctl suspend"
reboot_cmd = "systemctl reboot"
logout_cmd = "loginctl kill-user $(whoami)"
```

The `lock_cmd` parameter is optional. If not set, or set to an empty string, the lock button will not appear.

The `hibernate_cmd` parameter is optional. If not set, or set to an empty string, the hibernate button will not appear.

With the `audio_sinks_more_cmd` and `audio_sources_more_cmd`  
options you can set commands to open the audio settings  
for sinks and sources, if not set the related buttons will not appear.  
When configured, right-clicking the speaker or microphone indicators (or their quick settings buttons) launches the respective command immediately.

With the `wifi_more_cmd`, `vpn_more_cmd` and `bluetooth_more_cmd` options  
you can set commands to open the network, VPN and bluetooth settings.  
Right-clicking the Wi-Fi, VPN, Bluetooth or airplane-mode quick settings buttons (and the Wi-Fi indicator in the bar) triggers these commands directly when they are set.

Optional command fields in this module treat empty or whitespace-only strings as unset.

With the `remove_airplane_btn` option you can remove the airplane mode button.

With the `remove_idle_btn` option you can remove the idle inhibitor button.

## Tooltips

By default, hovering over the status bar indicators shows a tooltip describing
each one. With the `enable_tooltips` option you can disable these hover tooltips.

The default value is `true`.

```toml
[settings]
enable_tooltips = false
```

## Indicator Format Options

With the format options you can customize how different indicators are displayed in the status bar.

Every format option accepts the same set of values, but not every indicator has
something meaningful to show for each of them. The values are:

| Value | Shows |
| --- | --- |
| `Icon` | Only the icon |
| `Percentage` (alias `Value`) | Only the value (percentage, count, or strength) |
| `IconAndPercentage` (alias `IconAndValue`) | The icon followed by the value |
| `Time` | Only the remaining time (battery indicators) |
| `IconAndTime` | The icon followed by the remaining time |
| `Name` | Only the name |
| `IconAndName` | The icon followed by the name |

:::info
`Name` and `IconAndName` change what is rendered only for the **network**
indicator, which is the one that has a name to show. Every other indicator has
no name, so it keeps rendering its usual value: `Name` behaves like
`Percentage` and `IconAndName` like `IconAndPercentage`. The one exception is
`peripheral_battery_format`, where both values fall back to icon-only.

The defaults are not uniform either: `battery_format` defaults to
`IconAndPercentage`, every other format option defaults to `Icon`.
:::

### Battery Format

With the `battery_format` option you can customize the battery indicator format.

The possible values are:

- `Icon` - Show only the battery icon
- `Percentage` - Show only the battery percentage
- `IconAndPercentage` - Show both the battery icon and percentage (default)
- `Time` - Show smart time display (time to full when charging, time to empty when discharging, "100%" when full)
- `IconAndTime` - Show battery icon with smart time display
- `Name` - Accepted, but a battery has no name: renders like `Percentage`
- `IconAndName` - Accepted, but renders like `IconAndPercentage`

```toml
[settings]
battery_format = "IconAndPercentage"
```

### Battery Hide When Full

With the `battery_hide_when_full` option you can hide the battery indicator when the battery is fully charged.

The default value is `false`.

```toml
[settings]
battery_hide_when_full = true
```

### Battery Time Display Behavior

The `Time` and `IconAndTime` formats provide intelligent time display:

- **When charging**: Shows time until full (e.g., "45m", "2h 15m")
- **When discharging**: Shows time until empty (e.g., "1h 30m", "3h 45m")
- **When at 100%, full, or charging with no estimate yet**: Shows "100%"
- **When discharging with no estimate yet**: Shows "Calculating..." (translated),
  for both the system battery and peripherals
- **When not charging**: Shows the plain percentage
- **When the battery status is unknown**: Shows nothing

```toml
[settings]
# Show time only for system battery
battery_format = "Time"

# Show icon + time for peripheral batteries
peripheral_battery_format = "IconAndTime"

# Example outputs:
# Charging: "45m" or "🔋 45m"
# Discharging: "2h 15m" or "🔋 2h 15m"
# Full: "100%" or "🔋 100%"
```

### Peripheral Battery Format

In the same way it's possible to customize the peripheral battery indicator
format with the `peripheral_battery_format` option. It accepts the same values,
except that `Name` and `IconAndName` render icon-only here; a peripheral
battery has no name to display in the bar.

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
audio_indicator_format = "Icon"
microphone_indicator_format = "Icon"
network_indicator_format = "Icon"
bluetooth_indicator_format = "Icon"
```

### Peripheral Expanded By Default

When set to `true`, the peripheral battery submenu will be open by default when opening the settings menu.

The default value is `false`.

```toml
[settings]
peripheral_expanded_by_default = true
```

### Audio Format

With the `audio_indicator_format` option you can customize the audio volume indicator format.
The value it shows is the current output volume as a percentage.

The default value is `Icon`.

```toml
[settings]
audio_indicator_format = "IconAndPercentage"
```

### Volume Step

With the `volume_step` option you can configure the increment/decrement step for volume IPC commands (`volume-up` / `volume-down`).

The default value is `5` (percent). Valid range: 1–50.

```toml
[settings]
volume_step = 10
```

### Max Volume

With the `max_volume` option you can allow volume to exceed 100% hardware level. When set above 100, the slider extends beyond normal range and visual overdrive indicators appear (red slider fill, overdrive icon).

The default value is `100`. Valid range: 1–200.

```toml
[settings]
max_volume = 150
```

### Microphone Format

With the `microphone_indicator_format` option you can customize the microphone volume indicator format.

The default value is `Icon`.

```toml
[settings]
microphone_indicator_format = "IconAndPercentage"
```

### Network Format

With the `network_indicator_format` option you can customize the network connection indicator format.
For WiFi connections, this shows the signal strength as a percentage.
You can also use `Name` or `IconAndName` to display the connected network name (the SSID for WiFi
connections, the interface name for wired connections, or the VPN name for VPN connections).

The default value is `Icon`.

```toml
[settings]
network_indicator_format = "IconAndPercentage"
# or, to show the SSID next to the wifi icon:
# network_indicator_format = "IconAndName"
```

### Bluetooth Format

With the `bluetooth_indicator_format` option you can customize the bluetooth indicator format.
The value it shows is the number of connected devices.

The indicator is only rendered while Bluetooth is on. When no device is
connected there is no count to show, so the bar falls back to a plain Bluetooth
icon whatever format you set.

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

## Status Bar Indicators

With the `indicators` option you can customize which status indicators
are displayed in the status bar and in what order they appear.

Available indicators are:

- `IdleInhibitor` - Shows an icon when idle inhibitor is active
- `PowerProfile` - Shows the current power profile icon
- `Audio` - Shows the audio volume level icon
- `Microphone` - Shows the microphone volume level and mute status
- `Network` - Shows the network connection status icon
- `Vpn` - Shows the VPN connection status icon
- `Bluetooth` - Shows a Bluetooth icon when connected to at least one device
- `Battery` - Shows the battery level and charging status
- `PeripheralBattery` - Shows the peripheral battery status
- `Brightness` - Shows the current brightness level

```toml
[settings]
# Customize which indicators to show and their order
indicators = ["Battery", "Bluetooth", "Network", "Audio", "Microphone"]

# The default, for reference (shown in this order):
indicators = ["IdleInhibitor", "PowerProfile", "Audio", "Microphone", "Bluetooth", "Network", "Vpn", "Battery"]
```

## Custom Buttons

You can add custom buttons to the settings panel using the `CustomButton` configuration.
These buttons can execute commands when clicked.

### Button Behavior

- If `status_command` is provided, the button acts as a **toggle** with visual state tracking
- If `status_command` is not provided, the button acts as a **launcher** (simple command execution)

An empty or whitespace-only `status_command` is treated as not provided.

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
# Optional: show hibernate button
hibernate_cmd = "systemctl hibernate"
remove_airplane_btn = true
remove_idle_btn = true
indicators = ["Battery", "Bluetooth", "Network", "Audio", "Microphone", "Brightness"]

battery_format = "IconAndTime"
peripheral_battery_format = "Time"
peripheral_indicators = "All"


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
