# build.rs: Font Subsetting

The build script (`build.rs`) runs at compile time and performs two tasks: Nerd Font subsetting and git hash extraction.

## Font Subsetting

ashell uses [Nerd Font](https://www.nerdfonts.com/) symbols for icons (battery, WiFi, Bluetooth, volume, etc.). The full font files are ~4.8 MB. Since ashell only uses ~80 icons, the build script subsets the fonts to include only the needed glyphs.

### How It Works

1. **Parse icons**: Read `src/components/icons.rs` and find all `\u{XXXX}` Unicode escape sequences.

2. **Convert to characters**: Each hex code is converted to its Unicode character.

3. **Subset the font**: Using the [allsorts](https://docs.rs/allsorts) crate, create new TTF files containing only the needed glyphs.

4. **Write output**: Save the subsetted fonts to `target/generated/`:
   - `SymbolsNerdFont-Regular-Subset.ttf`
   - `SymbolsNerdFontMono-Regular-Subset.ttf`

### Source Files

| Input | Output |
|-------|--------|
| `assets/SymbolsNerdFont-Regular.ttf` (~2.4 MB) | `target/generated/SymbolsNerdFont-Regular-Subset.ttf` (~few KB) |
| `assets/SymbolsNerdFontMono-Regular.ttf` (~2.4 MB) | `target/generated/SymbolsNerdFontMono-Regular-Subset.ttf` (~few KB) |

### Adding a New Icon

To add a new icon to ashell:

1. Find the Unicode codepoint from the [Nerd Fonts cheat sheet](https://www.nerdfonts.com/cheat-sheet).
2. Add a constant to `src/components/icons.rs`:
   ```rust
   pub const MY_ICON: char = '\u{f0001}';
   ```
3. Build — `build.rs` automatically detects the new codepoint and includes it in the subset.

No manual font editing is required.

## Git Hash Extraction

The build script also extracts the current git commit hash:

```rust
let output = Command::new("git")
    .args(["rev-parse", "--short", "HEAD"])
    .output();

match output {
    Ok(output) if output.status.success() => {
        let git_hash = String::from_utf8(output.stdout)?;
        println!("cargo:rustc-env=GIT_HASH={}", git_hash.trim());
    }
    _ => {
        println!("cargo:rustc-env=GIT_HASH=unknown");
    }
}
```

This is used in the `--version` output via clap:

```rust
#[command(version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH"), ")"))]
```

Producing output like: `ashell 0.7.0 (abc1234)`

## Font Loading at Runtime

The subsetted fonts are embedded in the binary at compile time:

```rust
// In main.rs
const NERD_FONT: &[u8] = include_bytes!("../target/generated/SymbolsNerdFont-Regular-Subset.ttf");
const NERD_FONT_MONO: &[u8] = include_bytes!("../target/generated/SymbolsNerdFontMono-Regular-Subset.ttf");
const CUSTOM_FONT: &[u8] = include_bytes!("../assets/AshellCustomIcon-Regular.otf");
```

These are loaded into iced's font system at startup:

```rust
iced::daemon(/* ... */)
    .font(Cow::from(NERD_FONT))
    .font(Cow::from(NERD_FONT_MONO))
    .font(Cow::from(CUSTOM_FONT))
```
