# Code Style and Conventions

## Formatting

Use `cargo fmt` with the default rustfmt configuration:

```bash
cargo fmt
```

CI enforces formatting with `cargo fmt --all -- --check`.

## Linting

All clippy warnings are treated as errors:

```bash
cargo clippy -- -D warnings
```

This is enforced in CI. Fix all warnings before submitting a PR.

## Quick Check

The Makefile runs both:

```bash
make check
# Equivalent to: cargo fmt && cargo check && cargo clippy -- -D warnings
```

## Module Structure Conventions

### File Naming

- Simple modules: `src/modules/my_module.rs`
- Complex modules with sub-parts: `src/modules/my_module/mod.rs` + sub-files
- Services follow the same pattern: `src/services/my_service.rs` or `src/services/my_service/mod.rs`

### Message Enums

Every module defines its own `Message` enum:

```rust
#[derive(Debug, Clone)]
pub enum Message {
    // Module-specific variants
}
```

### Action Pattern

Modules that need to communicate side effects to the App use an `Action` enum:

```rust
pub enum Action {
    None,
    Command(Task<Message>),
    CloseMenu,
    // ...
}
```

### Constructor Convention

Modules take their config in `new()`:

```rust
pub fn new(config: MyModuleConfig) -> Self { /* ... */ }
```

## Logging

Use the `log` crate macros:

```rust
use log::{debug, info, warn, error};

debug!("Detailed debugging info");
info!("Notable events");
warn!("Something unexpected but recoverable");
error!("Something went wrong");
```

Avoid `println!` — all output should go through the logger so it's captured in log files.

## Error Handling

- Services use `anyhow` or custom error types
- Config parsing uses `Box<dyn Error>`
- Prefer logging errors over panicking in service code
- Use `unwrap_or_default()` or `unwrap_or_else()` for recoverable cases

## Imports

- Group imports by crate (std, external, internal)
- Use `crate::` prefix for internal imports
- Prefer specific imports over glob imports
