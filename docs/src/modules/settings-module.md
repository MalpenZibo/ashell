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

### Required System Packages

Each sub-module depends on a specific system service. If the service is not available, that part of the Settings panel will be hidden or non-functional. Additionally, the Settings module as a whole requires `systemd-logind` for shutdown/reboot/sleep actions.

| Sub-module | Required Package | D-Bus Service |
|------------|-----------------|---------------|
| Audio | PulseAudio or PipeWire-Pulse | — (uses libpulse directly) |
| Bluetooth | `bluez` | `org.bluez` |
| Brightness | systemd-logind (usually pre-installed) | `org.freedesktop.login1` |
| Network | `networkmanager` or `iwd` | `org.freedesktop.NetworkManager` or `net.connman.iwd` |
| Power (battery) | `upower` | `org.freedesktop.UPower` |

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

## Status Indicator Tooltips

The Settings module displays compact status indicators in the bar (audio, bluetooth, wifi, battery, peripheral battery). Hovering over these indicators shows a tooltip popup with detailed information.

### Tooltip Menu Types

Each indicator maps to a dedicated `MenuType` variant:

| Indicator | Menu Type | Content |
|-----------|-----------|---------|
| Audio / Microphone | `AudioTooltip` | Active sink and source device names |
| Bluetooth | `BluetoothTooltip` | Connected peripheral devices with battery level and device-specific battery icon |
| Network / VPN | `WifiTooltip` | Connected WiFi network name |
| Battery | `BatteryTooltip` | Charge percentage, charging/discharging/full status, time remaining |
| Peripheral Battery | `PeripheralBatteryTooltip(index)` | Device name, capacity percentage, and device-specific battery icon |

### Hover Interaction

Tooltips use the `PositionButton` hover events (`on_hover_with_position` / `on_unhover`). When the cursor enters an indicator, the module opens a tooltip menu positioned relative to the button. When the cursor leaves, the tooltip closes.

Tooltips are suppressed when a non-tooltip menu (e.g., the Settings panel) is already open, to avoid conflicting popups.

### Peripheral Battery Icons

Bluetooth and peripheral battery tooltips use device-specific battery icons from `Peripheral::get_icon_state()` (e.g., `KeyboardBatteryCharging`, `MouseBatteryMedium`, `HeadphoneBatteryLow`). These icons encode both the device type and battery level in a single glyph.
