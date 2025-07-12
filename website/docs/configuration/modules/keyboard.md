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

### Example

In this example we're mapping the "English (US)" layout to the 🇺🇸 flag and
the "Italian" layout to the 🇮🇹 flag.

```toml
[keyboard_layout.labels]
"English (US)" = "🇺🇸"
"Italian" = "🇮🇹"
```

## Keyboard Submap

This module displays the current keyboard submap in use. You can find more information
about submap in the [Hyprland documentation](https://wiki.hyprland.org/Hyprland-Submaps/).
