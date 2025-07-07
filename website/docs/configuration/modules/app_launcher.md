---
sidebar_position: 1
---

# App Launcher

:::warning

This module will be deprecated in the futures releases

:::

Provides a way to launch applications from the status bar.

To configure this module you need to specify a command that will
start your launcher when the module is clicked.

:::info

Without this configuration the module will not appear in the status bar.

:::

## Example

In this example I use [Walker](https://github.com/abenz1267/walker)
as my application launcher.

```toml
app_launcher_cmd = "walker"
```
