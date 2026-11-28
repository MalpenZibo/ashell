# D-Bus Interfaces

ashell connects to several D-Bus services. This reference lists all interfaces used and where their proxy definitions are located.

## System Bus

| Service | Interface | Proxy File | Purpose | Required Package |
|---------|-----------|------------|---------|-----------------|
| BlueZ | `org.bluez.Adapter1` | `crates/ashell-services/src/bluetooth/dbus.rs` | Bluetooth adapter control | `bluez` |
| BlueZ | `org.bluez.Device1` | `crates/ashell-services/src/bluetooth/dbus.rs` | Bluetooth device management | `bluez` |
| BlueZ | `org.bluez.Battery1` | `crates/ashell-services/src/bluetooth/dbus.rs` | Bluetooth device battery level | `bluez` |
| NetworkManager | `org.freedesktop.NetworkManager` | `crates/ashell-services/src/network/dbus.rs` | Network state and connections | `networkmanager` |
| NetworkManager | `org.freedesktop.NetworkManager.Device` | `crates/ashell-services/src/network/dbus.rs` | Network device info | `networkmanager` |
| NetworkManager | `org.freedesktop.NetworkManager.Connection.Active` | `crates/ashell-services/src/network/dbus.rs` | Active connections | `networkmanager` |
| NetworkManager | `org.freedesktop.NetworkManager.Settings` | `crates/ashell-services/src/network/dbus.rs` | Saved connection profiles | `networkmanager` |
| NetworkManager | `org.freedesktop.NetworkManager.Settings.Connection` | `crates/ashell-services/src/network/dbus.rs` | Individual connection profile (e.g. VPN) | `networkmanager` |
| NetworkManager | `org.freedesktop.NetworkManager.Device.Wireless` | `crates/ashell-services/src/network/dbus.rs` | WiFi device control | `networkmanager` |
| NetworkManager | `org.freedesktop.NetworkManager.AccessPoint` | `crates/ashell-services/src/network/dbus.rs` | WiFi access point info | `networkmanager` |
| NetworkManager | `org.freedesktop.NetworkManager.AgentManager` | `crates/ashell-services/src/network/secret_agent.rs` | Registering ashell as a secret agent | `networkmanager` |
| NetworkManager | `org.freedesktop.NetworkManager.SecretAgent` (served by ashell) | `crates/ashell-services/src/network/secret_agent.rs` | Asking the user for Wi-Fi passphrases NetworkManager needs | `networkmanager` |
| IWD | `net.connman.iwd.Station` | `crates/ashell-services/src/network/iwd_dbus/` | WiFi station management | `iwd` |
| IWD | `net.connman.iwd.Network` | `crates/ashell-services/src/network/iwd_dbus/` | WiFi network connections | `iwd` |
| IWD | `net.connman.iwd.Device` | `crates/ashell-services/src/network/iwd_dbus/` | Wireless device | `iwd` |
| IWD | `net.connman.iwd.Agent` (served by ashell) | `crates/ashell-services/src/network/iwd_dbus/` | Passphrase agent: the passphrase typed to join a network, or asks the user for it | `iwd` |
| UPower | `org.freedesktop.UPower` | `crates/ashell-services/src/upower/dbus.rs` | Power daemon | `upower` |
| UPower | `org.freedesktop.UPower.Device` | `crates/ashell-services/src/upower/dbus.rs` | Battery/device info | `upower` |
| UPower | `org.freedesktop.UPower.PowerProfiles` | `crates/ashell-services/src/upower/dbus.rs` | Power profile switching | `power-profiles-daemon` |
| UPower | `org.freedesktop.UPower.KbdBacklight` | `crates/ashell-services/src/upower/dbus.rs` | Keyboard backlight brightness | `upower` |
| logind | `org.freedesktop.login1.Manager` | `crates/ashell-services/src/logind.rs` | Sleep/wake detection | systemd-logind |
| logind | `org.freedesktop.login1.Session` | `crates/ashell-services/src/brightness/dbus.rs` | Brightness control via SetBrightness | systemd-logind |

## Session Bus

| Service | Interface | Proxy File | Purpose | Required Package |
|---------|-----------|------------|---------|-----------------|
| MPRIS | `org.mpris.MediaPlayer2.*` bus names | `services/mpris/mod.rs` | Media player discovery (by bus name prefix) | MPRIS-compatible player |
| MPRIS | `org.mpris.MediaPlayer2.Player` | `services/mpris/dbus.rs` | Playback control | MPRIS-compatible player |
| StatusNotifier | `org.kde.StatusNotifierWatcher` (served by ashell only when no other watcher runs) | `services/tray/dbus.rs` | System tray icon registration; ashell always registers as a host (`org.freedesktop.StatusNotifierHost-<pid>-<n>`) with the current owner and follows it when it changes | — |
| StatusNotifier | `org.kde.StatusNotifierItem` | `services/tray/dbus.rs` | Individual tray icons | — |
| DBusMenu | `com.canonical.dbusmenu` | `services/tray/dbus.rs` | Tray menu layout (SNI context menus) | — |
| Notifications | `org.freedesktop.Notifications` (served by ashell) | `crates/ashell-services/src/notifications/dbus.rs` | Notification daemon | — |

## Checking D-Bus Availability

You can verify that D-Bus services are running:

```bash
# System bus
busctl --system list | grep -E "bluez|NetworkManager|UPower|login1|connman"

# Session bus
busctl --user list | grep -E "mpris|StatusNotifier|Notifications"
```

If a module is not working (e.g., battery info is missing), check that the corresponding service is active:

```bash
# Check if UPower is running (required for battery/power profile info)
systemctl status upower

# Check if BlueZ is running (required for Bluetooth)
systemctl status bluetooth

# Check if NetworkManager is running (required for WiFi/network)
systemctl status NetworkManager
```

## D-Bus Introspection

To explore a D-Bus interface:

```bash
# Example: inspect BlueZ adapter
busctl --system introspect org.bluez /org/bluez/hci0

# Example: inspect UPower battery
busctl --system introspect org.freedesktop.UPower /org/freedesktop/UPower/devices/battery_BAT0
```
