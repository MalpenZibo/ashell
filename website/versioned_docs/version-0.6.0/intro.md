---
sidebar_position: 1
---

# 🚀 Getting Started

Ashell is a status bar for Hyprland, written in Rust using the `iced` library.

## Does it only work on Hyprland?

While it’s currently tailored for Hyprland, it could potentially
work with other compositors.

However, it currently relies on [hyprland-rs](https://github.com/hyprland-community/hyprland-rs)
to gather information about the active window and workspaces.  
I haven’t implemented any feature flags to disable these functionalities or  
provide alternative methods to obtain this data.

## Features

- App Launcher button
- Clipboard button
- OS Updates indicator
- Hyprland Active Window
- Hyprland Workspaces
- System Information (CPU, RAM, Temperature)
- Hyprland Keyboard Layout
- Hyprland Keyboard Submap
- Tray
- Date and Time
- Privacy indicators (microphone, camera, and screen sharing usage)
- Media Player
- Settings Panel
  - Power menu
  - Battery information
  - Audio sources and sinks
  - Screen brightness
  - Network controls
  - VPN
  - Bluetooth
  - Power profiles
  - Idle inhibitor
  - Airplane mode
- Custom Modules
