---
sidebar_position: 7
---

# Keyboard

There are two keyboard modules available in the status bar.

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

On MangoWC this reflects the current keybind mode (keymode).
