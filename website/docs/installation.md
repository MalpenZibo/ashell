---
sidebar_position: 2
---

# 🛠️ Installation

You can install Ashell using the following methods:

## Packages

:::info

I'll maintain only the Arch Linux package and the nix configuration inside the repository.

In case of a broken package, try building from source first.

:::

[![Packaging status](https://repology.org/badge/vertical-allrepos/ashell.svg)](https://repology.org/project/ashell/versions)

### Arch Linux

Install a tagged release from the arch packages:

`sudo pacman -S hyprland`

or install from the AUR, which compiles the latest source:

`yay -S hyprland-git`

### Nix

To install ashell using the nix package be sure to enable flakes and then run

#### Tagged release

```
nix profile install github:MalpenZibo/ashell?ref=0.5.0
```

#### Main branch

```
nix profile install github:MalpenZibo/ashell
```

### NixOS/Home-Manager

To use this flake do

```nix
flake.nix
inputs = {
  # ... other inputs
  ashell.url = "github:MalpenZibo/ashell";
  # ... other inputs
};
outputs = {...} @ inputs: {<outputs>}; # Make sure to pass inputs to your specialArgs!
```

```nix
configuration.nix
{ pkgs, inputs, ... }:

{
  environment.systemPackages = [inputs.ashell.defaultPackage.${pkgs.system}];
  # or home.packages = ...
}
```

This will build ashell from source, but you can also use `pkgs.ashell` from nixpkgs which is cached.

## Building from source

To build Ashell from source, you need to have the following dependencies installed:

- Rust (with `cargo`)
- wayland-protocols
- clang
- libxkbcommon
- wayland
- dbus
- libpipewire
- libpulse

Then from the root of the repository run:

```bash
cargo build --release

# to install it system-wide
sudo cp target/release/ashell /usr/bin
```
