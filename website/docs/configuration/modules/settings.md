---
sidebar_position: 12
---

# Settings

This module provides access to system settings like audio, network, bluetooth,  
battery, power profile and idle inhibitor.

It displays in the status bar indicator about:

- Audio volume
- Network status
- Battery status
- Power profile
- Idle inhibitor status

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
- Suspend, logout, reboot, or shutdown the system

You can configure some function of this module.

With the `lock_cmd` option you can set a command to lock  
the system, if not set the related button will not appear.

With the `shutdown_cmd`, `suspend_cmd`, `reboot_cmd`, and `logout_cmd`,  
you can change the related commands to shut down, suspend, reboot,
or log out of the system.  
These parameters are optional and have the following default values:

```toml
shutdown_cmd = "shutdown now"
suspend_cmd = "systemctl suspend"
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
- **Nerd Font symbols**: ``, ``, ``, etc. (requires Nerd Font installed)

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
