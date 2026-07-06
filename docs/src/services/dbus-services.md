# D-Bus Services Pattern

Most of ashell's services communicate with system daemons over D-Bus using the [zbus](https://docs.rs/zbus) crate (version 5).

## D-Bus Overview

D-Bus is the standard IPC mechanism on Linux desktops. ashell connects to the **system bus** (for hardware services like BlueZ, UPower, logind) and the **session bus** (for user services like MPRIS, StatusNotifier).

## The zbus Proxy Pattern

ashell uses zbus's `#[proxy]` attribute macro to generate type-safe D-Bus client code. These are defined in `dbus.rs` files alongside each service:

```rust
// Example from services/bluetooth/dbus.rs
#[zbus::proxy(
    interface = "org.bluez.Adapter1",
    default_service = "org.bluez",
)]
trait Adapter1 {
    #[zbus(property)]
    fn powered(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn set_powered(&self, value: bool) -> zbus::Result<()>;

    #[zbus(property)]
    fn discovering(&self) -> zbus::Result<bool>;

    fn start_discovery(&self) -> zbus::Result<()>;
    fn stop_discovery(&self) -> zbus::Result<()>;
}
```

The `#[zbus::proxy]` macro generates a `Adapter1Proxy` struct with async methods for each D-Bus method and property.

## Services Using D-Bus

| Service | Bus | D-Bus Service Name | Proxy File |
|---------|-----|-------------------|------------|
| Bluetooth | System | `org.bluez` | `services/bluetooth/dbus.rs` |
| Network (NM) | System | `org.freedesktop.NetworkManager` | `services/network/dbus.rs` |
| Network (IWD) | System | `net.connman.iwd` | `services/network/iwd_dbus/` |
| UPower | System | `org.freedesktop.UPower` | `services/upower/dbus.rs` |
| Logind | System | `org.freedesktop.login1` | `services/logind.rs` |
| MPRIS | Session | `org.mpris.MediaPlayer2.*` | `services/mpris/dbus.rs` |
| Tray | Session | `org.kde.StatusNotifierWatcher` | `services/tray/dbus.rs` |
| Privacy | Session | `org.freedesktop.portal.Desktop` | `services/privacy.rs` |
| Brightness | System | `org.freedesktop.login1.Session` | `services/brightness.rs` |

## Common D-Bus Service Structure

A typical D-Bus service follows this pattern:

```
services/my_service/
├── mod.rs    # Service trait impl, business logic
└── dbus.rs   # zbus proxy definitions
```

In `mod.rs`, the subscription connects to D-Bus and watches for signals/property changes:

```rust
fn subscribe() -> Subscription<ServiceEvent<Self>> {
    Subscription::run_with_id(
        TypeId::of::<Self>(),
        channel(10, async move |mut output| {
            // 1. Connect to D-Bus
            let connection = zbus::Connection::system().await.unwrap();

            // 2. Create proxy
            let proxy = MyProxy::new(&connection).await.unwrap();

            // 3. Get initial state
            let state = MyService::from_proxy(&proxy).await;
            output.send(ServiceEvent::Init(state)).await;

            // 4. Watch for changes
            let mut stream = proxy.receive_property_changed().await;
            while let Some(change) = stream.next().await {
                output.send(ServiceEvent::Update(change.into())).await;
            }
        }),
    )
}
```

## IWD Bindings

The IWD (iNet Wireless Daemon) integration has the most extensive D-Bus bindings in the project, located in `services/network/iwd_dbus/`. This includes proxy definitions for:

- `Station` — WiFi station management
- `Network` — WiFi network connections
- `KnownNetwork` — Previously connected networks
- `Device` — Wireless device control
- `AccessPoint` — AP mode (not used by ashell but defined for completeness)

## Signal Watching vs. Property Polling

ashell uses two approaches depending on the D-Bus service:

- **Signal watching** (preferred): Subscribe to D-Bus signals for real-time updates. Used for Bluetooth device changes, MPRIS playback state, etc.
- **Property polling**: Some services don't emit reliable signals for all changes. In these cases, ashell uses periodic polling or watches the `PropertiesChanged` signal.
