# Architecture Overview

## High-Level Design

ashell is structured in three layers:

```
┌──────────────────────────────────────────────────┐
│                   main.rs                        │
│          (logging, CLI args, iced daemon)         │
└──────────────────────┬───────────────────────────┘
                       │
┌──────────────────────▼───────────────────────────┐
│                  Core Layer                       │
│  app.rs · config.rs · outputs.rs · theme.rs      │
│  menu.rs · password_dialog.rs                    │
│                                                  │
│  Central state, message routing, config,         │
│  multi-monitor management, theming               │
└───────┬──────────────────────────────┬───────────┘
        │                              │
┌───────▼──────────┐    ┌─────────────▼────────────┐
│   Modules (UI)   │    │   Services (Backend)     │
│                  │    │                          │
│  clock, tempo,   │    │  compositor, audio,      │
│  workspaces,     │    │  bluetooth, network,     │
│  settings,       │◄───│  mpris, tray, upower,    │
│  system_info,    │    │  brightness, privacy,    │
│  tray, media,    │    │  logind, idle_inhibitor  │
│  privacy, etc.   │    │                          │
└──────────────────┘    └──────────────────────────┘
```

- **Core Layer**: The `App` struct owns all state. It routes messages, manages windows/surfaces, and coordinates modules.
- **Modules**: Self-contained UI components displayed in the bar. Each module has its own `Message` type, `view()`, `update()`, and `subscription()`.
- **Services**: Backend integrations that produce events and accept commands. They have no UI. Modules consume services via subscriptions.

## Why iced?

[iced](https://iced.rs/) is a cross-platform GUI library for Rust that follows the [Elm Architecture](elm-architecture.md) (Model-View-Update). It was chosen for ashell because:

- **Rust-native**: No FFI bindings to GTK/Qt, keeping the stack uniform.
- **Elm Architecture**: Predictable state management with unidirectional data flow.
- **Wayland layer shell support**: Available through a fork (see below).
- **GPU-accelerated rendering**: Via wgpu.

### The iced Fork Chain

ashell does **not** use upstream iced directly. Instead, it uses a chain of forks:

```
upstream iced
    └── Pop!_OS / cosmic-iced (adds Wayland layer shell support via SCTK)
            └── MalpenZibo/iced (ashell's fork: fixes, features, Wayland tweaks)
```

The Pop!_OS fork adds SCTK (Smithay Client Toolkit) integration for Wayland layer surfaces, which is essential for a status bar. MalpenZibo's fork on top of that includes additional fixes and features specific to ashell's needs.

This fork dependency is tracked in `Cargo.toml` as a git dependency with a pinned revision:

```toml
iced = { git = "https://github.com/MalpenZibo/iced", rev = "...", features = [...] }
```

> **Note**: The fork dependency is a known maintenance burden. See [Known Limitations](known-limitations.md) for more context and the long-term plans.

## Design Principles

1. **Modular**: Each module is self-contained and optional. Adding or removing a module should not affect others.
2. **Reactive**: State flows in one direction. Events come in through subscriptions, state is updated, and the view re-renders.
3. **Service-agnostic UI**: Modules don't directly interact with system APIs. They consume data from services, making the UI layer testable and compositor-independent.
4. **Configuration-driven**: Everything is configurable via a TOML file with sensible defaults. The bar works out of the box with zero configuration.
