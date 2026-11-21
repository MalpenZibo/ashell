//! The timer: a countdown, with presets and a start/pause/reset.

use std::time::{Duration, Instant};

use ashell_i18n::t;
use guido::prelude::*;
use styletto::components::button::{self, Fill, Variant, chip_button, icon_button};
use styletto::components::progress::progress_bar;
use styletto::components::roll;
use styletto::foundations::control::Size;
use styletto::foundations::tokens::size;
use styletto::foundations::typography as ty;
use styletto::{Icon, centered_row, ui};

use super::duration;
use super::{alarms, clock::Clock};

pub const TOGGLE: &str = "toggle_timer";
pub const RESET: &str = "reset_timer";
pub const ADD_MINUTE: &str = "add_minute";

pub const ACTIONS: &[&str] = &[TOGGLE, RESET, ADD_MINUTE];

/// The minutes a tap sets, and the first one a fresh timer has.
const PRESETS: [u64; 8] = [1, 5, 10, 15, 25, 30, 45, 60];
const DEFAULT_MINUTES: u64 = 5;

/// What the timer is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Set, not started.
    Idle,
    Running,
    /// Started, then stopped with time left.
    Paused,
    /// Reached zero.
    Finished,
}

/// A countdown.
///
/// The time left is kept in one of two places: `ends_at` while it runs, so
/// the display is right without a tick, and `paused` while it does not.
#[derive(Clone, Copy)]
pub struct Timer {
    /// How long a fresh start counts down.
    duration: RwSignal<Duration>,
    /// When it ends, while it runs.
    ends_at: RwSignal<Option<Instant>>,
    /// What is left while it is stopped.
    paused: RwSignal<Duration>,
    /// Whether it has been started: a fresh timer is idle, not paused.
    engaged: RwSignal<bool>,
    finished: RwSignal<bool>,
    /// The length it counts down over: its preset, plus any time added while
    /// it runs or is stopped, so the progress bar follows.
    total: RwSignal<Duration>,
}

impl Timer {
    pub fn new() -> Self {
        let duration = Duration::from_secs(DEFAULT_MINUTES * 60);

        Self {
            duration: create_signal(duration),
            ends_at: create_signal(None),
            paused: create_signal(Duration::ZERO),
            engaged: create_signal(false),
            finished: create_signal(false),
            total: create_signal(duration),
        }
    }

    /// A timer at rest, for pictures of the module.
    #[cfg(test)]
    pub fn fixed() -> Self {
        Self::new()
    }

    pub fn duration(self) -> Duration {
        self.duration.get()
    }

    pub fn state(self) -> State {
        if self.ends_at.get().is_some() {
            State::Running
        } else if self.finished.get() {
            State::Finished
        } else if self.engaged.get() {
            State::Paused
        } else {
            State::Idle
        }
    }

    pub fn running(self) -> bool {
        self.ends_at.get().is_some()
    }

    pub fn finished(self) -> bool {
        self.finished.get()
    }

    /// Whether it has finished, for a tick that is not a view.
    pub fn finished_untracked(self) -> bool {
        self.finished.get_untracked()
    }

    /// What is left at `now`.
    pub fn remaining(self, now: Instant) -> Duration {
        match self.ends_at.get() {
            Some(end) => end.saturating_duration_since(now),
            None if self.engaged.get() => self.paused.get(),
            None => self.duration.get(),
        }
    }

    /// How far it has gone, from 0 to 1.
    pub fn fraction(self, now: Instant) -> f32 {
        let total = self.total.get().as_secs_f32();

        if total <= 0.0 {
            0.0
        } else {
            (1.0 - self.remaining(now).as_secs_f32() / total).clamp(0.0, 1.0)
        }
    }

    /// Starts, or stops and keeps what is left.
    pub fn toggle(self, now: Instant) {
        if self.running() {
            self.pause(now);
        } else {
            self.start(now);
        }
    }

    pub fn start(self, now: Instant) {
        let mut left = if self.finished() {
            self.duration.get()
        } else {
            self.remaining(now)
        };

        if left.is_zero() {
            left = self.duration.get();
        }

        // A fresh start (from idle or finished) counts down over its preset;
        // resuming keeps the length the countdown already had.
        if !self.engaged.get() || self.finished() {
            self.total.set(left);
        }

        self.finished.set(false);
        self.engaged.set(true);
        self.paused.set(left);
        self.ends_at.set(now.checked_add(left));
    }

