---
sidebar_position: 4
---

# 🛠️ Installation

You can install Ashell using the following methods:

## Packages

:::info

Officially maintained: Arch Linux package and the Nix configuration
included in the repository.

Community packaging: Fedora via Copr and Gentoo Linux via GURU (see below). If a package is broken,
try building from source first.

:::

[![Packaging status](https://repology.org/badge/vertical-allrepos/ashell.svg)](https://repology.org/project/ashell/versions)

### Arch Linux

Install a tagged release from the AUR repositories:

```bash
yay -S ashell
```

Or install from the AUR, which compiles the latest source:

```bash
yay -S ashell-git
```

### Nix

Ashell is available in nixpkgs:

- **nixpkgs unstable**: version 0.8.0
- **nixpkgs 25.11 (stable)**: version 0.6.0

#### nix profile (flake)

To install directly from the repository using flakes:

```bash
nix profile install github:MalpenZibo/ashell
```

#### nixpkgs (cached, no build required)

```bash
# Unstable
nix profile install nixpkgs#ashell

# Or with nix-shell for a temporary session
nix shell nixpkgs#ashell
```

### NixOS / Home Manager

Using the flake directly:

```nix
inputs = {
  # ... other inputs
  ashell.url = "github:MalpenZibo/ashell";
  # ... other inputs
};
outputs = {...} @ inputs: {<outputs>}; # Make sure to pass inputs to your specialArgs!
```

And in your `configuration.nix`:

```nix
{ pkgs, inputs, ... }:

{
  environment.systemPackages = [inputs.ashell.packages.${pkgs.system}.default];
  # or home.packages = ...
}
```

Or using nixpkgs (cached, recommended):

```nix
{ pkgs, ... }:

{
  environment.systemPackages = [pkgs.ashell];
  # or home.packages = ...
}
```

### Fedora (Copr)

Unofficial Copr repository (maintained by @killcrb):

```bash
sudo dnf -y copr enable killcrb/ashell
sudo dnf -y install ashell
```

### Gentoo Linux (GURU)

First, [add the GURU repository](https://wiki.gentoo.org/wiki/Project:GURU/Information_for_End_Users#Adding_the_GURU_repository)

Next, unmask ashell package in any file in ```/etc/portage/package.accept_keywords/```

```bash
gui-apps/ashell ~amd64
```

And finally install:

```bash
emerge gui-apps/ashell
```

## Building from Source

To build Ashell from source, ensure the following dependencies are installed.

### Build dependencies

- Rust with `cargo`: minimum supported version **1.89**
- `pkg-config`
- `clang` / `llvm`: required by `bindgen` to generate the `libpulse`,
  `libpipewire`, and `libudev` bindings
- development headers for `libxkbcommon`, `wayland`, `libudev`, `libpipewire`,
  and `libpulse`
- `git` (optional): the build script embeds the current commit hash into
  `ashell --version`, falling back to `unknown` when `git` is unavailable

### Runtime dependencies

- `libxkbcommon`
- `wayland`
- `libudev`
- `libpipewire`
- `libpulse`
- a running D-Bus session bus
- a working graphics stack for `wgpu`: a Vulkan loader, or Mesa/libGL when
  falling back to the OpenGL backend

Package names differ between distributions, and the development packages
usually pull in their matching runtime libraries. See
[Troubleshooting](./configuration/troubleshooting.md) if the bar builds but
fails to render.

### Building

From the root of the repository, run:

```bash
cargo build --release

# To install it system-wide
sudo cp target/release/ashell /usr/local/bin/ashell
```
