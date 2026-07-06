# Nix Flake

ashell provides a `flake.nix` for reproducible builds and development environments.

## Development Shell

```bash
nix develop
```

This provides:
- Latest stable Rust toolchain via `rust-overlay`
- `rust-analyzer` for editor integration
- All native build dependencies (Wayland, PipeWire, PulseAudio, etc.)
- Correct `LD_LIBRARY_PATH` for runtime libraries
- `RUST_SRC_PATH` set for rust-analyzer

## Building with Nix

```bash
nix build
```

The built binary includes a wrapper that sets `LD_LIBRARY_PATH` for runtime dependencies (Wayland, Vulkan, Mesa, OpenGL).

## Flake Inputs

| Input | Purpose |
|-------|---------|
| `crane` | Rust build system for Nix |
| `nixpkgs` | Package repository (nixos-unstable channel) |
| `rust-overlay` | Rust toolchain overlay |

## Build Dependencies

```nix
buildInputs = [
  rustToolchain.default
  rustPlatform.bindgenHook   # For C library bindings
  pkg-config
  libxkbcommon
  libGL
  pipewire
  libpulseaudio
  wayland
  vulkan-loader
  udev
];
```

## Runtime Dependencies

```nix
runtimeDependencies = [
  libpulseaudio
  wayland
  mesa
  vulkan-loader
  libGL
  libglvnd
];
```

The `postInstall` step wraps the binary with `wrapProgram` to set `LD_LIBRARY_PATH`:

```nix
postInstall = ''
  wrapProgram "$out/bin/ashell" --prefix LD_LIBRARY_PATH : "${ldLibraryPath}"
'';
```