    pub fn pause(self, now: Instant) {
        if self.running() {
            self.paused.set(self.remaining(now));
            self.ends_at.set(None);
        }
    }

    pub fn reset(self) {
        self.ends_at.set(None);
        self.paused.set(Duration::ZERO);
        self.engaged.set(false);
        self.finished.set(false);
        self.total.set(self.duration.get());
    }

    /// Sets a new length and goes back to it.
    pub fn set_minutes(self, minutes: u64) {
        self.duration.set(Duration::from_secs(minutes * 60));
        self.reset();
    }

    /// Adds a minute to a running or a stopped timer.
    pub fn add(self, by: Duration) {
        match self.ends_at.get() {
            Some(end) => self.ends_at.set(end.checked_add(by)),
            None if self.engaged.get() => {
                let left = self.paused.get().saturating_add(by);
                self.paused.set(left);
            }
            None => {
                let duration = self.duration.get().saturating_add(by);
                self.duration.set(duration);
                self.total.set(duration);
            }
        }

        // Time added while it runs or is stopped stretches the countdown's
        // length too, so the progress bar keeps filling forward.
        if self.engaged.get() {
            let total = self.total.get().saturating_add(by);
            self.total.set(total);
        }

        self.finished.set(false);
    }

    /// Called on every tick: a timer that has passed its end stops there.
    pub fn check(self, now: Instant) {
        let ended = self.ends_at.get_untracked().is_some_and(|end| now >= end);

        if ended {
            self.ends_at.set(None);
            self.paused.set(Duration::ZERO);
            self.engaged.set(true);
            self.finished.set(true);
        }
    }

    /// Announces a timer that has just reached zero.
    pub fn announce() {
        log::info!("the timer finished");
        alarms::notify(&t!("tempo-timer-finished"), "00:00");
    }

