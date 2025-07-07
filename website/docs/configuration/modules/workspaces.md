---
sidebar_position: 4
---

# Workspaces

This module provides information about the current workspaces and allows switching between them.

You can switch between two main visibility modes:

- `All`: All the workspaces will be displayed.
- `MonitorSpecific`: Only the workspaces of the related monitor will be displayed.

You can also enable/disable the filling of the workspace list with empty workspaces using the `enable_workspace_filling`.

The default configuration is:

```toml
[workspaces]
visibility_mode = "All"
enable_workspace_filling = true
```

If you want specific number of empty workspaces always displayed, you can use the `max_workspaces` option. This settings only works if `enable_workspace_filling` is set to `true`.

Usually `enable_workspace_filling` will create empty workspaces up to the greatest workspace in use.
For example, if you have a window open in workspace 1 and another one in workspace 5, ashell will
create empty workspaces 2, 3, and 4 to fill the gap.

With `max_workspaces` set to 10, ashell will create also the workspaces 6, 7, 8, 9 and 10.

By default `max_workspaces` is None, which disable this feature.

## Examples

If we want to disable the workspace filling and set the visibility mode to "MonitorSpecific", we can do it like this:

```toml
[workspaces]
visibility_mode = "MonitorSpecific"
enable_workspace_filling = false
```

If we want to set the maximum number of workspaces to 10, we can do it like this:

```toml
[workspaces]
enable_workspace_filling = true
max_workspaces = 10
```
