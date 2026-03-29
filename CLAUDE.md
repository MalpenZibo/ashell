# CLAUDE.md — ashell project context

## Quick reference

```bash
make check    # format check + cargo check + clippy -D warnings (run before pushing)
make build    # cargo build --release
make fmt      # cargo fmt
make start    # run ./target/release/ashell
make install  # install binary to /usr/bin (requires sudo)
```

## Project overview

ashell is a Wayland status bar for Hyprland and Niri, written in Rust with the iced GUI framework.
It follows the Elm architecture (model → update → view) with a modular design separating UI modules from backend services.

- **Edition:** 2024, **MSRV:** 1.89
- **License:** MIT
- **iced fork:** custom fork with multi-window, wayland, wgpu support

## Code quality

- **Clippy is strict:** CI runs `cargo clippy --all-features -- -D warnings`. Zero warnings allowed.
- **Formatting:** `cargo fmt --all -- --check` enforced in CI.
- No test suite — quality is enforced through clippy, formatting, and build checks.

## Project structure

```
src/
├── main.rs              # entry point, font embedding
├── app.rs               # App state, Elm update/view cycle
├── config.rs            # configuration types (TOML deserialization)
├── theme.rs             # theming (Islands/Solid/Gradient styles)
├── menu.rs              # menu UI
├── outputs.rs           # multi-monitor output management
├── components/          # shared UI components, icons
├── modules/             # UI modules (clock, workspaces, settings, tray, etc.)
│   └── settings/        # settings sub-panels (audio, network, bluetooth, power, brightness)
├── services/            # backend services (D-Bus, IPC, system integration)
│   ├── compositor/      # Hyprland/Niri IPC abstraction
│   ├── network/         # NetworkManager + IWD backends
│   ├── bluetooth/
│   ├── mpris/           # media player control
│   ├── tray/
│   └── upower/          # battery/power
├── widgets/             # custom iced widgets (centerbox, position_button, menu_wrapper)
└── utils/
```

## Key system dependencies

libxkbcommon, libwayland, libpipewire-0.3, libpulse, dbus, udev, pkg-config, clang/llvm

## build.rs

The build script does two things:
1. Captures `GIT_HASH` from `git rev-parse --short HEAD`
2. **Font subsetting:** parses `src/components/icons.rs` for `\u{XXXX}` patterns and creates a subset of Nerd Fonts containing only used glyphs (via the `allsorts` crate)

## Commit conventions

Format: `<type>(<optional-scope>): <subject>`

Types: `feat`, `fix`, `docs`, `chore`, `style`, `refactor`, `perf`, `ci`
Scope examples: `fix(brightness)`, `feat(system_info)`, `fix(network)`

## Branch naming

`feat/`, `fix/`, `chore/`, `docs/`, `refactor/`, `style/` prefixes. PRs target `main`.

## Documentation

- **User docs:** `website/` (Docusaurus) → https://malpenzibo.github.io/ashell/
- **Developer guide:** `docs/` (mdbook) → https://malpenzibo.github.io/ashell/dev-guide/
- The developer guide is AI-generated. AI contributions are welcome but should use frontier-class models.

## Architecture patterns

- **Elm architecture:** `App` struct holds all state. `Message` enum drives updates. `update()` returns `Task<Message>` for async work.
- **Services** are backend abstractions (D-Bus, IPC). Two traits: `ReadOnlyService` and `Service` (mutable).
- **Modules** are UI components (workspaces, clock, settings). Each implements `view()` returning iced `Element`.
- **Compositor abstraction:** `CompositorService` trait with Hyprland and Niri implementations, auto-detected at runtime.
- **Config hot-reload:** inotify file watcher triggers `ConfigChanged` message on config file changes.