    pub fn act(self, action: &str) {
        match action {
            TOGGLE => self.toggle(Instant::now()),
            RESET => self.reset(),
            ADD_MINUTE => self.add(Duration::from_secs(60)),
            _ => {}
        }
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

pub fn view(clock: Clock) -> Container {
    let t = ui();
    let timer = clock.timer;
    let tick = clock.tick;
    let now = move || tick.get();

    let left = create_memo(move || duration::clock(timer.remaining(now())));
    let running = create_memo(move || timer.running());
    let fraction = create_memo(move || timer.fraction(now()));
    let minutes = create_memo(move || timer.duration().as_secs() / 60);

    let start = move || {
        button::button()
            .label(move || {
                if running.get() {
                    t!("tempo-pause")
                } else {
                    t!("tempo-start")
                }
            })
            .icon(move || {
                Some(if running.get() {
                    Icon::Pause
                } else {
                    Icon::Play
                })
            })
            .variant(Variant::Accent)
            .size(Size::Md)
            .on_click(move || timer.toggle(Instant::now()))
    };

    container()
        .width(fill())
        .padding([size::S2.px(), size::S4.px(), size::S4.px(), size::S4.px()])
        .layout(
            Flex::column()
                .spacing(size::S4.px())
                .cross_alignment(CrossAlignment::Center),
        )
        .child(
            roll::rolling(left)
                .look(ty::DISPLAY_MEDIUM.em())
                .color(t.colors.text),
        )
        .child(
            container()
                .width(size::S40.px())
                .child(progress_bar(fraction).color(t.colors.accent)),
        )
        .child(
            container().layout(centered_row(size::S1.px())).children(
                PRESETS
                    .into_iter()
                    .map(|preset| {
                        chip_button(t!("tempo-minutes", n = preset))
                            .active(move || minutes.get() == preset)
                            .on_click(move || timer.set_minutes(preset))
                    })
                    .collect::<Vec<_>>(),
            ),
        )
        .child(
            container()
                .layout(centered_row(size::S2.px()))
                .child(start())
                .child(
                    icon_button(Icon::Refresh)
                        .size(Size::Md)
                        .fill(Fill::Ghost)
                        .on_click(move || timer.reset()),
                )
                .child(
                    chip_button(t!("tempo-add-minute"))
                        .icon(Some(Icon::Plus))
                        .on_click(move || timer.add(Duration::from_secs(60))),
                ),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_timer_is_idle_with_its_length() {
        let timer = Timer::new();
        let now = Instant::now();

        assert_eq!(timer.state(), State::Idle);
        assert_eq!(timer.remaining(now), Duration::from_secs(300));
        assert_eq!(timer.fraction(now), 0.0);
    }

    #[test]
    fn a_started_timer_counts_down_and_stops_at_zero() {
        let timer = Timer::new();
        let start = Instant::now();

        timer.start(start);
        assert_eq!(timer.state(), State::Running);
        assert_eq!(
            timer.remaining(start + Duration::from_secs(10)),
            Duration::from_secs(290)
        );
        assert_eq!(
            timer.remaining(start + Duration::from_secs(300)),
            Duration::ZERO
        );

        timer.check(start + Duration::from_secs(300));
        assert_eq!(timer.state(), State::Finished);
        assert_eq!(
            timer.remaining(start + Duration::from_secs(400)),
            Duration::ZERO
        );
    }

    #[test]
    fn a_paused_timer_keeps_what_is_left() {
        let timer = Timer::new();
        let start = Instant::now();

        timer.start(start);
        timer.pause(start + Duration::from_secs(30));

        assert_eq!(timer.state(), State::Paused);
        assert_eq!(
            timer.remaining(start + Duration::from_secs(999)),
            Duration::from_secs(270)
        );

        timer.toggle(start + Duration::from_secs(1000));
        assert_eq!(timer.state(), State::Running);
        assert_eq!(
            timer.remaining(start + Duration::from_secs(1000)),
            Duration::from_secs(270)
        );
    }

    #[test]
    fn a_finished_timer_starts_over() {
        let timer = Timer::new();
        let start = Instant::now();

        timer.start(start);
        timer.check(start + Duration::from_secs(300));
        timer.toggle(start + Duration::from_secs(301));

        assert_eq!(timer.state(), State::Running);
        assert_eq!(
            timer.remaining(start + Duration::from_secs(301)),
            Duration::from_secs(300)
        );
    }

    #[test]
    fn a_minute_added_to_a_running_timer_keeps_its_end() {
        let timer = Timer::new();
        let start = Instant::now();

        timer.start(start);
        timer.add(Duration::from_secs(60));

        assert_eq!(timer.remaining(start), Duration::from_secs(360));
    }

    #[test]
    fn adding_time_keeps_the_progress_bar_a_real_progress() {
        let timer = Timer::new();
        let start = Instant::now();

        timer.start(start);
        let before = timer.fraction(start + Duration::from_secs(150));
        assert!((before - 0.5).abs() < 1e-6, "{before}");

        // The elapsed time is unchanged, the total grew, so the fraction is
        // elapsed/total: lower, but never negative and never clamped to 0.
        timer.add(Duration::from_secs(60));
        let after = timer.fraction(start + Duration::from_secs(150));
        assert!((after - 150.0 / 360.0).abs() < 1e-6, "{after}");

        // Even with a great deal added, the fraction stays a real progress.
        timer.add(Duration::from_secs(300));
        let piled = timer.fraction(start + Duration::from_secs(150));
        assert!((0.0..=1.0).contains(&piled), "{piled}");
        assert!(piled > 0.0, "the bar must not fall back to empty: {piled}");
    }

    #[test]
    fn a_minute_added_to_a_stopped_timer_keeps_it_stopped() {
        let timer = Timer::new();
        let start = Instant::now();

        timer.add(Duration::from_secs(60));
        assert_eq!(timer.state(), State::Idle);
        assert_eq!(timer.remaining(start), Duration::from_secs(360));
    }

    #[test]
    fn a_new_length_goes_back_to_it() {
        let timer = Timer::new();
        let start = Instant::now();

        timer.start(start);
        timer.set_minutes(10);

        assert_eq!(timer.state(), State::Idle);
        assert_eq!(timer.remaining(start), Duration::from_secs(600));
    }

    #[test]
    fn the_fraction_follows_the_countdown() {
        let timer = Timer::new();
        let start = Instant::now();

        timer.start(start);
        assert_eq!(timer.fraction(start), 0.0);

        let half = timer.fraction(start + Duration::from_secs(150));
        assert!((half - 0.5).abs() < f32::EPSILON, "{half}");

        assert_eq!(timer.fraction(start + Duration::from_secs(300)), 1.0);
    }
}
