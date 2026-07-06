# Audio Service (PulseAudio/PipeWire)

The audio service (`src/services/audio.rs`) manages volume control and audio device routing through PulseAudio (which PipeWire implements as a compatibility layer).

## Architecture

Unlike D-Bus services, the audio service uses **libpulse** (the PulseAudio C library) via the `libpulse-binding` crate. This requires a fundamentally different threading model.

```
┌──────────────────────┐     ┌──────────────────────┐
│   PulseAudio Thread  │     │     Tokio Runtime     │
│                      │     │                       │
│  libpulse Mainloop   │────►│  UnboundedReceiver    │
│  (OS thread, !Send)  │ tx  │                       │
│                      │     │  ThrottleExt adapter  │
│                      │◄────│                       │
│                      │ cmd │  iced Subscription    │
└──────────────────────┘     └──────────────────────┘
```

### Why a Dedicated Thread?

libpulse's `Mainloop` is `!Send` — it cannot be moved between threads. It also has its own event loop that conflicts with tokio. The solution is:

1. Spawn a dedicated OS thread (`std::thread::spawn`)
2. Run the PulseAudio mainloop on that thread
3. Communicate with the tokio runtime via `tokio::sync::mpsc::UnboundedSender/Receiver`

## Data Model

```rust
pub struct Device {
    pub name: String,
    pub description: String,
    pub volume: ChannelVolumes,
    pub is_mute: bool,
    pub is_filter: bool,        // Virtual devices (e.g., audio filters)
    pub ports: Vec<Port>,
}

pub struct Port {
    pub name: String,
    pub description: String,
    pub device_type: DevicePortType,
    pub active: bool,
}

pub struct Route<'a> {
    pub device: &'a Device,
    pub port: Option<&'a Port>,
}
```

## Throttling

PulseAudio can emit events very rapidly (e.g., during volume slider dragging). The `ThrottleExt` stream adapter in `services/throttle.rs` rate-limits these events to prevent UI thrashing:

```rust
// Conceptual usage
let stream = pa_events.throttle(Duration::from_millis(50));
```

This ensures the UI updates at most once every 50ms regardless of how fast PulseAudio emits events.

## Commands

The audio service implements the `Service` trait with these commands:

- Set default sink/source
- Set volume for a sink/source
- Toggle mute for a sink/source
- Move audio to a different device/port

## PipeWire Compatibility

Most modern Linux distributions use PipeWire, which provides a PulseAudio-compatible API. ashell's audio service works transparently with both PulseAudio and PipeWire — no code changes needed.

The `privacy.rs` service separately uses PipeWire's portal API for detecting active microphone/camera/screenshare sessions.
