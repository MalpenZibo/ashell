# Prerequisites

## System Requirements

- **Linux** with a Wayland session
- **Compositor**: [Hyprland](https://hyprland.org/) or [Niri](https://github.com/YaLTeR/niri)

## Rust Toolchain

ashell requires **Rust 1.89+** (edition 2024). Install via [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Build Dependencies

The following system libraries are required to compile ashell:

| Package | Purpose |
|---------|---------|
| `pkg-config` | Library discovery |
| `llvm-dev` | LLVM development files |
| `libclang-dev` / `clang` | Clang for bindgen (PipeWire/PulseAudio bindings) |
| `libxkbcommon-dev` | Keyboard handling |
| `libwayland-dev` | Wayland client protocol |
| `libpipewire-0.3-dev` | PipeWire audio integration |
| `libpulse-dev` | PulseAudio integration |
| `libudev-dev` | Device monitoring |
| `dbus` | D-Bus daemon and development files |

### Ubuntu / Debian

```bash
sudo apt-get install -y pkg-config llvm-dev libclang-dev clang \
  libxkbcommon-dev libwayland-dev dbus libpipewire-0.3-dev \
  libpulse-dev libudev-dev
```

### Fedora

```bash
sudo dnf install -y pkg-config llvm-devel clang-devel clang \
  libxkbcommon-devel wayland-devel dbus-devel pipewire-devel \
  pulseaudio-libs-devel systemd-devel
```

### Arch Linux

```bash
sudo pacman -S pkg-config llvm clang libxkbcommon wayland \
  dbus pipewire libpulse systemd-libs
```

### Nix (Alternative)

If you use Nix, you can skip all of the above. The project's `flake.nix` provides a complete development shell with all dependencies:

```bash
nix develop
```

See [Development Environment](development-environment.md) for details.

## Runtime Dependencies

At runtime, ashell needs:

- Wayland client libraries (`libwayland-client`)
- D-Bus
- `libxkbcommon`
- PipeWire libraries (`libpipewire-0.3`)
- PulseAudio libraries (`libpulse`)
- A running Hyprland or Niri compositor
