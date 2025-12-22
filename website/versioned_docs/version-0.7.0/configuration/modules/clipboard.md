---
sidebar_position: 2
---

# Clipboard

:::warning

This module will be deprecated in future releases.

:::

Provides a way to open your clipboard manager from the status bar.

To configure this module, you need to specify a command that will
start your clipboard manager when the module is clicked.

:::info

Without this configuration, the module will not appear in the status bar.

:::

## Example

In this example, I use [cliphist](https://github.com/sentriz/cliphist)
as my clipboard manager.

```toml
clipboard_cmd = "cliphist-rofi-img | wl-copy"
```
