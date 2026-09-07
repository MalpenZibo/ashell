---
sidebar_position: 13
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
where `ashell` is displayed will also contain the workspaces
from the other monitor.

### MonitorSpecificExclusive

This mode displays only the workspaces associated with the monitor
where `ashell` is displayed.
If `ashell` is not displayed on a specific monitor the workspaces for that monitor
will not be shown.

## Grouping Workspaces by Monitor

You can configure whether workspaces should be grouped by monitor using the `group_by_monitor` option.

When enabled, workspaces are sorted by monitor first, then by their index within each monitor.
This is particularly useful in multi-monitor setups where you want to keep workspaces from the same monitor together.

By default, `group_by_monitor` is `false`, which means workspaces are displayed in their natural order.

```toml
[workspaces]
group_by_monitor = true
```

## Showing special workspaces

If you would like to make the special workspaces invisible, set the `disable_special_workspaces` to `true`.
By default, special workspaces will be visible.

Special workspaces are currently a Hyprland-only concept. On other compositors
there are none to show, so this option has no effect.

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
This lets you display alternative numerals (e.g., Roman numerals, Chinese numerals)
instead of typical Arabic numerals.  
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
workspace_names = ["一","二","三","四","五","六","七","八","九","十"]
```

## Virtual Desktop Plugin Support

If you are using the Hyprland plugin [hyprland-virtual-desktops](https://github.com/levnikmyskin/hyprland-virtual-desktops)
you probably want the workspaces to be grouped together instead
of showing up individually. Use the configuration option
`enable_virtual_desktops` to handle a virtual desktop as a single
workspace. Custom workspace colors and names will be applied to
the virtual desktop number instead of the workspace when this is
enabled.

```toml
[workspaces]
enable_virtual_desktops = true
```

## Urgent Workspace Highlighting

When a window on a non-focused workspace requests attention, that workspace is
highlighted with the danger palette color until you visit it. This works on
Niri and on generic Wayland compositors that report the workspace urgent state
(`ext-workspace-v1`), as well as MangoWC. Hyprland does not report urgent states
yet, so the highlight never triggers there.

## Workspace Scrolling Direction

Scrolling over the workspace indicator switches workspace. By default, scrolling
up goes to the *previous* workspace and scrolling down to the *next* one, for
both mice and trackpads.

Use the `invert_scroll_direction` field to invert that, independently per device
kind (ashell distinguishes them by the kind of scroll event the compositor
sends: discrete steps for a wheel, smooth deltas for a trackpad):

- `All`: Inverts both the mouse wheel and the trackpad.
- `Mouse`: Inverts the mouse wheel only. Scrolling up goes to the next workspace and down to the previous one.
- `Trackpad`: Inverts the trackpad only. Swiping up goes to the next workspace and down to the previous one.

Scrolling is not available on the generic Wayland backend, which can activate a
workspace but not step to the neighbouring one.

## Window Icons

By default the workspace indicator shows only the workspace name or number
(`indicator_format = "Name"`). Set `indicator_format = "NameAndIcons"` to also
show an icon for each application open in the workspace, similar to waybar's
window-rewrite feature.

Icons are resolved from each window's application class through the standard
XDG icon lookup (icon theme plus `.desktop` files). When a class cannot be
resolved a fallback glyph is shown instead. Window class collection is only
performed while this option is enabled, so the default configuration adds no
extra cost.

```toml
[workspaces]
indicator_format = "NameAndIcons"
```

:::note

Window class collection is currently implemented for Hyprland and Niri. On other
compositors the indicator falls back to showing the name only.

The icon index is built once at startup. Newly installed applications or a
changed icon theme are picked up the next time `ashell` is restarted.

:::

## Default Configuration

The default configuration is:

```toml
[workspaces]
visibility_mode = "All"
indicator_format = "Name"
group_by_monitor = false
enable_workspace_filling = false
disable_special_workspaces = false
enable_virtual_desktops = false
workspace_names = []
# max_workspaces is unset by default: omit the key to disable the limit
# invert_scroll_direction is unset by default: omit the key to keep the
# default scroll direction
```

:::warning
TOML has no `None` literal. Leave optional keys such as `max_workspaces` and
`invert_scroll_direction` out of the file to keep their defaults. Writing
`invert_scroll_direction = None` is a parse error that makes ashell fall back
to the *entire* default config.
:::

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
