---
sidebar_position: 2
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
- `command`: Command to execute when the module is clicked (for `button` type). Empty or whitespace-only values are treated as unset.
- `on_right_click` _(optional)_: Command to execute on right-click.
- `on_middle_click` _(optional)_: Command to execute on middle-click.
- `on_scroll_up` _(optional)_: Command to execute when scrolling up over the module.
- `on_scroll_down` _(optional)_: Command to execute when scrolling down over the module.
- `listen_cmd` _(optional)_: Command to run in the background to update the module's display. Empty or whitespace-only values are treated as unset.
- `icons` _(optional)_: Regex-to-icon mapping to change the icon based on the `listen_cmd` output (for `button` type). The first matching regex wins; since the mappings are stored as a map, the evaluation order is not guaranteed. Prefer mutually exclusive regexes or keep patterns precise to avoid ambiguous matches.
- `alert` _(optional)_: Regex to trigger a red alert dot on the icon when matched in the `listen_cmd` output (for `button` type).

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

:::tip JSON Output

Output compact single-line JSON:

```json
{"text": "3", "alt": "notification"}
```

If you have pretty-printed JSON and want to use it in a single line, pipe it through `jq` to compact it:

```bash
your-command | jq -c --unbuffered .
```

For example:

```bash
echo '{
  "text": "3",
  "alt": "notification"
}' | jq -c --unbuffered .
# Output: {"text":"3","alt":"notification"}
```

Or you can output pretty-printed multiline JSON directly, which is buffered until valid JSON is formed:

```json
{
  "text": "3",
  "alt": "notification"
}
```

:::warning Multiline JSON Buffer Limit

When using pretty-printed (multiline) JSON, output is accumulated in an internal buffer until a complete, parseable JSON object is received. As a safeguard against malformed output, for example a script that opens a `{` and never closes it, the buffer is capped at **1 MiB**. If that limit is exceeded, the buffered bytes are dropped, a warning is logged, and the buffer is cleared.

This limit only applies while buffering multiline JSON. Single-line (compact) JSON output is unaffected since each line is parsed independently.

:::

See the configuration examples below for multiline usage.

:::

### Example Output

```json
{"text": "3", "alt": "notification"}
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
listen_cmd = '''
while true; do
  cat <<EOF
{
  "text": "$(date +'%H:%M')",
  "alt": ""
}
EOF
  sleep 1
done
'''
```

### Button Module with Icon (Interactive)

Button modules display an icon and/or text with a click action. They also support right-click, middle-click, and scroll events:

```toml
[[CustomModule]]
name = "volume"
type = "Button"
icon = ""
command = "pactl get-sink-volume @DEFAULT_SINK@"
on_right_click = "pactl set-sink-mute @DEFAULT_SINK@ toggle"
on_middle_click = "pavucontrol"
on_scroll_up = "pactl set-sink-volume @DEFAULT_SINK@ +5%"
on_scroll_down = "pactl set-sink-volume @DEFAULT_SINK@ -5%"
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

Button modules can display text from a multiline JSON output from `listen_cmd` with a click action:

```toml
[[CustomModule]]
name = "Clipboard"
type = "Button"
icon = "📋"
command = "cliphist-rofi-img | wl-copy"
listen_cmd = '''printf '%s\n' '{
  "text": "Clipboard content",
  "alt": ""
}'
'''
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
right = [ [ "Tempo", "Privacy", "Settings", "AppLauncher", "Clipboard" ] ]
```
