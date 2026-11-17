# ashell-experimental

The experimental ashell: a bar described by its configuration, built with
[GUIdo](https://github.com/MalpenZibo/guido) on the components of
[styletto](https://github.com/MalpenZibo/styletto). One grammar gives both
today's ashell (islands along a bar, menus that drop down) and an
island that morphs into its panels, and every middle ground in between.

It is an experiment, rebuilt step by step after
[`docs/bar-model.md`](docs/bar-model.md): the layout, the islands, the
background, the panels and the details are in place, while zones and panels
still show placeholders sized like the real thing.

The user docs are on the [site](https://malpenzibo.github.io/ashell/experimental/),
from [`../website`](../website).

## Running

```sh
cd experimental
cargo run --release -p ashell-experimental
```

Logs go to stderr at `warn`; `RUST_LOG=info` shows more, including each
reload of the configuration.

## Configuration

`~/.config/ashell/ashell-experimental.toml`
(`$XDG_CONFIG_HOME/ashell/ashell-experimental.toml`), reloaded live. An
empty or missing file is a complete bar.

```toml
[bar.main]
shape   = true      # one island as long as the bar
padding = "s2"      # floating off the screen's edges
panels  = "merge"
hover   = true

start  = ["workspaces"]
center = [["window", "clock"]]
end    = [["privacy", "status"]]
```

[`docs/examples`](docs/examples) has complete files to start from, and the
site documents every key.

## Development

```sh
cargo test -p ashell-experimental
cargo clippy --workspace --all-targets -- -D warnings
```

The bar can be drawn offscreen, frame by frame, to look at a shape or an
animation without a screen; the PNGs land in `target/snapshots`:

```sh
cargo test -p ashell-experimental snapshots -- --ignored --nocapture
```

- `src/config`: the configuration, parsed and resolved into one plain tree;
- `src/bar`: the bar, its islands, its panels and its details.

[`docs/bar-model.md`](docs/bar-model.md) is the model the bar follows, with
the proposals for the parts it has not settled yet.

## License

GPL-3.0-or-later.
