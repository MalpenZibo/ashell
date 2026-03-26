# Network Service (NetworkManager/IWD)

The network service (`src/services/network/`) manages WiFi connections and VPN, supporting two backends: NetworkManager and IWD.

## Structure

```
services/network/
├── mod.rs       # Service implementation, backend abstraction
├── dbus.rs      # NetworkManager D-Bus proxy definitions
└── iwd_dbus/    # IWD D-Bus proxy definitions (extensive)
    ├── mod.rs
    ├── station.rs
    ├── network.rs
    ├── known_network.rs
    ├── device.rs
    └── ...
```

## Dual Backend

The network service supports two backends:

- **NetworkManager**: The traditional Linux network management daemon. Used on most distributions.
- **IWD (iNet Wireless Daemon)**: Intel's lightweight wireless daemon. Used on some minimal setups and can be used as a backend for NetworkManager.

The backend is detected based on which D-Bus service is available.

## Capabilities

- List available WiFi networks
- Connect/disconnect from WiFi networks
- WiFi network scanning
- VPN connection management
- Connection state monitoring
- Signal strength display

## Known Challenges

The network service is the most problematic service in the codebase (see [GitHub Issue #445](https://github.com/MalpenZibo/ashell/issues/445)):

- **WiFi scanning reliability**: Scan results can be stale or incomplete depending on the backend.
- **Architectural differences**: NetworkManager and IWD have fundamentally different D-Bus APIs and event models, making a unified abstraction difficult.
- **Connection state tracking**: Race conditions can occur between connection state changes and UI updates.

This is an active area of refactoring.

## IWD D-Bus Bindings

The IWD integration includes a comprehensive set of D-Bus proxy definitions — one of the most extensive in the project. This covers:

| Interface | Purpose |
|-----------|---------|
| `net.connman.iwd.Station` | WiFi station management |
| `net.connman.iwd.Network` | Network connection control |
| `net.connman.iwd.KnownNetwork` | Saved network management |
| `net.connman.iwd.Device` | Wireless device control |
| `net.connman.iwd.Adapter` | Physical adapter properties |
