# Building from Source

## Quick Build

The simplest way to build ashell:

```bash
cargo build --release
```

The binary will be at `target/release/ashell`.

## Using the Makefile

The project includes a `Makefile` with convenience targets:

| Target | Command | Description |
|--------|---------|-------------|
| `make build` | `cargo build --release` | Build release binary |
| `make start` | Build + `./target/release/ashell` | Build and run |
| `make install` | Build + `sudo cp -f target/release/ashell /usr/bin` | Install to system |
| `make fmt` | `cargo fmt` | Format code |
| `make check` | `cargo fmt` + `cargo check` + `cargo clippy -- -D warnings` | Full lint check |

## What build.rs Does

The `build.rs` script runs at compile time and performs two tasks:

### 1. Font Subsetting

ashell bundles [Nerd Font](https://www.nerdfonts.com/) for icons. The full font files are ~4.8 MB. To reduce binary size, `build.rs` uses the [allsorts](https://github.com/AltSign/allsorts) crate to:

1. Parse `src/components/icons.rs` to find all Unicode codepoints in use (e.g., `\u{f0e7}`)
2. Subset the Nerd Font TTF files to only include those glyphs
3. Write the optimized fonts to `target/generated/`

This means adding a new icon to `icons.rs` automatically includes it in the subset on the next build.

### 2. Git Hash Extraction

`build.rs` runs `git rev-parse --short HEAD` and embeds the result as the `GIT_HASH` environment variable. This is used in the `--version` output:

```
ashell 0.7.0 (abc1234)
```

## Release Profile

The release build profile in `Cargo.toml` is optimized for production:

```toml
[profile.release]
lto = "thin"       # Thin Link-Time Optimization
strip = true       # Strip debug symbols
opt-level = 3      # Maximum optimization
panic = "abort"    # Abort on panic (smaller binary, no unwinding)
```

## Common Build Issues

- **Missing system libraries**: If you get `pkg-config` errors, ensure all [prerequisites](prerequisites.md) are installed.
- **Font subsetting failure**: The `target/generated/` directory is created automatically by `build.rs`. If the build fails on font subsetting, ensure `assets/SymbolsNerdFont-Regular.ttf` exists.
- **Slow first build**: The first build compiles all dependencies including iced (which is large). Subsequent builds are incremental and much faster.
