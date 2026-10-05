# Environment Variables

## Compositor Detection

| Variable | Checked By | Purpose |
|----------|-----------|---------|
| `HYPRLAND_INSTANCE_SIGNATURE` | `services/compositor/hyprland.rs` | Detects Hyprland compositor |
| `NIRI_SOCKET` (or `NIRI_SOCKET_PATH`) | `services/compositor/niri.rs` | Detects Niri compositor |
| `WAYLAND_DISPLAY` | `services/compositor/generic.rs` | Fallback generic Wayland backend |

ashell checks these in order (Hyprland, Niri, MangoWC, then the generic Wayland backend). The first one found determines the compositor backend. MangoWC is detected through its `mmsg` IPC rather than an environment variable.

## Config Path

| Variable | Purpose |
|----------|---------|
| `HOME` | The default config path is `~/.config/ashell/config.toml` (`XDG_CONFIG_HOME` is not consulted) |

The config path can be overridden with the `--config-path` CLI flag. The path is expanded with `~` and `$VAR` support.

## Runtime Directory

| Variable | Purpose |
|----------|---------|
| `XDG_RUNTIME_DIR` | Location of the IPC socket (`ashell.sock`) and the default log directory (logs fall back to `/tmp/ashell` if unset) |

## Graphics

| Variable | Purpose |
|----------|---------|
| `WGPU_BACKEND` | Force a specific GPU backend. Set to `gl` for OpenGL (useful for NVIDIA compatibility) |

## Logging

ashell uses [flexi_logger](https://docs.rs/flexi_logger) which reads the log level from the config file's `logging.level` field. The format follows [env_logger syntax](https://docs.rs/env_logger/latest/env_logger/#enabling-logging):

```toml
# In config.toml
[logging]
level = "debug"
# level = "warn,ashell::services=debug"
# level = "info,ashell::modules::settings=trace"
```

`logging.target` selects the destination (`"file"`, the default, or `"stdout"` / `"stderr"`) and `logging.directory` overrides the log directory used by the `"file"` target.

## Wayland

| Variable | Purpose |
|----------|---------|
| `WAYLAND_DISPLAY` | The Wayland display socket. Must be set for ashell to run |
| `LD_LIBRARY_PATH` | May need to include paths to Wayland/Vulkan/Mesa libraries (handled automatically by Nix wrapper) |
