---
sidebar_position: 4
---

# Workspaces

This module provides information about the current workspaces  
and allows switching between them.

## Visibility Modes

You can configure how workspaces are displayed using the `visibility_mode` option.

### All

This mode displays all workspaces across all monitors. Workspaces from different
monitors are shown using different colors.

### MonitorSpecific

This mode displays only the workspaces associated with the monitor
where `ashell` is displayed.
If `ashell` is not displayed on a specific monitor, the monitor
where `ashell` is displayed will contain also the workspaces
from the other monitor.

### MonitorSpecificExclusive

This mode displays only the workspaces associated with the monitor
where `ashell` is displayed.
If `ashell` is not displayed on a specific monitor the workspaces for that monitor
will not be shown.

## Workspace Filling And Maximum Workspaces

You can also enable or disable filling the workspace  
list with empty workspaces using the `enable_workspace_filling` option.

:::warning

`enable_workspace_filling` will not work if the `visibility_mode`
is set to `MonitorSpecificExclusive`.

:::

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

## Custom Workspace Names

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

## Default Configuration

The default configuration is:

```toml
[workspaces]
visibility_mode = "All"
enable_workspace_filling = true
```

## Examples

If you want to disable workspace filling and set the visibility mode  
to `MonitorSpecific`, you can do it like this:

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
