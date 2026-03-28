<div align="center">
  <a href="https://malpenzibo.github.io/ashell/">
    <img src="https://raw.githubusercontent.com/MalpenZibo/ashell/main/website/static/img/logo_header_dark.svg" alt="ashell" height="140"/>
  </a>
</div>

## What is ashell?

ashell is a ready to go Wayland status bar for Hyprland and Niri.

Feel free to fork this project and customize it for your needs or just open an
issue to request a particular feature.

## 🚀 Getting Started

Refer to the [Getting Started](https://malpenzibo.github.io/ashell/docs/intro)
page on website

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

## 📖 Developer Guide

If you want to contribute or understand the codebase, check out the
[Developer Guide](https://malpenzibo.github.io/ashell/dev-guide/).

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
