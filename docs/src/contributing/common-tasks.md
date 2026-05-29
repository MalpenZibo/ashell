# Common Development Tasks

## Adding a New Icon

1. Find the Unicode codepoint from the [Nerd Fonts cheat sheet](https://www.nerdfonts.com/cheat-sheet).
2. Add it to `src/components/icons.rs`:
   ```rust
   pub const MY_NEW_ICON: char = '\u{f0001}';
   ```
3. Build — `build.rs` automatically subsets the font to include the new glyph.

## Adding a New Config Option

1. Add the field to the relevant config struct in `src/config.rs`:
   ```rust
   #[derive(Deserialize, Clone, Debug)]
   #[serde(default)]
   pub struct MyModuleConfig {
       pub new_option: bool,  // Add this
   }
   ```

2. Set a default value in the `Default` impl:
   ```rust
   impl Default for MyModuleConfig {
       fn default() -> Self {
           Self {
               new_option: false,
           }
       }
   }
   ```

3. Use the option in your module.

4. If the option should be hot-reloadable, handle it in the module's `ConfigReloaded` message.

## Adding a D-Bus Integration

1. Create proxy definitions in a `dbus.rs` file:
   ```rust
   #[zbus::proxy(
       interface = "org.example.Service1",
       default_service = "org.example.Service",
   )]
   trait Service1 {
       #[zbus(property)]
       fn my_property(&self) -> zbus::Result<String>;
   }
   ```

2. Implement the `ReadOnlyService` or `Service` trait.

3. Subscribe from a module. See [Writing a New Service](../services/writing-a-new-service.md).

## Working with the Theme

To add a new style or modify existing styles, edit `src/theme.rs`:

```rust
// Add a new button style method
pub fn my_button_style(&self) -> impl Fn(&Theme, Status) -> button::Style {
    let opacity = self.opacity;
    move |theme, status| {
        // Return button::Style based on status and theme
    }
}
```

## Adding a Translation

ashell uses [Fluent](https://projectfluent.org/) via the `i18n-embed` crate. 
Catalogs live in `i18n/<lang-tag>/ashell.ftl` and are baked into the binary at compile time with `include_str!`.

1. Copy the seed catalog to a new BCP-47 directory (e.g. `fr-FR`, `de-DE`, `it-IT`):
   ```bash
   mkdir -p i18n/fr-FR
   cp i18n/en-US/ashell.ftl i18n/<lang-tag>/ashell.ftl
   ```

2. Translate the right-hand side of every entry. Preserve:
   - the message keys and `## Section` comments,
   - every `{ $variable }` placeholder (e.g. `{ $count }`, `{ $ssid }`, `{ $duration }`),
   - Fluent selector keys: state selectors `[on]/*[off]` and plural selectors like `[one]/*[other]`. 
     The bracketed identifiers are Fluent literals — only the messages on the right-hand side get translated. 
     Use the [CLDR plural categories](https://www.unicode.org/cldr/charts/latest/supplemental/language_plural_rules.html) that apply to your language; `*[other]` is the required default.

3. Register the catalog in the `CATALOGS` slice in `src/i18n.rs` (keep it alphabetically sorted):
   ```rust
   const CATALOGS: &[(&str, &str)] = &[
       ("en-US", include_str!("../i18n/en-US/ashell.ftl")),
       ("<lang-tag>", include_str!("../i18n/<lang-tag>/ashell.ftl")),
   ];
   ```

4. Test by setting `language = "<lang-tag>"` in `~/.config/ashell/config.toml`, 
   or by launching with `LC_MESSAGES=<lang-tag> ./target/release/ashell`. 
   POSIX locale names are normalized to BCP-47 automatically.

5. Run `make check` before pushing.

## Running Checks Before Committing

Always run the full check before pushing:

```bash
make check
```

This runs `cargo fmt`, `cargo check`, and `cargo clippy -- -D warnings`.

## Debugging a Specific Module

To see debug output for a specific module:

```toml
# In config.toml
log_level = "warn,ashell::modules::my_module=debug"
```

## Testing Config Hot-Reload

1. Start ashell: `make start`
2. Edit `~/.config/ashell/config.toml` in another terminal
3. Save — changes should appear immediately
4. Check logs if changes don't apply: `tail -f $XDG_STATE_HOME/ashell/*.log`
