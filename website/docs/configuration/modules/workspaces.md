---
sidebar_position: 4
---

# Workspaces

This module provides information about the current workspaces  
and allows switching between them.

You can switch between two main visibility modes:

- `All`: All workspaces will be displayed.
- `MonitorSpecific`: Only the workspaces of the related monitor will be displayed.

You can also enable or disable filling the workspace  
list with empty workspaces using the `enable_workspace_filling` option.

The default configuration is:

```toml
[workspaces]
visibility_mode = "All"
enable_workspace_filling = true
```

If you want a specific number of empty workspaces always displayed,  
you can use the `max_workspaces` option. This setting only works  
if `enable_workspace_filling` is set to `true`.

Usually, `enable_workspace_filling` will create empty workspaces  
up to the greatest workspace in use.  
For example, if you have a window open in workspace 1 and  
another one in workspace 5, ashell will create empty  
workspaces 2, 3, and 4 to fill the gap.

With `max_workspaces` set to 10, ashell will also create  
workspaces 6, 7, 8, 9, and 10.

By default, `max_workspaces` is None, which disables this feature.

## Examples

If you want to disable workspace filling and set the visibility mode  
to "MonitorSpecific", you can do it like this:

```toml
[workspaces]
visibility_mode = "MonitorSpecific"
enable_workspace_filling = false
```

If you want to set the maximum number of workspaces to 10, you can do it like this:

```toml
[workspaces]
enable_workspace_filling = true
max_workspaces = 10
```

You can also assign **custom names** to your workspaces using
the `workspace_names` option.  
This lets you display alternative numerals (e.g., roman numerals, chinese numerals)
instead of typical arabic numerals.  
If a name is missing for a given workspace index, the numeric ID will be used
as a fallback.

:::warning

Ashell comes with a set of default icons that are used internally.

If you decide to use a font icon in your workspace names configuration remember
to install the font with that icon on your system.

For example you can use [Nerd Fonts](https://www.nerdfonts.com/)

:::

```toml
[workspaces]
workspace_names = ["一","二","三","四","五","六","七","八","九","十",]

```
