# Audio Service (PulseAudio/PipeWire)

The audio service (`crates/ashell-services/src/audio/`, behind the `audio` feature) manages volume control and audio device routing through PulseAudio, which PipeWire implements as a compatibility layer. The iced glue in `src/services/audio.rs` wraps it for the settings panel.

## Architecture

Unlike D-Bus services, the audio service uses **libpulse** (the PulseAudio C library) via the `libpulse-binding` crate. This requires a different threading model.

```
┌──────────────────────────┐            ┌──────────────────────────┐
│    "pulseaudio" thread   │  snapshot  │      Tokio runtime       │
│                          │───────────►│                          │
│  libpulse Mainloop       │   watch    │  Audio::updates()        │
│  (OS thread, !Send)      │            │                          │
│                          │◄───────────│  Audio::execute(command) │
│                          │ mpsc + pipe│                          │
└──────────────────────────┘            └──────────────────────────┘
```

### Why a Dedicated Thread?

libpulse's `Mainloop` is `!Send`: it cannot be moved between threads, and it runs an event loop of its own that would block tokio. So:

1. `Audio::connect()` spawns an OS thread that owns the main loop (`audio/pulse.rs`).
2. The thread publishes the state as a snapshot on a `watch` channel, which is also what its commands are resolved against. A server event names what changed (the server info, the sinks or the sources), and only that is asked for again. The first snapshot is sent once all three are in; after that, unchanged snapshots are not sent, and a consumer that falls behind gets the latest one.
3. Commands go through a `std::sync::mpsc` channel; writing a byte to a pipe the main loop polls wakes it to apply them.

The thread also owns reconnection: when the server goes away (e.g. `systemctl --user restart pipewire` moves the context out of `Ready`), it publishes an empty `AudioData` and connects again every 5 seconds. Once every handle is dropped, the thread sees the command channel closed and exits.

## Data Model

```rust
pub struct AudioData {
    pub default_sink: String,
    pub default_source: String,
    pub sinks: Vec<Device>,
    pub sources: Vec<Device>,      // monitors left out
}

pub struct Device {
    pub name: String,
    pub description: String,
    pub volume: u32,               // the loudest channel; NORMAL_VOLUME is 100%
    pub is_mute: bool,
    pub ports: Vec<Port>,          // the available ones; none for a WirePlumber smart filter
    pub active_port: Option<String>, // the name of the port in use
}

pub struct Port {
    pub name: String,
    pub description: String,
    pub kind: PortKind,
}

pub struct Route<'a> {
    pub device: &'a Device,
    pub port: Option<&'a Port>,
}
```

No libpulse type is part of the API. `AudioData` has `active_sink`/`active_source` and `sink_routes`/`source_routes`, the choices the settings panel lists: each port of a device, or the device itself when it has none. A route `is_active` when its port is the one its device uses, so the panel marks only the port in use of the default device. The settings panel writes their labels.

## Commands

`AudioCommand` acts on the default sink or source, as the server has it when the command runs:

- Set default sink/source, optionally selecting a port (`DefaultSink`, `DefaultSource`)
- Set volume for the active sink/source (`SinkVolume`, `SourceVolume`), scaled over the channels the server reports at that moment, so their balance stays
- Toggle or set mute for the active sink/source (`ToggleSinkMute`, `SetSinkMute`, `ToggleSourceMute`, `SetSourceMute`)

The glue keeps what is only UI: the `Remote<u32>` sliders that show the requested volume while dragging.

## PipeWire Compatibility

Most modern Linux distributions use PipeWire, which provides a PulseAudio-compatible API. ashell's audio service works transparently with both PulseAudio and PipeWire.

The privacy service (`crates/ashell-services/src/privacy.rs`) separately watches the PipeWire registry (and every `/dev/video*` node) for detecting active microphone/camera/screenshare sessions.
