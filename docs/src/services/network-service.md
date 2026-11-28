# Network Service (NetworkManager/IWD)

The network service (`crates/ashell-services/src/network/`, behind the `network` feature) manages WiFi connections and VPN, supporting two backends: NetworkManager and IWD. The iced glue in `src/services/network.rs` wraps it for the settings panel.

## Structure

```
crates/ashell-services/src/network/
├── mod.rs       # NetworkData, NetworkCommand, the Network handle and its updates() loop
├── dbus.rs      # NetworkManager backend and D-Bus proxy definitions
└── iwd_dbus/    # IWD backend and D-Bus proxy definitions (extensive)
    ├── mod.rs
    ├── station.rs
    ├── network.rs
    ├── device.rs
    └── ...
```

## API

Like the other `ashell-services` services (see [Service Traits](service-traits.md)):

- `Network::connect()` opens the system bus connection. It fails only when there is no system bus.
- `updates()` yields a `NetworkData` snapshot as soon as it is subscribed, then a fresh one on every change, or `None` while no backend runs: the backend in use, access points (one per SSID, strongest first), active and known connections, whether Wi-Fi is on, connectivity and whether a scan is running.
- `execute(NetworkCommand)` scans, toggles Wi-Fi and VPNs, and joins access points. Toggles read the current state from the backend, and their effect arrives through `updates()`.
- `read_psk(ssid)` reads the passphrase of the Wi-Fi network in use to share it (NetworkManager only, see `NetworkData::can_read_psk()`), and `PskOutcome::wifi_uri(ssid)` writes the `WIFI:` URI its QR code carries.

Wi-Fi icons, band labels, the indicator state and the in-flight state of a share request stay in ashell: the icon mapping is the `ActiveConnectionInfoExt` extension trait in the glue, and the crate reports a `Band` the glue writes as "2.4 GHz" and so on. Like the brightness glue, the network glue creates the service on the first `Some` and the settings panel drops it on `None`.

## Dual Backend

The network service supports two backends:

- **NetworkManager**: The traditional Linux network management daemon. Used on most distributions.
- **IWD (iNet Wireless Daemon)**: Intel's lightweight wireless daemon. Used on some minimal setups and can be used as a backend for NetworkManager.

Both backends implement the private `NetworkBackend` trait in `mod.rs`: reading a snapshot, subscribing to the changes, and the commands. NetworkManager is preferred when both are running, including when it uses IWD as its Wi-Fi backend.

## Availability

`updates()` watches who owns `org.freedesktop.NetworkManager` and `net.connman.iwd` on the bus, which never activates either daemon:

- While neither runs, it yields `None` and waits for a name owner change, without polling.
- When the backend in use stops, or NetworkManager starts while IWD is in use, it switches to the backend to use now. A read that fails because the daemon just stopped switches at once, without waiting for the owner change.
- A failed read is logged once as a warning, then retried every 5 seconds at debug level.

Airplane mode is not part of this service: it follows the kernel rfkill switches in its own service (`crates/ashell-services/src/airplane.rs`), which needs neither daemon.

Each backend subscribes once to everything that can change the snapshot: one `PropertiesChanged` match rule per interface it reads (`bus::interface_properties_changed`), so the bus doesn't send the others, filtered to the properties shown. NetworkManager also watches saved profiles being edited in place (`Settings.Connection.Updated`). IWD also watches stations and known networks appearing or going, and registers a `SignalLevelAgent` per station, unregistered before subscribing again when a station appears or goes. Signals already queued are handled as one burst.

A read is one `GetManagedObjects` call, plus one `GetOrderedNetworks` per IWD station for signal strength, with no cached proxy. For the known connections, NetworkManager's saved profiles are cached on the handle, fetched once per session and then only when new or updated, and emptied when NetworkManager's owner changes; joining a network reads them afresh. Signal strength changes and the scanning flag are applied to the last snapshot instead of reading everything again, and the access points are sorted again.

NetworkManager has no property telling that a scan is running: the handle keeps the devices it asked to scan, and `LastScan` changing on a device marks its scan done.

Joining a network through NetworkManager reads the access point's security from its `Flags`, `RsnFlags` and `WpaFlags`: a new profile gets the matching security section (and autoconnect off when connecting once), a saved one gets the passphrase, and `sae` is switched to `wpa-psk` for a WPA2-only access point. Enterprise and WEP networks, and secured ones without a passphrase, are refused; OWE is left to NetworkManager. A profile pinned to a BSSID is only rewritten for that access point.

IWD asks an agent for passphrases: each handle exports one, which answers from the passphrase `select_access_point` leaves for the network being joined.

## Capabilities

- List available WiFi networks
- Connect to WiFi networks (with an optional password)
- Enable/disable WiFi
- WiFi network scanning
- VPN connection toggling (NetworkManager only)
- Connection state monitoring
- Signal strength display
- Sharing the active WiFi network as a QR code (NetworkManager only)

## Known Challenges

The network service is the most problematic service in the codebase (see [GitHub Issue #445](https://github.com/MalpenZibo/ashell/issues/445)):

- **WiFi scanning reliability**: Scan results can be stale or incomplete depending on the backend.
- **Architectural differences**: NetworkManager and IWD have fundamentally different D-Bus APIs and event models, making a unified abstraction difficult.

This is an active area of refactoring.

## IWD D-Bus Bindings

The IWD integration includes a comprehensive set of D-Bus proxy definitions, one of the most extensive in the project. This covers:

| Interface | Purpose |
|-----------|---------|
| `net.connman.iwd.Station` | WiFi station management |
| `net.connman.iwd.Network` | Network connection control |
| `net.connman.iwd.Device` | Wireless device control |
| `net.connman.iwd.Adapter` | Physical adapter properties |
