use std::time::{Duration, Instant};

use log::warn;
use tokio::process::Command;

const MIN_VOLUME_DELTA_PERCENT: u32 = 5;
const MIN_TIME_BETWEEN_PLAYS: Duration = Duration::from_millis(150);
const VOL_PERCENT: u32 = 65536 / 100;

fn kill_pid(pid: u32) {
    unsafe {
        libc::kill(pid as i32, libc::SIGKILL);
    }
}

pub struct AudioFeedback {
    last_played: Instant,
    last_volume_percent: u32,
    child_pid: Option<u32>,
    sound_path: Option<String>,
}

impl AudioFeedback {
    pub fn with_sound_path(sound_path: String) -> Self {
        Self {
            last_played: Instant::now() - MIN_TIME_BETWEEN_PLAYS,
            last_volume_percent: 0,
            child_pid: None,
            sound_path: Some(sound_path),
        }
    }

    pub fn disabled() -> Self {
        Self {
            last_played: Instant::now() - MIN_TIME_BETWEEN_PLAYS,
            last_volume_percent: 0,
            child_pid: None,
            sound_path: None,
        }
    }

    pub fn play(&mut self, volume_raw: u32) {
        let volume_percent = volume_raw / VOL_PERCENT;
        let delta = volume_percent.abs_diff(self.last_volume_percent);
        let elapsed = self.last_played.elapsed();

        if delta < MIN_VOLUME_DELTA_PERCENT || elapsed < MIN_TIME_BETWEEN_PLAYS {
            return;
        }

        self.kill_and_spawn();
        self.last_played = Instant::now();
        self.last_volume_percent = volume_percent;
    }

    pub fn play_mute_toggle(&mut self) {
        let elapsed = self.last_played.elapsed();
        if elapsed < MIN_TIME_BETWEEN_PLAYS {
            return;
        }

        self.kill_and_spawn();
        self.last_played = Instant::now();
    }

    fn kill_and_spawn(&mut self) {
        if let Some(sound_path) = &self.sound_path {
            if let Some(pid) = self.child_pid.take() {
                kill_pid(pid);
            }

            match Command::new("pw-cat")
                .arg("--playback")
                .arg(sound_path)
                .stderr(std::process::Stdio::null())
                .spawn()
            {
                Ok(child) => {
                    self.child_pid = child.id();
                }
                Err(e) => {
                    warn!("Failed to spawn pw-cat: {e}");
                }
            }
        }
    }
}

impl Drop for AudioFeedback {
    fn drop(&mut self) {
        if let Some(pid) = self.child_pid.take() {
            kill_pid(pid);
        }
    }
}
