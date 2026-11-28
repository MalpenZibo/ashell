# Network Service (NetworkManager/IWD)

The network service (`crates/ashell-services/src/network/`, behind the `network` feature) manages WiFi connections and VPN, supporting two backends: NetworkManager and IWD. The iced glue in `src/services/network.rs` wraps it for the settings panel.

## Structure

```
crates/ashell-services/src/network/
├── mod.rs           # NetworkData, NetworkCommand, the Network handle and its updates() loop
├── dbus.rs          # NetworkManager backend and D-Bus proxy definitions
├── password.rs      # Pending password requests, shared by both agents
├── secret_agent.rs  # The NetworkManager secret agent
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
- `password_requests()` yields the Wi-Fi passphrases NetworkManager or IWD wait for, and `reply_password(id, PasswordReply)` ends one (see [Password Requests](#password-requests)).

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

IWD asks an agent for passphrases: each handle exports one, which answers from the passphrase `select_access_point` leaves for the network being joined, and otherwise asks the user (see below).

## Password Requests

When a daemon needs the passphrase of a Wi-Fi network, for example because the one saved was rejected, it asks an agent and the activation waits for the answer. Both backends turn these into one list of `PasswordRequest`s (an id and the SSID: only WPA, WPA2 and WPA3 Personal passphrases are asked for), oldest first, which `password_requests()` yields: the pending ones when subscribed, then after every change. A request ends by id with `reply_password` and a `PasswordReply`: `Password`, `Cancel` (the user dismissed it: NetworkManager asks no other agent) or `Decline` (nothing can show it: NetworkManager asks the other agents). IWD has a single agent, so for IWD `Decline` is the same as `Cancel`: the connection attempt fails. It is a plain method rather than a `NetworkCommand`, since it only ends a call waiting in the handle: no daemon is called and nothing can fail. A request also leaves the list when its daemon takes it back, or stops or restarts.

It is a stream of its own rather than part of `NetworkData` because subscribing has a side effect on the whole system: while one is listened to, the handle is a NetworkManager secret agent, so NetworkManager routes secrets to ashell instead of failing or asking another agent. Without a listener, both agents leave every request to other agents at once, and the requests pending when the last stream goes are withdrawn. The glue (`NetworkService::subscription(password_requests)`) merges both streams of one handle, since an answer has to reach the agents of that connection, and listens to the requests only while the settings module is in the bar: the settings services also run without it, for the OSD, and nothing could show a request there.

NetworkManager (`secret_agent.rs`):

- The agent is served at `/org/freedesktop/NetworkManager/SecretAgent` and registered with `AgentManager.Register("org.ashell.SecretAgent")` every time NetworkManager starts, following its bus name like `updates()` does (`secret_agent::serve`). NetworkManager owns its name before it serves the `AgentManager`, so a failed registration is retried, from 100 ms up to every 5 seconds (warned about once, like other retries), until it succeeds or the owner changes.
- The handle remembers the NetworkManager instance it sent `Register` to and whether it replied. The streams of a handle take turns keeping the agent registered, one at a time, so a second stream, or one taking over from a stream dropped, registers nothing again, and only an owner change after the first withdraws requests. A takeover that finds a `Register` left unreplied sends it again and takes a refusal from the same instance as that earlier registration.
- NetworkManager accepts one agent per identifier and user: another ashell of the same user keeps the registration, and this one logs a warning.
- `GetSecrets` asks the user only for the `802-11-wireless-security` setting of a profile whose `key-mgmt` is `wpa-psk` or `sae`, with no hint other than `psk`, and only when the `ALLOW_INTERACTION` flag is set. ashell keeps no secrets, so there is nothing to answer without asking, and `REQUEST_NEW` (the last secret was wrong) is asked like any other. Everything else, VPN and 802.1X secrets included, gets `NoSecrets` at once, so NetworkManager asks the next agent (nm-applet, a desktop's own agent). The answer is `{"802-11-wireless-security": {"psk": ...}}`; a dismissed request gets `UserCanceled`, which makes NetworkManager stop asking.
- `CancelGetSecrets` withdraws the request of that profile and setting.
- `SaveSecrets` and `DeleteSecrets` succeed without storing anything: NetworkManager saves the secrets it owns itself, and agent-owned ones are asked again next time.

IWD: `RequestPassphrase` answers from the passphrase ashell was given to join that network, and otherwise asks the user for it, with the SSID read from the network's `Name`. A dismissed or withdrawn request gets `net.connman.iwd.Agent.Error.Canceled`. `Cancel` and `Release` withdraw the request IWD waits for: it waits for one at a time per agent.

The agents' methods wait for the user inside the zbus method call, so they are `async fn(&self)`: zbus runs each call in a task of its own, and `CancelGetSecrets` or `Cancel` arrives while `GetSecrets` or `RequestPassphrase` waits.

In the UI, the settings module shows the oldest request in the Wi-Fi password dialog whenever no other dialog is open; with no settings menu open, a dialog left from the last one doesn't count. `App` does the rest, since it knows the menus (`App::sync_password_request`):

- With a settings menu open, it gives that menu surface the keyboard and focuses the input. The focus waits for the next update, since iced_layershell runs widget operations before it creates the surfaces requested in the same update.
- Otherwise it opens the menu through `Message::ToggleMenu`: where it was last opened, or else on the first bar. Before the settings button was ever pressed its position is unknown, so the menu opens at the left edge when the module is in the left section, and at the right edge otherwise, the centre included.
- A menu opened that way closes once the request ends (answered, dismissed or withdrawn), while a menu the user opened stays open and releases the keyboard, unless Escape needs it.
- With the bar hidden, or no bar at all, the requests are declined at once.

Closing the menu without answering keeps the request pending, and the dialog comes back when the menu opens.

## Capabilities

- List available WiFi networks
- Connect to WiFi networks (with an optional password)
- Ask for the passphrase NetworkManager or IWD needs to join a network
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
