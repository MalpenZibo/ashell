---
sidebar_position: 1
---

# 🚪 Main

This page contains the base configuration for Ashell.

It is used to change things like the log level, the monitor used to render the status bar or the bar position.

All these configuration are in the root structure of the `toml` file

## Log Level

The log level is used to control the verbosity of the logs.

It can be set with different values, from the most straightforward like a simple `debug|info|warn|error` to something more complex to enable only log from a specific module in the codebase like `ashell::services::network=debug`.

See more on the log levels [here](https://docs.rs/env_logger/latest/env_logger/#enabling-logging).

:::warning

This configuration require a restart of Ashell to take effect.

:::

### Example

**Set the global log level to debug for all modules**

```toml
log_level = "debug"
```

**Set the log level for ashell module**

```toml
log_level = "ashell=debug"
```

**Set the log level to warn for all modules, to info for ashell modules and to debug only for the network service**

```toml
log_level = "warn, ashell=info, ashell::services::network=debug"
```

To understand all the possible modules name you can use, you can check the [source code](https://github.com/MalpenZibo/ashell). The `src` folder il the ashell module and every directory or file under that folder declares a module or a sub modules.

For example the file: `src/modules/media_player.rs` is the `ashell::modules::media_player` module.

> **Note**: Do not confuse ashell modules with rust modules (the `mod.rs` files). The ashell modules are not related to the rust modules, they are just a way to group the features of Ashell. In this configuration we're talking about rust modules.

## Outputs

The outputs are used to configure the monitors used to render the status bar.

You can render the status bar on all monitors, on active one (the one with the focus when ashell starts)
or a list of monitors.

### Example

**Render the status bar on all monitors**

```toml
outputs = "All"
```

**Render the status bar on the active monitor**

```toml
outputs = "Active"
```

**Render the status bar on a list of monitors**

```toml
outputs = { Targets = ["DP-1", "eDP-1"] }
```

## Position

The position of the status bar can be set to either `Top` or `Bottom`.

### Example

**Set the bar position to top**

```toml
position = "Top"
```

**Set the bar position to bottom**

```toml
position = "Bottom"
```
