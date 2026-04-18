<h1 align="center">
  <a href="https://malpenzibo.github.io/ashell/">
    <img src="https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/logo_header_dark.svg" alt="ashell" height="140"/>
  </a>
</h1>
<p align="center">A ready to go Wayland status bar for Hyprland and Niri.</p>
<p align="center">
    <a href="https://matrix.to/#/#ashell:matrix.org"><img alt="Matrix" src="https://img.shields.io/badge/matrix-%23ashell-blue?logo=matrix"></a>
    <a href="https://github.com/MalpenZibo/ashell/blob/main/LICENSE"><img alt="GitHub License" src="https://img.shields.io/github/license/MalpenZibo/ashell"></a>
    <a href="https://github.com/MalpenZibo/ashell/releases"><img alt="GitHub Release" src="https://img.shields.io/github/v/release/MalpenZibo/ashell?logo=github"></a>
</p>

<p align="center">
    <a href="https://malpenzibo.github.io/ashell/docs/intro">Getting Started</a> | <a href="https://malpenzibo.github.io/ashell/docs/configuration">Configuration</a> | <a href="https://malpenzibo.github.io/ashell/dev-guide/">Developer&nbsp;Guide</a>
</p>

## ✨ Features

- Automatic Hyprland/Niri compositor detection
- Multi-monitor support (all monitors, active monitor, or specific targets)
- Hot-reload configuration (changes apply automatically via file watch)
- Bar positioning (top or bottom) with configurable layer (Bottom, Top, Overlay)
- Theming: Islands, Solid, and Gradient styles with custom colors, opacity, scale, and fonts
- OS Updates indicator with configurable check interval
- Hyprland/Niri Active Window (title, class, or initial title/class)
- Hyprland/Niri Workspaces with naming, color coding, and per-monitor visibility
- System Information (CPU, RAM, Disk, IP address, Network speed, Temperature) with warn/alert thresholds
- Hyprland/Niri Keyboard Layout with custom labels
- Hyprland Keyboard Submap
- System Tray with context menus
- Clock with calendar, weather, timezone cycling, and format cycling (Tempo)
- Privacy indicators (microphone, camera, and screenshare usage)
- Media Player with album art and track info
- Notification manager with toast popups, grouping, and urgency support
- Settings panel
  - Power menu (shutdown, suspend, hibernate, reboot, logout, lock)
  - Battery and peripheral battery information
  - Audio sources and sinks (with microphone)
  - Screen brightness
  - Network (WiFi scanning, password entry; supports NetworkManager and IWD backends)
  - VPN
  - Bluetooth
  - Power profiles
  - Idle inhibitor
  - Airplane mode
  - Custom quick-action buttons with status commands
- IPC socket for scripting and keybindings (`ashell msg <command>`)
- OSD overlay for volume, brightness, and airplane mode changes
- Custom Modules
  - Button (execute command on click)
  - Text (display-only, update UI with command output via `listen_cmd`)
  - Regex-based icon mapping and alert states

## 🛠️ Install

[![Packaging status](https://repology.org/badge/vertical-allrepos/ashell.svg)](https://repology.org/project/ashell/versions)

Refer to the [Installation](https://malpenzibo.github.io/ashell/docs/installation)
page for more details.

## ⚙️ Configuration

ashell comes with a default configuration that should work out of the box.

If you want to customize it you can refer to
the [Configuration](https://malpenzibo.github.io/ashell/docs/configuration)
page for more details.

## 💬 Community

Join the conversation on [Matrix](https://matrix.to/#/#ashell:matrix.org) or open an
[issue](https://github.com/MalpenZibo/ashell/issues) on GitHub.

## 📖 Developer Guide

If you want to contribute or understand the codebase, check out the
[Developer Guide](https://malpenzibo.github.io/ashell/dev-guide/).

## 🤖 AI-Assisted Contributions

AI-assisted contributions are accepted — the same quality standards apply regardless of how
the code was written. Frontier-class models (e.g., Claude Opus or equivalent) are strongly
recommended. **You are responsible for the code you submit**: review AI output carefully,
ensure `make check` passes, and be prepared to explain your changes.

Before working on a feature or large change, **discuss it with maintainers first**.
Small, incremental PRs are preferred — code review is manual and remains the bottleneck.

For the full AI contribution guide, see the
[Developer Guide](https://malpenzibo.github.io/ashell/dev-guide/contributing/ai-assisted-contributions.html).

## 📷 Screenshots

I will try my best to keep these screenshots as updated as possible but some details
could be different

#### default style

<img src="https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/ashell.png"></img>

#### solid style

<img src="https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/ashell-solid.png"></img>

#### gradient style

<img src="https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/ashell-gradient.png"></img>

#### opacity settings

<img src="https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/opacity.png"></img>

| ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/updates-panel.png)   | ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/system-menu.png)  |
| ------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------- |
| ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/tray-menu.png)       | ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/power-menu.png)   |
| ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/sinks-selection.png) | ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/network-menu.png) |
| ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/bluetooth-menu.png)  | ![](https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/gallery/vpn-menu.png)     |
