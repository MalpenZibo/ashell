# Cargo and Dependencies

ashell's dependencies are managed in `Cargo.toml`. This chapter covers the key dependencies and why they're used.

## Core Dependencies

### UI Framework

| Crate | Version | Purpose |
|-------|---------|---------|
| `iced` (iced_layershell) | Git | GUI framework with Wayland layer shell support via [iced_layershell](https://github.com/MalpenZibo/iced_layershell) |

The dependency is aliased as `iced` in `Cargo.toml` (`package = "iced_layershell"`). It uses these features:
- `tokio` — Async runtime integration
- `advanced` — Custom widget support
- `wgpu` — GPU-accelerated rendering
- `image`, `svg`, `canvas` — Graphics capabilities

### Async Runtime

| Crate | Version | Purpose |
|-------|---------|---------|
| `tokio` | 1 | Async runtime for services |
| `tokio-stream` | 0.1 | Stream utilities |

### Compositor Integration

| Crate | Version | Purpose |
|-------|---------|---------|
| `hyprland` | 0.4.0-beta.2 | Hyprland IPC client |
| `niri-ipc` | 25.11.0 | Niri IPC client |

### System Integration

| Crate | Version | Purpose |
|-------|---------|---------|
| `zbus` | 5 | D-Bus client (BlueZ, NM, UPower, etc.) |
| `libpulse-binding` | 2.28 | PulseAudio client library |
| `pipewire` | 0.9 | PipeWire integration |
| `wayland-client` | 0.31.12 | Wayland protocol client |
| `wayland-protocols` | 0.32.10 | Wayland protocol definitions |
| `sysinfo` | 0.37 | CPU, RAM, disk, network statistics |
| `udev` | 0.9 | Device monitoring |

### Configuration

| Crate | Version | Purpose |
|-------|---------|---------|
| `toml` | 0.9 | TOML config file parsing |
| `serde` | 1.0 | Serialization/deserialization |
| `serde_json` | 1 | JSON for tray menu data |
| `serde_with` | 3.12 | Advanced serde derivation |
| `clap` | 4.5 | CLI argument parsing |
| `inotify` | 0.11.0 | File change watching |

### Utilities

| Crate | Version | Purpose |
|-------|---------|---------|
| `chrono` | 0.4 | Date/time handling |
| `chrono-tz` | 0.10.4 | Timezone support |
| `regex` | 1.12.2 | Regular expressions (config parsing) |
| `hex_color` | 3 | Hex color parsing in config |
| `itertools` | 0.14 | Iterator utilities |
| `anyhow` | 1 | Error handling |
| `log` | 0.4 | Logging facade |
| `flexi_logger` | 0.31 | Logging implementation |
| `signal-hook` | 0.4.3 | Unix signal handling (SIGUSR1) |
| `reqwest` | 0.13 | HTTP client (weather data) |
| `uuid` | 1 | UUID generation |
| `url` | 2.5.7 | URL parsing |
| `freedesktop-icons` | 0.4 | XDG icon lookup |
| `linicon-theme` | 1.2.0 | Icon theme resolution |
| `shellexpand` | 3 | Tilde/env var expansion in paths |
| `parking_lot` | 0.12.5 | Synchronization primitives |
| `pin-project-lite` | 0.2.16 | Pin projection (for throttle stream) |
| `libc` | 0.2.182 | System call interfaces |

## Build Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `allsorts` | 0.15 | Font parsing and subsetting in `build.rs` |

## Release Profile

```toml
[profile.release]
lto = "thin"       # Link-Time Optimization (balances speed vs compile time)
strip = true       # Remove debug symbols from binary
opt-level = 3      # Maximum optimization
panic = "abort"    # No stack unwinding (smaller binary)
```

## Runtime Package Dependencies

For binary distribution, runtime dependencies are declared in `Cargo.toml` metadata:

```toml
[package.metadata.nfpm]
provides = ["ashell"]
depends = ["libxkbcommon", "dbus"]

[package.metadata.nfpm.deb]
depends = ["libwayland-client0", "libpipewire-0.3-0t64", "libpulse0"]

[package.metadata.nfpm.rpm]
depends = ["libwayland-client", "pipewire-libs", "pulseaudio-libs"]
```
