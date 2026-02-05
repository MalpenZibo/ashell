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
- `icon`: Icon displayed in the status bar.
- `command`: Command to execute when the module is clicked.
- `listen_cmd` _(optional)_: Command to run in the background once on startup.
  The command should continuously output JSON to update the module's display.
  The most recent output is used to update the module's display.
- `icons` _(optional)_: Regex-to-icon mapping to change the icon based on
  the `listen_cmd` output.
- `alert` _(optional)_: Regex to trigger a red alert dot on the icon when
  matched in the `listen_cmd` output.

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

### Notifications (with swaync-client)

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

## Best Practices

### Writing Effective `listen_cmd` Scripts

#### Use Event-Driven Approaches

Instead of polling, prefer event-driven approaches:

```bash
# Bad: Polling every second
while true; do
    check_updates
    sleep 1
done

# Good: Event-driven
inotifywait -m /path/to/monitor | while read; do
    check_updates
done
```

#### Output Valid JSON

Always ensure your output is valid JSON:

```bash
# Good: Proper JSON escaping
echo "{\"text\": \"$message\", \"alt\": \"$status\"}"

# Bad: Unescaped quotes
echo '{"text": "Hello "world", "alt": "normal"}'
```

### Performance Considerations

- **Avoid frequent updates**: Only output when something actually changes
- **Use efficient tools**: Prefer lightweight tools over heavy applications
- **Minimize dependencies**: Use system tools when possible
- **Handle errors gracefully**: Ensure your script doesn't crash the module

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
