# Summary

[Introduction](introduction.md)

---

# Getting Started

- [Prerequisites](getting-started/prerequisites.md)
- [Building from Source](getting-started/building.md)
- [Development Environment](getting-started/development-environment.md)
- [Project Layout](getting-started/project-layout.md)

---

# Architecture

- [Architecture Overview](architecture/overview.md)
- [The Elm Architecture in ashell](architecture/elm-architecture.md)
- [Data Flow: Messages, Tasks, and Subscriptions](architecture/data-flow.md)
- [Surface Model: Layer Shell and Multi-Monitor](architecture/surface-model.md)
- [Known Limitations and Design Debt](architecture/known-limitations.md)

---

# Core Systems

- [The App Struct](core/app-struct.md)
- [The Message Enum](core/message-enum.md)
- [Configuration System](core/config-system.md)
- [Theme System](core/theme-system.md)
- [Outputs and Surface Management](core/outputs-and-surfaces.md)
- [Menu System](core/menu-system.md)

---

# Modules (UI)

- [Modules Overview](modules/overview.md)
- [Anatomy of a Module](modules/anatomy-of-a-module.md)
- [Module Registry and Routing](modules/module-registry.md)
- [Walkthrough: The Clock Module](modules/clock-walkthrough.md)
- [Deep Dive: The Settings Module](modules/settings-module.md)
- [Writing a New Module](modules/writing-a-new-module.md)

---

# Services (Backend)

- [Services Overview](services/overview.md)
- [Service Traits: ReadOnlyService and Service](services/service-traits.md)
- [Compositor Service and Abstraction Layer](services/compositor-service.md)
- [D-Bus Services Pattern](services/dbus-services.md)
- [Audio Service (PulseAudio/PipeWire)](services/audio-service.md)
- [Network Service (NetworkManager/IWD)](services/network-service.md)
- [Writing a New Service](services/writing-a-new-service.md)

---

# Custom Widgets

- [Widgets Overview](widgets/overview.md)
- [Centerbox](widgets/centerbox.md)
- [PositionButton](widgets/position-button.md)
- [MenuWrapper](widgets/menu-wrapper.md)

---

# Build System

- [Cargo and Dependencies](build-system/cargo-and-dependencies.md)
- [build.rs: Font Subsetting](build-system/build-rs-font-subsetting.md)
- [Nix Flake](build-system/nix-flake.md)

---

# CI and Release

- [CI Pipeline](ci-and-release/ci-pipeline.md)
- [Release Process](ci-and-release/release-process.md)
- [Packaging (deb, rpm, Arch, Nix)](ci-and-release/packaging.md)

---

# Contributing

- [Contribution Workflow](contributing/workflow.md)
- [Code Style and Conventions](contributing/code-style.md)
- [Testing and Debugging](contributing/testing-and-debugging.md)
- [Common Development Tasks](contributing/common-tasks.md)

---

# Reference

- [Configuration Reference](reference/config-reference.md)
- [Environment Variables](reference/environment-variables.md)
- [D-Bus Interfaces](reference/dbus-interfaces.md)
- [Glossary](reference/glossary.md)
