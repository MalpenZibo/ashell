# D-Bus Interfaces

ashell connects to several D-Bus services. This reference lists all interfaces used and where their proxy definitions are located.

## System Bus

| Service | Interface | Proxy File | Purpose | Required Package |
|---------|-----------|------------|---------|-----------------|
| BlueZ | `org.bluez.Adapter1` | `services/bluetooth/dbus.rs` | Bluetooth adapter control | `bluez` |
| BlueZ | `org.bluez.Device1` | `services/bluetooth/dbus.rs` | Bluetooth device management | `bluez` |
| NetworkManager | `org.freedesktop.NetworkManager` | `services/network/dbus.rs` | Network state and connections | `networkmanager` |
| NetworkManager | `org.freedesktop.NetworkManager.Device.Wireless` | `services/network/dbus.rs` | WiFi device control | `networkmanager` |
| NetworkManager | `org.freedesktop.NetworkManager.AccessPoint` | `services/network/dbus.rs` | WiFi access point info | `networkmanager` |
| IWD | `net.connman.iwd.Station` | `services/network/iwd_dbus/` | WiFi station management | `iwd` |
| IWD | `net.connman.iwd.Network` | `services/network/iwd_dbus/` | WiFi network connections | `iwd` |
| IWD | `net.connman.iwd.KnownNetwork` | `services/network/iwd_dbus/` | Saved networks | `iwd` |
| IWD | `net.connman.iwd.Device` | `services/network/iwd_dbus/` | Wireless device | `iwd` |
| UPower | `org.freedesktop.UPower` | `services/upower/dbus.rs` | Power daemon | `upower` |
| UPower | `org.freedesktop.UPower.Device` | `services/upower/dbus.rs` | Battery/device info | `upower` |
| logind | `org.freedesktop.login1.Manager` | `services/logind.rs` | Sleep/wake detection, power actions | systemd-logind |
| logind | `org.freedesktop.login1.Session` | `services/brightness.rs` | Brightness control via SetBrightness | systemd-logind |

## Session Bus

| Service | Interface | Proxy File | Purpose | Required Package |
|---------|-----------|------------|---------|-----------------|
| MPRIS | `org.mpris.MediaPlayer2` | `services/mpris/dbus.rs` | Media player discovery | MPRIS-compatible player |
| MPRIS | `org.mpris.MediaPlayer2.Player` | `services/mpris/dbus.rs` | Playback control | MPRIS-compatible player |
| StatusNotifier | `org.kde.StatusNotifierWatcher` | `services/tray/dbus.rs` | System tray icon registration | — |
| StatusNotifier | `org.kde.StatusNotifierItem` | `services/tray/dbus.rs` | Individual tray icons | — |
| Portal | `org.freedesktop.portal.Desktop` | `services/privacy.rs` | Privacy indicators (mic/camera) | `pipewire` |

## Checking D-Bus Availability

You can verify that D-Bus services are running:

```bash
# System bus
busctl --system list | grep -E "bluez|NetworkManager|UPower|login1|connman"

# Session bus
busctl --user list | grep -E "mpris|StatusNotifier|portal"
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
