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
