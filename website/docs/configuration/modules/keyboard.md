---
sidebar_position: 3
---

# Keyboard

There are three keyboard modules available in the status bar.

## Keyboard Layout

The Keyboard Layout module displays the current keyboard layout and allows
switching between layouts by clicking on the module.

You can add an optional configuration to map a keyboard layout label
to another label using the `labels` configuration.

:::warning

Ashell comes with a set of default icons that are used internally.

If you decide to use a font icon in your keyboard layout configuration remember
to install the font with that icon on your system.

For example you can use [Nerd Fonts](https://www.nerdfonts.com/)

:::

### Example

In this example we're mapping the "English (US)" layout to the 🇺🇸 flag and
the "Italian" layout to the 🇮🇹 flag.

```toml
[keyboard_layout.labels]
"English (US)" = "🇺🇸"
"Italian" = "🇮🇹"
```

## Keyboard Submap

This module displays the current keyboard submap in use. It only appears when a submap is active. You can find more information
about submap in the [Hyprland documentation](https://wiki.hypr.land/Configuring/Binds/#submaps).

## Keyboard Locks

This module displays the state of the keyboard lock keys: Caps Lock, Num Lock,
and Scroll Lock. State is read from `/dev/input/event*` via evdev, so the
module works on any compositor (Hyprland, Niri, …) without compositor-specific
IPC. Clicking an indicator toggles the corresponding lock.

:::info

Ashell needs access to two device nodes to provide the lock indicators:

- Read access to `/dev/input/event*` (typically mode `660 root:input`) to
  observe lock state.
- Write access to `/dev/uinput` (typically mode `660 root:input`) to toggle
  locks when an indicator is clicked.

Both nodes are owned by the `input` group on most distributions, so the user
running ashell just needs to be a member of that group:

```
sudo usermod -aG input "$USER"
```

Log out and back in for the new group to take effect.

If no input devices are readable, the module silently disables itself and a
warning is logged. If `/dev/uinput` cannot be opened, the display still works
but clicks become no-ops and a single warning is logged.

<details>
<summary>Manual permission setup</summary>

On systems where `/dev/uinput` or `/dev/input/event*` are not granted to the
`input` group out of the box, a udev rule can be used to align them:

```
# /etc/udev/rules.d/99-uinput.rules
KERNEL=="uinput", GROUP="input", MODE="0660"
```

Reload udev with `sudo udevadm control --reload-rules && sudo udevadm trigger`
for the rule to take effect.

</details>

:::

Each lock indicator can be configured independently with three options:

- `enabled` (`true` / `false`): whether the indicator is considered at all.
- `visibility`: when the indicator is rendered.
  - `ActiveOnly` (default): the indicator is only shown while the lock is on.
  - `AlwaysVisible`: the indicator is always shown; rendered dim when the lock
    is off and bright when it is on.
- `icon` (optional): a custom glyph or string to render instead of the default
  icon. When unset, ashell uses the bundled Nerd Font glyphs (no extra font
  needed): caps-lock, numeric and arrow-up-down icons for Caps Lock, Num Lock
  and Scroll Lock respectively. When set, the value is rendered through the
  same `DynamicIcon` path used by custom modules, so any Nerd Font glyph
  (e.g. `"\uF11C"`) or plain text (e.g. `"NUM"`) works.

By default all three indicators are enabled in `ActiveOnly` mode, so the
module stays out of sight until a lock is engaged.

### Example

```toml
[keyboard_locks.caps_lock]
enabled = true
visibility = "AlwaysVisible"

[keyboard_locks.num_lock]
enabled = true
visibility = "ActiveOnly"
icon = "NUM"

[keyboard_locks.scroll_lock]
enabled = false
```
