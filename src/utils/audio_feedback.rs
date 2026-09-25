use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::{Duration, Instant};

use libpulse_binding::sample::{Format, Spec};
use libpulse_binding::stream::Direction;
use libpulse_simple_binding::Simple;
use log::warn;

const SPEC: Spec = Spec {
    format: Format::S16NE,
    channels: 1,
    rate: 44100,
};
const BELL_PCM: &[u8] = include_bytes!("../../assets/bell.pcm");
const MIN_VOLUME_DELTA_PERCENT: u32 = 4;
const MIN_TIME_BETWEEN_PLAYS: Duration = Duration::from_millis(150);
const VOL_PERCENT: u32 = 65536 / 100;

pub struct AudioFeedback {
    enabled: bool,
    last_played: Instant,
    last_volume_percent: u32,
    sender: Option<Sender<()>>,
}

impl AudioFeedback {
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            last_played: Instant::now() - MIN_TIME_BETWEEN_PLAYS,
            last_volume_percent: 0,
            sender: Some(Self::spawn_player()),
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            last_played: Instant::now() - MIN_TIME_BETWEEN_PLAYS,
            last_volume_percent: 0,
            sender: None,
        }
    }

    pub fn play(&mut self, volume_raw: u32) {
        if !self.enabled {
            return;
        }
        let volume_percent = volume_raw / VOL_PERCENT;
        let delta = volume_percent.abs_diff(self.last_volume_percent);
        let elapsed = self.last_played.elapsed();
        if delta < MIN_VOLUME_DELTA_PERCENT || elapsed < MIN_TIME_BETWEEN_PLAYS {
            return;
        }
        self.trigger_bell();
        self.last_played = Instant::now();
        self.last_volume_percent = volume_percent;
    }

    pub fn play_mute_toggle(&mut self) {
        if !self.enabled {
            return;
        }
        if self.last_played.elapsed() < MIN_TIME_BETWEEN_PLAYS {
            return;
        }
        self.trigger_bell();
        self.last_played = Instant::now();
    }

    fn trigger_bell(&self) {
        if let Some(sender) = &self.sender
            && sender.send(()).is_err()
        {
            warn!("Audio feedback player thread is not running");
        }
    }

    // Runs on a single long-lived thread holding one PulseAudio connection,
    // so repeated beeps don't each pay a fresh connection handshake.
    fn spawn_player() -> Sender<()> {
        let (tx, rx) = mpsc::channel::<()>();
        thread::spawn(move || {
            let stream = match Simple::new(
                None,
                "ashell",
                Direction::Playback,
                None,
                "audio-feedback",
                &SPEC,
                None,
                None,
            ) {
                Ok(s) => s,
                Err(e) => {
                    warn!("Failed to open audio feedback stream: {e}");
                    return;
                }
            };

            while rx.recv().is_ok() {
                if let Err(e) = stream.write(BELL_PCM) {
                    warn!("Failed to write beep samples: {e}");
                } else if let Err(e) = stream.drain() {
                    warn!("Failed to drain beep: {e}");
                }
            }
        });
        tx
    }
}
