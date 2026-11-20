# ashell-experimental

The experimental ashell: a bar described by its configuration, built with
[GUIdo](https://github.com/MalpenZibo/guido) on the components of
[styletto](https://github.com/MalpenZibo/styletto). One grammar gives both
today's ashell (islands along a bar, menus that drop down) and an
island that morphs into its panels, and every middle ground in between.

It is an experiment, rebuilt step by step after
[`docs/bar-model.md`](docs/bar-model.md): the layout, the islands, the
background, the panels and the details are in place, and the first module,
`tempo` (the time, the weather and the calendar), has its data; the other
zones and panels still show placeholders sized like the real thing.

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

The site's pictures of the bar are drawn the same way, at a fixed time with
sample weather, in both of the site's themes. They land in
`target/snapshots/site`, to copy into `website/static/img`:

```sh
cargo test -p ashell-experimental site_pictures -- --ignored --nocapture
```

- `src/config`: the configuration, parsed and resolved into one plain tree;
- `src/bar`: the bar, its islands, its panels and its details, and the items
  in its zones with their gestures;
- `src/modules`: what the items and the panels show, one module each, from
  the services of `ashell-services`;
- `i18n`: the translations, read through `ashell-i18n`. `t!()` checks its
  keys against `i18n/en-US/ashell_experimental.ftl`.

[`docs/bar-model.md`](docs/bar-model.md) is the model the bar follows, with
the proposals for the parts it has not settled yet.

## License

GPL-3.0-or-later.
