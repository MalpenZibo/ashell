# experimental

Experimental work on ashell, built on [GUIdo](https://github.com/MalpenZibo/guido)
and [styletto](https://github.com/MalpenZibo/styletto). It is a Cargo
workspace of its own, out of ashell's: nothing here touches ashell's build,
lock file or CI.

- [`ashell-experimental`](ashell-experimental/README.md): the experimental ashell, a
  bar described by its configuration. A rolling, experimental release.
- [`website`](website/README.md): its site, built with Zola, served at
  <https://malpenzibo.github.io/ashell/experimental/>.

```sh
cd experimental
cargo run --release -p ashell-experimental
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
