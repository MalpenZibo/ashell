# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Added a clipboard button

### Fixed

- Fix workspace indicator foreground color selection

### Changed

- Nerd fonts are now included in the binary
- Workspace indicator now has an hover state

## [0.1.4] - 2024-10-23

### Fixed

- bluetooth quick toggle button no longer appear when no bluetooth device is available
- rfkill absence doesn't cause an error during network service initialization
- rfkill is launched using absolute path to avoid issues with $PATH
- webcam absence doesn't cause an error during privacy service initialization

### Changed

- added more logging to the services in case of errors

## [0.1.3] - 2024-10-22

### Fixed

- resolved problem with `cargo vendor` command

## [0.1.2] - 2024-10-17

### Added

- Privacy module: webcam usage indicator

### Changed

- Reduced clock refresh rate to 5 sec
- Increased update check frequency to 3600 sec

### Removed

- Privacy module: removed privacy sub-menu

### Fixed

- Improve wifi indicator

## [0.1.1] - 2024-10-03

### Fixed

- re-added vpn toggle functionality that was removed during the services refactor

## [0.1.0] - 2024-09-30

### Added

- First release
- Configuration system
- Lancher button
- OS Updates indicator
- Hyprland Active Window
- Hyprland Workspaces
- System Information (CPU, RAM, Temperature)
- Date time
- Settings panel
  - Power menu
  - Battery information
  - Audio sources and sinks
  - Screen brightness
  - Network stuff
  - VPN
  - Bluetooth
  - Power profiles
  - Idle inhibitor
  - Airplane mode
