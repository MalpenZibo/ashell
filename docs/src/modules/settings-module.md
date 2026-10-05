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
    BatteryMenu,
    Power,
    Sinks,
    Sources,
    Wifi,
    Vpn,
    Bluetooth,
}
```

The main settings view shows quick-access buttons. Clicking one navigates to the sub-menu view.

## The Action Enum

Settings is one of the modules that uses the Action pattern:

```rust
pub enum Action {
    None,
    Command(Task<Message>),
    CloseMenu(SurfaceId),
    RequestKeyboardWithCommand(SurfaceId, Task<Message>),
    ReleaseKeyboard(SurfaceId),
    ReleaseKeyboardWithCommand(SurfaceId, Task<Message>),
    OpenTooltipMenu(SurfaceId, MenuType, ButtonUIRef),
    CloseTooltipMenu(SurfaceId, MenuType),
}
```

- **RequestKeyboardWithCommand**: When the WiFi password input field needs keyboard focus, the module requests keyboard interactivity for the menu surface (and focuses the input).
- **ReleaseKeyboard** / **ReleaseKeyboardWithCommand**: When the password dialog is confirmed or dismissed.
- **CloseMenu**: When an action should close the settings panel.
- **OpenTooltipMenu** / **CloseTooltipMenu**: Open and close the status indicator tooltips (see below).

## Service Integration

The Settings module interacts with multiple services:

| Sub-module | Service | Operations |
|------------|---------|------------|
| Audio | `AudioService` | List sinks/sources, set volume, toggle mute |
| Bluetooth | `BluetoothService` | List devices, connect/disconnect, toggle power |
| Brightness | `BrightnessService` | Get/set brightness level |
| Network | `NetworkService` | List WiFi networks, connect, manage VPN |
| Power | `UPowerService` | Battery and peripheral status, power profile, charge limit |

Shutdown, reboot, suspend, hibernate and logout run the configured shell commands (`shutdown_cmd`, `reboot_cmd`, `suspend_cmd`, `hibernate_cmd`, `logout_cmd`).

### Required System Packages

Each sub-module depends on a specific system service. If the service is not available, that part of the Settings panel will be hidden or non-functional. Additionally, the default power commands (`systemctl suspend`, `systemctl reboot`, `loginctl kill-user ...`) assume systemd.

| Sub-module | Required Package | D-Bus Service |
|------------|-----------------|---------------|
| Audio | PulseAudio or PipeWire-Pulse | — (uses libpulse directly) |
| Bluetooth | `bluez` | `org.bluez` |
| Brightness | systemd-logind (usually pre-installed) | `org.freedesktop.login1` |
| Network | `networkmanager` or `iwd` | `org.freedesktop.NetworkManager` or `net.connman.iwd` |
| Power (battery) | `upower` | `org.freedesktop.UPower` |

## Password Dialog Integration

The network sub-module can trigger a password dialog for WiFi authentication (or a warning before joining an open network). The dialog state lives in the Settings module (`network_dialog`), and its view comes from the `src/components/password_dialog.rs` component, rendered in place of the settings menu content. Because the dialog needs input focus, the module requests keyboard interactivity through its `Action` (see above).

## Custom Buttons

The Settings config supports user-defined buttons with status indicators:

```toml
[[settings.CustomButton]]
name = "VPN"
icon = "\u{f023}"
command = "vpn-toggle"
status_command = "vpn-status"
tooltip = "Toggle VPN"
```

`command` runs on click; the optional `status_command`'s exit status (success = active) drives the button's on/off state.

## Idle Inhibitor

The Settings panel includes an idle inhibitor toggle that prevents the system from going to sleep. This uses the `IdleInhibitorManager` service (`src/services/idle_inhibitor.rs`), which uses the Wayland `zwp_idle_inhibit_manager_v1` protocol.

## Status Indicator Tooltips

The Settings module displays compact status indicators in the bar (audio, microphone, bluetooth, network, VPN, battery, peripheral battery, brightness). Hovering over these indicators (except brightness) shows a tooltip popup with detailed information.

Tooltips can be enabled or disabled via the `enable_tooltips` setting inside `[settings]` (default: `true`). When disabled, hovering over indicators produces no tooltip popup.

```toml
[settings]
enable_tooltips = false
```

### Tooltip Menu Types

Each indicator maps to a dedicated `MenuType` variant:

| Indicator | Menu Type | Content |
|-----------|-----------|---------|
| Audio / Microphone | `AudioTooltip` | Active sink and source device names |
| Bluetooth | `BluetoothTooltip` | Connected devices with battery level (when available) |
| Network | `WifiTooltip` | Connected WiFi network name and band |
| VPN | `VpnTooltip` | Active VPN connection names |
| Battery | `BatteryTooltip` | Charge percentage, charging/discharging/full status, time remaining |
| Peripheral Battery | `PeripheralBatteryTooltip(index)` | Device name, capacity percentage, and device-specific battery icon |

### Hover Interaction

Tooltips use the `PositionButton` hover events (`on_hover_with_position` / `on_unhover`). When the cursor enters an indicator, the module opens a tooltip menu positioned relative to the button. When the cursor leaves, the tooltip closes.

Tooltips are suppressed when a non-tooltip menu (e.g., the Settings panel) is already open, to avoid conflicting popups.

### Peripheral Battery Icons

Peripheral battery indicators and tooltips use device-specific battery icons from `Peripheral::get_icon_state()` (e.g., `KeyboardBatteryCharging`, `MouseBatteryMedium`, `HeadphoneBatteryLow`). These icons encode both the device type and battery level in a single glyph.
