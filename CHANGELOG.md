# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### WARNING BREAKING CHANGES

The configuration switch from `yaml` to `toml` format. The configuration file must be updated to adapt to the new format.
The `camelCase` format has been removed in favor of `snake_case`, which better aligns with the `toml` syntax.

You could use an online tool like: https://transform.tools/yaml-to-toml but remember to change the `camelCase` to `snake_case` format.

Now the configuration file is located in `~/.config/ashell/config.toml`

### Added

- Add font name configuration
- Add main bar solid and gradient style
- Add main bar opacity settings
- Add menu opacity and backdrop settings
- Add experimental IWD support as fallback for the network module
- Handle system with multiple battery
- Allow to specify custom labels for keyboard layouts
- Allow to always show a specific number of workspaces, whether they have windows or not

### Changed

- Change configuration file format
- Enhance the system info module adding network and disk usage
- Simplify style of "expand" button on wifi/bluetooth buttons
- Allow to specify custom labels for keyboard layouts
- Removed background on power info in menu

### Fixed

- Fix missing tray icons

## [0.4.1] - 2025-03-16

### Added

- Media player module

### Fixed

- Fix bluetooth service in NixOS systems
- Fix brightness wrong value in some situations
- Fix settings menu not resetting it's state when closed
- Fix bluetooth service crash during listing of devices without battery info
- Fix centerbox children positioning

### Thanks

- Huge thanks to @mazei513 for the implementation of the media player module

## [0.4.0] - 2025-01-19

A big update with new features and new configurations!

The configuration file must be updated to adapt to the new stuff.

### Added

- Multi monitor support
- Tray module
- Dynamic modules system configuration
- New workspace module configuration

### Changed

- Update to pop-os Iced 14.0-dev
- Dynamic menu positioning

### Thanks

- @fiersik for participating in the discussions
- @ReshetnikovPavel for the proposal of the new dynamic modules system configuration

## [0.3.1] - 2024-12-13

### Fixed

- Fix upower service startup fail in case of missing `org.freedesktop.UPower.PowerProfiles` dbus interface

## [0.3.0] - 2024-11-26

A small release with some new Hyprland related modules

Thanks @fiersik for the new modules and contributions to the project

### Added

- Hyprland Keyboard Layout module
- Hyprland Keyboard Submap module

### Changed

- Change main surface layer from Top to Bottom

## [0.2.0] - 2024-11-08

### Added

- Support for special workspaces

### Fixed

- Ashell crash when the title module try to split a multi-byte character
- Removed fixed monitor name in the workspace module
- Fix privacy webcam usage check during initialization
- Fix microphone selection
- Fix sink and source slider toggle button state
- Fix brightness initial value

### Thanks

- @fiersik for all the feedback
- @leftas for the PRs to fix the special workspace crash and the title module

## [0.1.5] - 2024-11-04

### Added

- Added a clipboard button

### Fixed

- Fix workspace indicator foreground color selection

### Changed

- Nerd fonts are now included in the binary
- Workspace indicator now has an hover state

### Thanks

- @fiersik for the clipboard button and the ALT Linux package

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
