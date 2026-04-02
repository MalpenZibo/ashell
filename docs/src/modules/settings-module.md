# Deep Dive: The Settings Module

The Settings module (`src/modules/settings/`) is the most complex module in ashell. It composes multiple sub-modules and interacts with several services simultaneously.

## Structure

```
modules/settings/
├── mod.rs          # Main settings container, sub-menu navigation
├── audio.rs        # Volume control, sink/source selection
├── bluetooth.rs    # Bluetooth device management
├── brightness.rs   # Screen brightness slider
├── network.rs      # WiFi and VPN management
└── power.rs        # Power menu (shutdown, reboot, sleep, logout)
```

## Sub-Menu Navigation

The Settings panel uses a `SubMenu` enum for navigation:

```rust
pub enum SubMenu {
    Audio,
    Bluetooth,
    Network,
    // ... other sub-menus
}
```

The main settings view shows quick-access buttons. Clicking one navigates to the sub-menu view.

## The Action Enum

Settings is one of the modules that uses the Action pattern:

```rust
pub enum Action {
    None,
    Command(Task<Message>),
    CloseMenu,
    RequestKeyboard,
    ReleaseKeyboard,
    ReleaseKeyboardWithCommand(Task<Message>),
}
```

- **RequestKeyboard**: When the WiFi password input field needs keyboard focus, the module requests keyboard interactivity for the menu surface.
- **ReleaseKeyboard**: When the password dialog is dismissed.
- **CloseMenu**: When an action should close the settings panel.

## Service Integration

The Settings module interacts with multiple services:

| Sub-module | Service | Operations |
|------------|---------|------------|
| Audio | `AudioService` | List sinks/sources, set volume, toggle mute |
| Bluetooth | `BluetoothService` | List devices, connect/disconnect, toggle power |
| Brightness | `BrightnessService` | Get/set brightness level |
| Network | `NetworkService` | List WiFi networks, connect, manage VPN |
| Power | `LogindService` | Shutdown, reboot, sleep, hibernate |

## Password Dialog Integration

The network sub-module can trigger a password dialog for WiFi authentication. This is handled through the `password_dialog.rs` module at the app level, since the dialog needs its own input focus and keyboard interactivity.

## Custom Buttons

The Settings config supports user-defined buttons with status indicators:

```toml
[settings]
custom_buttons = [
    { icon = "\u{f023}", label = "VPN", status_cmd = "vpn-status", on_click = "vpn-toggle" }
]
```

These execute shell commands and display the result as a status indicator.

## Idle Inhibitor

The Settings panel includes an idle inhibitor toggle that prevents the system from going to sleep. This uses the `IdleInhibitorManager` service, which interacts with systemd-logind's inhibit API.
