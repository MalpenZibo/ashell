---
sidebar_position: 1
---

# 🚪 Main

This page contains the base configuration options for Ashell.

It allows you to configure things like the log level, the monitor(s) used to
render the status bar, and the bar’s position.

All these configurations are defined in the root of the `toml` file.

## Log Level

The log level controls the verbosity of logs.

You can set it to a general level like `debug`, `info`, `warn`, or `error`,
or specify fine-grained control to enable logs from specific modules
in the codebase, e.g., `ashell::services::network=debug`.

See more about [log levels](https://docs.rs/env_logger/latest/env_logger/#enabling-logging).

:::warning

This configuration **requires** restarting Ashell to take effect.

:::

### Log Examples

Set the global log level to `debug` for all modules:

```toml
log_level = "debug"
```

Set the log level for the `ashell` module only:

```toml
log_level = "ashell=debug"
```

Set the log level to `warn` for all modules, `info` for Ashell modules,
and `debug` only for the network service:

```toml
log_level = "warn,ashell=info,ashell::services::network=debug"
```

To understand all possible module names you can use, check
the [source code](https://github.com/MalpenZibo/ashell).  
The `src` folder is the root of the `ashell` module, and every directory
or file under it declares a module or submodule.

For example, the file `src/modules/media_player.rs` maps to the module `ashell::modules::media_player`.

:::warning

Don’t confuse Ashell features (called “modules”) with Rust modules
(defined with `mod.rs` or in files).  
In this configuration, we're referring to Rust modules.

:::

## Outputs

You can configure which monitor(s) should display the status bar.

It can render on all monitors, only on the active one
(the focused monitor when Ashell starts), or on a list of specified monitors.

### Output Examples

Render the status bar on all monitors:

```toml
outputs = "All"
```

Render the status bar on the active monitor:

```toml
outputs = "Active"
```

Render the status bar on a specific list of monitors:

```toml
outputs = { Targets = ["DP-1", "eDP-1"] }
```

## Position

You can set the position of the status bar to either `Top` or `Bottom`.

### Position Examples

Set the bar position to the top:

```toml
position = "Top"
```

Set the bar position to the bottom:

```toml
position = "Bottom"
```

## Close menu with esc

You can enable the use of the `Esc` key to close the menu.

:::warning

With these features enabled ashell will use the keyboard
in an exclusive way when a menu is open.

That means other applications will not be able to use
the keyboard when the menu is open.

In the future, there will be a solution to avoid this problem.

:::

```toml
`enable_esc_key = true
```
