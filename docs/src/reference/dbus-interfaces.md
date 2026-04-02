# D-Bus Interfaces

ashell connects to several D-Bus services. This reference lists all interfaces used and where their proxy definitions are located.

## System Bus

| Service | Interface | Proxy File | Purpose |
|---------|-----------|------------|---------|
| BlueZ | `org.bluez.Adapter1` | `services/bluetooth/dbus.rs` | Bluetooth adapter control |
| BlueZ | `org.bluez.Device1` | `services/bluetooth/dbus.rs` | Bluetooth device management |
| NetworkManager | `org.freedesktop.NetworkManager` | `services/network/dbus.rs` | Network state and connections |
| NetworkManager | `org.freedesktop.NetworkManager.Device.Wireless` | `services/network/dbus.rs` | WiFi device control |
| NetworkManager | `org.freedesktop.NetworkManager.AccessPoint` | `services/network/dbus.rs` | WiFi access point info |
| IWD | `net.connman.iwd.Station` | `services/network/iwd_dbus/` | WiFi station management |
| IWD | `net.connman.iwd.Network` | `services/network/iwd_dbus/` | WiFi network connections |
| IWD | `net.connman.iwd.KnownNetwork` | `services/network/iwd_dbus/` | Saved networks |
| IWD | `net.connman.iwd.Device` | `services/network/iwd_dbus/` | Wireless device |
| UPower | `org.freedesktop.UPower` | `services/upower/dbus.rs` | Power daemon |
| UPower | `org.freedesktop.UPower.Device` | `services/upower/dbus.rs` | Battery/device info |
| logind | `org.freedesktop.login1.Manager` | `services/logind.rs` | Sleep/wake detection, power actions |
| logind | `org.freedesktop.login1.Session` | `services/brightness.rs` | Brightness control via SetBrightness |

## Session Bus

| Service | Interface | Proxy File | Purpose |
|---------|-----------|------------|---------|
| MPRIS | `org.mpris.MediaPlayer2` | `services/mpris/dbus.rs` | Media player discovery |
| MPRIS | `org.mpris.MediaPlayer2.Player` | `services/mpris/dbus.rs` | Playback control |
| StatusNotifier | `org.kde.StatusNotifierWatcher` | `services/tray/dbus.rs` | System tray icon registration |
| StatusNotifier | `org.kde.StatusNotifierItem` | `services/tray/dbus.rs` | Individual tray icons |
| Portal | `org.freedesktop.portal.Desktop` | `services/privacy.rs` | Privacy indicators (mic/camera) |

## Checking D-Bus Availability

You can verify that D-Bus services are running:

```bash
# System bus
busctl --system list | grep -E "bluez|NetworkManager|UPower|login1|connman"

# Session bus
busctl --user list | grep -E "mpris|StatusNotifier|portal"
```

## D-Bus Introspection

To explore a D-Bus interface:

```bash
# Example: inspect BlueZ adapter
busctl --system introspect org.bluez /org/bluez/hci0

# Example: inspect UPower battery
busctl --system introspect org.freedesktop.UPower /org/freedesktop/UPower/devices/battery_BAT0
```
