# Environment Variables

## Compositor Detection

| Variable | Checked By | Purpose |
|----------|-----------|---------|
| `HYPRLAND_INSTANCE_SIGNATURE` | `services/compositor/hyprland.rs` | Detects Hyprland compositor |
| `NIRI_SOCKET` | `services/compositor/niri.rs` | Detects Niri compositor |

ashell checks these in order. The first one found determines the compositor backend.

## Config Path

| Variable | Purpose |
|----------|---------|
| `XDG_CONFIG_HOME` | Base directory for config. Default config path is `$XDG_CONFIG_HOME/ashell/config.toml` (or `~/.config/ashell/config.toml` if unset) |

The config path can also be overridden with the `--config-path` CLI flag, which takes precedence over environment variables.

## Graphics

| Variable | Purpose |
|----------|---------|
| `WGPU_BACKEND` | Force a specific GPU backend. Set to `gl` for OpenGL (useful for NVIDIA compatibility) |

## Logging

ashell uses [flexi_logger](https://docs.rs/flexi_logger) which reads the log level from the config file's `log_level` field. The format follows [env_logger syntax](https://docs.rs/env_logger/latest/env_logger/#enabling-logging):

```toml
# In config.toml
log_level = "debug"
log_level = "warn,ashell::services=debug"
log_level = "info,ashell::modules::settings=trace"
```

## Wayland

| Variable | Purpose |
|----------|---------|
| `WAYLAND_DISPLAY` | The Wayland display socket. Must be set for ashell to run |
| `LD_LIBRARY_PATH` | May need to include paths to Wayland/Vulkan/Mesa libraries (handled automatically by Nix wrapper) |
