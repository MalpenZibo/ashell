---
sidebar_position: 3
---

# Keyboard

There are three keyboard modules available in the status bar.

## Keyboard Layout

The Keyboard Layout module displays the current keyboard layout and allows
switching between layouts by clicking on the module.

It is available on Hyprland, Niri and MangoWC. The generic Wayland backend
cannot report the active layout, so the module renders nothing there.

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

This module displays the current keyboard submap in use. It only appears when a
submap is active, and it is available on Hyprland and MangoWC only (on MangoWC
the equivalent concept is the *key mode*, and the `default` mode counts as no
submap). You can find more information
about submap in the [Hyprland documentation](https://wiki.hypr.land/Configuring/Binds/#submaps) or the [MangoWC documentation](https://mangowm.github.io/docs/bindings/keys#key-modes-submaps).

## Keyboard Locks

This module shows Caps Lock, Num Lock and Scroll Lock. It works on every
compositor and needs no permissions: it reads the keyboards' lock LEDs, which
any user can read. A lock no connected keyboard has a LED for is never shown.

The LEDs are read four times a second while the keyboard or the mouse are in
use, and not at all once they have been left alone for a few seconds, e.g. on
the lock screen: a lock changes when a key is pressed, so a change shows up at
most a quarter of a second later.
On a compositor without the `ext-idle-notify-v1` protocol they are read four
times a second all the time.

Each lock has three options:

- `enabled` (default `true`): whether the lock is shown at all.
- `visibility`: when the lock is shown.
  - `ActiveOnly` (default): only while it is on.
  - `AlwaysVisible`: always, dimmed while it is off.
- `icon`: a glyph or a text to show instead of the default icon, e.g. `"NUM"`.

By default the module only shows the locks that are on.

### Example

```toml
[keyboard_locks.caps_lock]
visibility = "AlwaysVisible"

[keyboard_locks.num_lock]
icon = "NUM"

[keyboard_locks.scroll_lock]
enabled = false
```
