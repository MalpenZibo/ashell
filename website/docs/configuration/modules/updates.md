---
sidebar_position: 3
---

# Updates

This module provides information about available updates for the system.

To configure this module, you need to specify a command that
will check for new system updates and a command that will launch the system updates.

:::info

Without this configuration, the module will not appear in the status bar.

:::

The module automatically checks for updates on a configurable interval (defaults to once per hour). You can also manually check for updates using the "Check now" button in the menu.

### Configuration

| Field        | Description                                                      |
| ------------ | ---------------------------------------------------------------- |
| `check_cmd`  | Command that outputs pending updates (one per line).             |
| `update_cmd` | Command that launches your system updates workflow.              |
| `interval`   | Optional polling interval in seconds (minimum 60, default 3600). |

The check command should return a list of updates,
one package per line in the following format:

`package_name version_from -> version_to`

## Output Example

```text
calf 0.90.7-1 -> 0.90.8-1
hyprutils 0.8.0-1 -> 0.8.1-1
lazygit 0.52.0-1 -> 0.53.0-1
```

## Example

In this example, I am using an Arch Linux distribution, with `paru` as my
AUR package manager and `alacritty` as a terminal emulator.

```toml
[updates]
check_cmd = "checkupdates; paru -Qua"
update_cmd = 'alacritty -e bash -c "paru; echo Done - Press enter to exit; read" &'
interval = 3600
```
