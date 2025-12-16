---
sidebar_position: 2
---

# 🛠️ Installation

You can install Ashell using the following methods:

## Packages

:::info

Officially maintained: Arch Linux package and the Nix configuration
included in the repository.

Community packaging: Fedora via Copr (see below). If a package is broken,
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

To install Ashell using the Nix package manager, make sure flakes are enabled,
then run:

#### Tagged Release

```bash
nix profile install github:MalpenZibo/ashell?ref=0.5.0
```

#### Main Branch

```bash
nix profile install github:MalpenZibo/ashell
```

### NixOS / Home Manager

To use this flake, add the input to your `flake.nix`:

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

This will build Ashell from source.  
Alternatively, you can use `pkgs.ashell` from `nixpkgs`, which is cached.

### Fedora (Copr)

Unofficial Copr repository (maintained by @killcrb):

```bash
sudo dnf -y copr enable killcrb/ashell
sudo dnf -y install ashell
```

## Building from Source

To build Ashell from source, ensure the following dependencies are installed:

- Rust (with `cargo`)
- wayland-protocols
- clang
- libxkbcommon
- wayland
- dbus
- libpipewire
- libpulse

Then, from the root of the repository, run:

```bash
cargo build --release

# To install it system-wide
sudo cp target/release/ashell /usr/bin
```
