---
sidebar_position: 14
---

# Custom Modules

This special module type lets you extend the functionality of Ashell
by creating your own simple components.

A **custom module** allows you to:

- Display the output of a command (live updates from a continuously running process).
- Run a command when the module is clicked.
- Change icons dynamically based on output.
- Show an alert indicator based on specific conditions.

:::warning

Ashell comes with a set of default icons that are used internally.

If you specify a font icon in the custom module configuration remember
to install the font with that icon on your system.

For example you can use [Nerd Fonts](https://www.nerdfonts.com/)

:::

## Configuration

To define a custom module, use the following fields:

- `name`: Name of the module. Use this to refer to it in the [modules definitions](./index.md).
- `type` _(optional)_: Display type. Can be `Button` (clickable, default) or `Text` (display only).
- `icon`: Icon displayed in the status bar (for `button` type).
- `command`: Command to execute when the module is clicked (for `button` type).
- `listen_cmd` _(optional)_: Command to run in the background to update the module’s display.
- `icons` _(optional)_: Regex-to-icon mapping to change the icon based on the `listen_cmd` output (for `button` type`). The first matching regex wins; since the mappings are stored as a map, the evaluation order is not guaranteed. Prefer mutually exclusive regexes or keep patterns precise to avoid ambiguous matches.
- `alert` _(optional)_: Regex to trigger a red alert dot on the icon when
  matched in the `listen_cmd` output (for `button` type).

---

## `listen_cmd`

The `listen_cmd` is started once on startup and should continuously run in the background,
outputting JSON whenever the module's display should be updated. The most recent output
is used to update the module's display.

:::tip How `listen_cmd` Works

The `listen_cmd` is **not** executed periodically. Instead, it runs as a long-lived process
that outputs JSON whenever the displayed information changes. This is ideal for:

- **Status monitoring**: Commands that watch for changes and output when they occur
- **Event listeners**: Commands that wait for specific events and report them
- **Live data**: Commands that continuously stream status updates

For example, a notification listener might output JSON only when new notifications arrive,
rather than polling every second.

:::

The `listen_cmd` should output JSON in
the [Waybar format](https://github.com/Alexays/Waybar/wiki/Module:-Custom#script-output),
using `text` and `alt` fields.

### Example Output

```json
{
  "text": "3",
  "alt": "notification"
}
```

---

## Dynamic Icons

You can change the icon depending on the value of `alt` in the `listen_cmd` output.

### Icons Example

```toml
icons.'dnd.*' = ""
```

This will change the icon to `` when `alt` matches `dnd.*`.

---

## Alerts

Use the `alert` field to show a red dot on the module icon if the output
matches a given regex.

### Alerts Example

```toml
alert = ".*notification"
```

---

## Examples

### Text-only Module (e.g., Custom Clock)

Text modules display only the text output from `listen_cmd` without any click action:

```toml
[[CustomModule]]
name = "MyClock"
type = "Text"
listen_cmd = "echo '{\"text\": \"$(date +'%H:%M')\", \"alt\": \"\"}'"
```

**Note for Fish Shell Users**: If you use Fish shell, wrap the command in `sh -c` for POSIX compatibility:

```toml
listen_cmd = "sh -c 'while true; do echo \"{\\\"text\\\": \\\"$(date +\"%H:%M\")\\\", \\\"alt\\\": \\\"\\\"}\"; sleep 1; done'"
```

### Button Module with Icon (Interactive)

Button modules display an icon and/or text with a click action:

```toml
[[CustomModule]]
name = "CustomNotifications"
type = "Button"
icon = ""
command = "swaync-client -t -sw"
listen_cmd = "swaync-client -swb"
icons.'dnd.*' = ""
alert = ".*notification"
```

### Button Module with Icon Only

```toml
[[CustomModule]]
name = "AppLauncher"
type = "Button"
icon = "󱗼"
command = "walker"
```

### Button Module with Text Output

Button modules can also display text output from `listen_cmd` with a click action:

```toml
[[CustomModule]]
name = "Clipboard"
type = "Button"
command = "cliphist-rofi-img | wl-copy"
listen_cmd = "echo '{\"text\": \"📋\", \"alt\": \"\"}'"
```

### Notifications (with wired)

```toml
[[CustomModule]]
name = "CustomNotifications"
type = "Button"
icon = ""
command = "wired --show 5"
listen_cmd = "wired count"
icons.'dnd.*' = ""
alert = ".*notification"
```

```toml
[[CustomModule]]
name = "CustomNotifications"
icon = ""
command = "swaync-client -t -sw"
listen_cmd = "swaync-client -swb"
icons.'dnd.*' = ""
alert = ".*notification"
```

### App Launcher (with walker)

```toml
[[CustomModule]]
name = "AppLauncher"
icon = "󱗼"
command = "walker"
```

### Clipboard (with cliphist)

```toml
[[CustomModule]]
name = "Clipboard"
icon = "📋"
command = "cliphist-rofi-img | wl-copy"
```

## Migration from Deprecated Modules

The `AppLauncher` and `Clipboard` modules have been deprecated in favor of custom modules.
To migrate from the deprecated modules:

### Previous App Launcher Configuration

```toml
# Old deprecated way
app_launcher_cmd = "walker"
```

### New Custom Module Configuration

```toml
# New recommended way
[[CustomModule]]
name = "AppLauncher"
icon = "�"
command = "walker"
```

### Previous Clipboard Configuration

```toml
# Old deprecated way
clipboard_cmd = "cliphist-rofi-img | wl-copy"
```

### New Custom Module Configuration

```toml
# New recommended way
[[CustomModule]]
name = "Clipboard"
icon = "󰅏"
command = "cliphist-rofi-img | wl-copy"
```

Then add the custom modules to your modules configuration:

```toml
[modules]
right = [ [ "Clock", "Privacy", "Settings", "AppLauncher", "Clipboard" ] ]
```
