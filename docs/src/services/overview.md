# Services Overview

Services are the backend layer of ashell. They manage communication with system APIs and produce events that modules consume. Services have no UI.

## Available Services

| Service | Location | Backend | Protocol | Required Package |
|---------|----------|---------|----------|-----------------|
| Compositor | `services/compositor/` | Hyprland / Niri | IPC socket | Hyprland or Niri |
| Audio | `services/audio.rs` | PulseAudio | libpulse C library | PulseAudio or PipeWire-Pulse |
| Brightness | `services/brightness.rs` | sysfs + logind | File I/O + D-Bus | systemd-logind |
| Bluetooth | `services/bluetooth/` | BlueZ | D-Bus | `bluez` |
| Network | `services/network/` | NetworkManager / IWD | D-Bus | `networkmanager` or `iwd` |
| MPRIS | `services/mpris/` | Media players | D-Bus | MPRIS-compatible player |
| Tray | `services/tray/` | StatusNotifierItem | D-Bus | — |
| UPower | `services/upower/` | UPower daemon | D-Bus | `upower` |
| Privacy | `services/privacy.rs` | PipeWire | PipeWire portals | `pipewire` |
| Idle Inhibitor | `services/idle_inhibitor.rs` | systemd-logind | D-Bus | systemd-logind |
| Logind | `services/logind.rs` | systemd-logind | D-Bus | systemd-logind |
| Throttle | `services/throttle.rs` | (utility) | Stream adapter | — |
| XDG Icons | `services/xdg_icons.rs` | (utility) | XDG icon theme | — |

## Shared Icon Resolution

`services/xdg_icons.rs` is a shared helper (not an event-producing service) used
by both the tray and the workspaces module to turn an identifier into a
displayable icon. Given a name it resolves an `XdgIcon` (`Image`, `Svg`, or a
`NerdFont` glyph fallback) via the XDG icon theme, a fuzzy/prefix match against
the installed icon set, and a `.desktop` `StartupWMClass` index.

Each consumer keeps its own concern on top of this core: the tray handles raw
D-Bus pixmaps, while the workspaces module lowercases the window class before
lookup. Results are memoized in a process-wide cache, and the underlying indexes
are built lazily; `warm_cache_async` pre-builds them off the UI thread when a
consumer is active.

## Services vs. Modules

| Aspect | Module | Service |
|--------|--------|---------|
| Has UI | Yes (`view()`) | No |
| Interacts with system | No (consumes services) | Yes |
| Has `Message` type | Yes | Has `UpdateEvent` + `ServiceEvent` |
| Defined by | Convention | `ReadOnlyService` / `Service` trait |
| Runs on | Main thread (iced event loop) | Async (tokio) or dedicated thread |

## Service Communication Pattern

```
Service (async/background)
    │
    ▼ ServiceEvent<S>
Module subscription
    │
    ▼ Module::Message
App::update()
    │
    ▼ Service::command() (for bidirectional services)
Service (executes command, returns result)
```

## Threading Model

- **Main thread**: iced event loop + rendering
- **Tokio runtime**: Most services (D-Bus watchers, timers, IPC)
- **Dedicated OS thread**: PulseAudio mainloop (libpulse requires its own event loop)
- **Communication**: `tokio::sync::mpsc` channels between threads, iced `channel()` for subscriptions
