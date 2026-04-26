# Development Environment

## Nix Development Shell (Recommended)

The easiest way to get a fully working development environment is with Nix:

```bash
nix develop
```

This provides:

- The correct Rust toolchain (stable, latest)
- All system library dependencies (Wayland, PipeWire, PulseAudio, etc.)
- `rust-analyzer` for editor integration
- Correct `LD_LIBRARY_PATH` for runtime libraries (Wayland, Vulkan, Mesa, OpenGL)
- `RUST_SRC_PATH` set for rust-analyzer goto-definition

You can then build and run normally:

```bash
cargo build --release
./target/release/ashell
```

## Manual Setup

If you don't use Nix, install the [prerequisites](prerequisites.md) for your distribution and ensure Rust 1.89+ is installed.

## Editor Setup

### rust-analyzer

ashell uses [iced_layershell](https://github.com/MalpenZibo/iced_layershell) as a git dependency (see [Architecture Overview](../architecture/overview.md)). `rust-analyzer` works normally — go-to-definition will navigate into `~/.cargo/git/`.

If you need to develop against a local checkout of iced_layershell, add a `[patch]` section to `Cargo.toml` (don't commit this):

```toml
[patch."https://github.com/MalpenZibo/iced_layershell"]
iced_layershell = { path = "../iced_layershell" }
```

## Running ashell

### Standard Run

```bash
make start
# or
cargo run --release
```

ashell must be launched within a Wayland session running Hyprland or Niri. It cannot run under X11 or without a compositor.

### Custom Config Path

```bash
ashell --config-path /path/to/my/config.toml
```

The default config path is `~/.config/ashell/config.toml`.

## Logging

ashell uses [flexi_logger](https://docs.rs/flexi_logger) and writes logs to `/tmp/ashell/`.

- Log files rotate daily and are kept for 7 days.
- In debug builds, logs are also printed to stdout.
- The log level is controlled by the `log_level` field in the config file (default: `"warn"`).
- The log level follows the [env_logger syntax](https://docs.rs/env_logger/latest/env_logger/#enabling-logging), e.g., `"debug"`, `"info"`, `"ashell=debug,iced=warn"`.

To watch logs in real time:

```bash
tail -f /tmp/ashell/*.log
```

## IPC Socket

ashell listens on a Unix domain socket at `$XDG_RUNTIME_DIR/ashell.sock`.
The same binary acts as a client when invoked with the `msg` subcommand:

```bash
ashell msg toggle-visibility
ashell msg volume-up
ashell msg volume-down
ashell msg volume-toggle-mute
ashell msg microphone-up
ashell msg microphone-down
ashell msg microphone-toggle-mute
ashell msg brightness-up
ashell msg brightness-down
ashell msg toggle-airplane-mode
ashell msg toggle-idle-inhibitor
```

Volume, microphone, brightness, airplane and idle inhibitor commands show an OSD overlay. Add `--no-osd` to suppress it.

## Signal Handling

- **SIGUSR1**: Toggles bar visibility (legacy, still supported):
  ```bash
  kill -USR1 $(pidof ashell)
  ```
