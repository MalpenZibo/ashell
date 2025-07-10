---
sidebar_position: 13
---

# Custom Modules

This special modules let you extend the functionality of
ashell by creating your own simple modules.

A custom module lets you execute a command on click,
or lets you display the output of a command.

To define a custom module you need to provide the following fields:

- `name`: The name of the module, which will be used to link it in the [modules definitions](./index.md).
- `icon`: The icon to display in the status bar.
- `command`: The command to execute when the module is clicked.
- `listen_cmd`: (optional) The command to execute to update the module output.
- `icons`: (optional) A set of regex to change the icon based on the output
  of the `listen_cmd`.
- `alert`: (optional) A regex to show a red "alert" dot on the icon
  when the output of the `listen_cmd` matches the regex.

## Listen command

### Format

The `listen_cmd` command should output a
waybar json-style output, using the `alt` and `text` field

#### Listen command format example

```bash
{
  "text": "3",
  "alt": "notification"
}
```

### Icons

Any number of regex can be used to change the icon based on the alt field

#### Icons Example

```toml
icons.'dnd.\*' = ""
```

### Alert

It's possible to add a regex to this field to show a red "alert" dot on the icon
when the output of the `listen_cmd` matches the regex.

#### Alert Example

```toml
alert = ".\*notification"
```

## Notification Module Example

Using swaync-client it's possible to create a notification module

```toml
[[CustomModule]]
name = "CustomNotifications"
icon = ""
command = "swaync-client -t -sw"
listen_cmd = "swaync-client -swb"
icons.'dnd.*' = ""
alert = ".*notification"
```

## App launcher Module Example

Using `walker` it's possible to create an app launcher module

```toml
[[CustomModule]]
name = "AppLauncher"
icon = "󱗼"
command = "walker"
```
