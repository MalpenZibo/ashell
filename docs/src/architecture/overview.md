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
│  tempo,          │    │  compositor, audio,      │
│  workspaces,     │    │  bluetooth, network,     │
│  settings,       │◄───│  mpris, tray, upower,    │
│  system_info,    │    │  brightness, privacy,    │
│  notifications,  │    │  notifications, logind,  │
│  tray, media,    │    │  idle_inhibitor          │
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
- **Wayland layer shell support**: Available through [iced_layershell](https://github.com/MalpenZibo/iced_layershell).
- **GPU-accelerated rendering**: Via wgpu.

### iced_layershell

ashell uses upstream iced 0.14 with [iced_layershell](https://github.com/MalpenZibo/iced_layershell), a Wayland layer shell backend built on Smithay Client Toolkit (SCTK). This provides layer surface management, multi-surface support, and input handling without forking iced.

In `Cargo.toml` the dependency is aliased as `iced` for convenience:

```toml
iced = { package = "iced_layershell", git = "https://github.com/MalpenZibo/iced_layershell", tag = "v0.1.3", features = [...] }
```

> **History**: ashell previously depended on a Pop!_OS/cosmic-iced fork chain. The migration to iced_layershell (v0.8.0+) eliminated that fork dependency.

## Design Principles

1. **Modular**: Each module is self-contained and optional. Adding or removing a module should not affect others.
2. **Reactive**: State flows in one direction. Events come in through subscriptions, state is updated, and the view re-renders.
3. **Service-agnostic UI**: Modules don't directly interact with system APIs. They consume data from services, making the UI layer testable and compositor-independent.
4. **Configuration-driven**: Everything is configurable via a TOML file with sensible defaults. The bar works out of the box with zero configuration.
