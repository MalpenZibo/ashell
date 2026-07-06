# Glossary

## Wayland Terminology

| Term | Definition |
|------|-----------|
| **Wayland** | The display server protocol used by modern Linux desktops, replacing X11 |
| **Compositor** | The program that manages windows and display output (e.g., Hyprland, Niri) |
| **Layer shell** | A Wayland protocol (`wlr-layer-shell`) that allows surfaces to be placed in specific layers (Background, Bottom, Top, Overlay) |
| **Layer surface** | A Wayland surface managed by the layer shell protocol |
| **Anchor** | Edges of the screen that a layer surface attaches to (top, bottom, left, right) |
| **Exclusive zone** | Screen space reserved by a layer surface that other windows won't overlap |
| **Output** | A display/monitor in Wayland terminology |
| **SCTK** | Smithay Client Toolkit — Rust library for Wayland client development |
| **xdg_popup** | Wayland protocol for creating popup surfaces attached to other surfaces |

## iced Terminology

| Term | Definition |
|------|-----------|
| **iced** | The Rust GUI framework used by ashell |
| **Element** | An iced widget tree node — the return type of `view()` |
| **Task** | A one-shot async effect that produces a message when complete |
| **Subscription** | A long-lived event stream that continuously produces messages |
| **daemon** | iced's multi-window mode, where the application manages multiple surfaces |
| **Theme** | iced's styling system with palette-based colors |
| **Palette** | A set of named colors (background, text, primary, secondary, success, danger) |
| **Widget** | A UI component (button, text, row, column, container, etc.) |

## ashell Terminology

| Term | Definition |
|------|-----------|
| **Module** | A self-contained UI component displayed in the bar (e.g., Clock, Workspaces, Settings) |
| **Service** | A backend integration that communicates with system APIs (e.g., audio, bluetooth, compositor) |
| **Islands** | A bar style where each module group has its own rounded background container |
| **Solid** | A bar style with a continuous flat background |
| **Gradient** | A bar style where the background fades from solid to transparent |
| **Menu** | A popup panel that appears when clicking certain modules |
| **Centerbox** | Custom widget providing a three-column layout with true centering |
| **ButtonUIRef** | Position and size information of a button, used for menu placement |
| **Hot-reload** | Automatic application of config changes without restarting |
| **Tempo** | The advanced clock module (replacement for the deprecated Clock module) |
| **Custom module** | A user-defined module that executes shell commands |

## Architecture Terminology

| Term | Definition |
|------|-----------|
| **MVU** | Model-View-Update — the Elm Architecture pattern used by iced |
| **Message** | An event type that triggers state changes (the "Update" in MVU) |
| **Action** | A module-level return type that communicates side effects to the App |
| **ServiceEvent** | The standard event enum for services (`Init`, `Update`, `Error`) |
| **ReadOnlyService** | A service that only produces events (no commands) |
| **Service (trait)** | A service that produces events and accepts commands |
| **Broadcast** | The pattern used by the compositor service to share events across multiple subscribers |

## System Terminology

| Term | Definition |
|------|-----------|
| **D-Bus** | The standard Linux IPC mechanism for communicating with system services |
| **zbus** | The Rust crate used for D-Bus communication |
| **BlueZ** | The Linux Bluetooth stack |
| **NetworkManager** | Standard Linux network management daemon |
| **IWD** | iNet Wireless Daemon — Intel's lightweight wireless daemon |
| **UPower** | Power management daemon (battery info, power profiles) |
| **MPRIS** | Media Player Remote Interfacing Specification — D-Bus interface for media player control |
| **StatusNotifierItem** | D-Bus protocol for system tray icons |
| **logind** | systemd's login manager (handles sleep/wake, power actions) |
| **PulseAudio** | Linux audio server (also provided as a compatibility layer by PipeWire) |
| **PipeWire** | Modern Linux multimedia framework (replaces PulseAudio and JACK) |
| **Nerd Font** | A font family patched with programming icons and symbols |
| **cargo-dist** | Rust tool for creating distributable binaries and installers |
| **nfpm** | Tool for creating .deb and .rpm packages |
