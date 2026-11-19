+++
title = "Installation"
weight = 1
+++

There are no packages: ashell-experimental is built from source, from the
`experimental` folder of the ashell repository. That folder is a Cargo
workspace of its own, kept out of ashell's.

## Requirements

- A Wayland compositor with the layer-shell protocol.
- Rust 1.97 or newer.
- The build dependencies of the stable ashell: see its
  [installation guide](https://malpenzibo.github.io/ashell/docs/installation).

## Build and run

```bash
git clone https://github.com/MalpenZibo/ashell
cd ashell/experimental
cargo run --release -p ashell-experimental
```

The binary is `ashell-experimental`, in `experimental/target/release/`. It
draws its bars beside anything else on the screen, the stable ashell
included: stop that one first if both are on the same edge.

## Start it with the session

From your compositor's autostart. For Hyprland:

```ini
exec-once = /path/to/ashell/experimental/target/release/ashell-experimental
```

For Niri:

```kdl
spawn-at-startup "/path/to/ashell/experimental/target/release/ashell-experimental"
```

## Logs

Warnings and errors go to standard error, problems in the configuration
included. Set `RUST_LOG` for more, as each reload of the configuration:

```bash
RUST_LOG=info ashell-experimental
```

## Update

It follows `main`, so updating is pulling and building again:

```bash
cd ashell
git pull
cd experimental
cargo build --release -p ashell-experimental
```
