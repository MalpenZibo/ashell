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

## First start

With no configuration yet, a welcome window opens over the screen, in two
steps:

1. Choose the bar to start from, and the edge of the screen it sits on:
   today's ashell, three islands along the edge; a solid bar from side to
   side; one island as long as the bar; or one island in the centre that
   turns into its panels. They are four of the
   [examples](@/docs/rolling/examples.md).
2. Make it yours: the theme and its mode, the density, the shape, the
   material, the motion and the fonts.

Each choice is written at once and shown on the bar as you make it. The
bar goes to `ashell-experimental.toml`, the look to `appearance.toml`
beside it, which the configuration imports. The settings panel, which the
`status` zone opens, has a button that opens the same two pages again, as
tabs, the look first.
Choosing another layout keeps the edge and the look. A configuration
written by hand is never replaced by a layout, but its `[bar.main]` can
still be moved to the other edge. Closing the window before choosing
anything writes nothing: it opens again at the next start.

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
