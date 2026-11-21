//! The stopwatch: elapsed time, with laps.

use std::time::{Duration, Instant};

use ashell_i18n::t;
use guido::prelude::*;
use styletto::components::button::{self, Fill, Variant, chip_button, icon_button};
use styletto::foundations::control::Size;
use styletto::foundations::tokens::size;
use styletto::foundations::typography as ty;
use styletto::primitives::text::typed;
use styletto::{Icon, centered_row, ui};

use super::clock::Clock;
use super::duration;

pub const TOGGLE: &str = "toggle_stopwatch";
pub const RESET: &str = "reset_stopwatch";
pub const LAP: &str = "lap_stopwatch";

pub const ACTIONS: &[&str] = &[TOGGLE, RESET, LAP];

/// A stopwatch.
///
/// `accumulated` is what was counted before the last start; while it runs,
/// `started_at` says from when, so the display is right without a tick.
#[derive(Clone, Copy)]
pub struct Stopwatch {
    accumulated: RwSignal<Duration>,
    started_at: RwSignal<Option<Instant>>,
    /// The elapsed time at each lap, in order.
    laps: RwSignal<Vec<Duration>>,
}

impl Stopwatch {
    pub fn new() -> Self {
        Self {
            accumulated: create_signal(Duration::ZERO),
            started_at: create_signal(None),
            laps: create_signal(Vec::new()),
        }
    }

    /// A stopwatch at rest, for pictures of the module.
    #[cfg(test)]
    pub fn fixed() -> Self {
        Self::new()
    }

    pub fn running(self) -> bool {
        self.started_at.get().is_some()
    }

    /// What it has counted at `now`.
    pub fn elapsed(self, now: Instant) -> Duration {
        let running = self
            .started_at
            .get()
            .map(|start| now.saturating_duration_since(start))
            .unwrap_or_default();

        self.accumulated.get() + running
    }

    /// The elapsed time at each lap, in order.
    pub fn laps(self) -> Vec<Duration> {
        self.laps.get()
    }

    pub fn toggle(self, now: Instant) {
        if self.running() {
            self.pause(now);
        } else {
            self.start(now);
        }
    }

    pub fn start(self, now: Instant) {
        if !self.running() {
            self.started_at.set(Some(now));
        }
    }

    pub fn pause(self, now: Instant) {
        if self.running() {
            self.accumulated.set(self.elapsed(now));
            self.started_at.set(None);
        }
    }

    pub fn reset(self) {
        self.accumulated.set(Duration::ZERO);
        self.started_at.set(None);
        self.laps.set(Vec::new());
    }

    /// Records the elapsed time, if it is running.
    pub fn lap(self, now: Instant) {
        if self.running() {
            self.laps.update(|laps| laps.push(self.elapsed(now)));
        }
    }

    pub fn act(self, action: &str) {
        match action {
            TOGGLE => self.toggle(Instant::now()),
            RESET => self.reset(),
            LAP => self.lap(Instant::now()),
            _ => {}
        }
    }
}

impl Default for Stopwatch {
    fn default() -> Self {
        Self::new()
    }
}

pub fn view(clock: Clock) -> Container {
    let t = ui();
    let stopwatch = clock.stopwatch;
    let tick = clock.tick;
    let now = move || tick.get();

    let elapsed = create_memo(move || duration::stopwatch(stopwatch.elapsed(now())));
    let running = create_memo(move || stopwatch.running());

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
            .on_click(move || stopwatch.toggle(Instant::now()))
    };

    let controls = move || {
        container()
            .layout(centered_row(size::S2.px()))
            .child(start())
            .child(move || {
                running.get().then(|| {
                    chip_button(t!("tempo-lap"))
                        .icon(Some(Icon::Plus))
                        .on_click(move || stopwatch.lap(Instant::now()))
                })
            })
            .child(
                icon_button(Icon::Refresh)
                    .size(Size::Md)
                    .fill(Fill::Ghost)
                    .on_click(move || stopwatch.reset()),
            )
    };

    container()
        .width(fill())
        .padding([size::S2.px(), size::S4.px(), size::S4.px(), size::S4.px()])
        .layout(Flex::column().spacing(size::S4.px()))
        .child(
            container()
                .width(fill())
                .layout(Flex::column().spacing(size::S3.px()))
                .child(
                    typed(elapsed, ty::DISPLAY_MEDIUM.em().tabular())
                        .color(t.colors.text)
                        .nowrap(),
                )
                .child(controls()),
        )
        .child(laps(stopwatch))
}

/// The laps, the newest first, with the split and the total.
fn laps(stopwatch: Stopwatch) -> Container {
    let t = ui();

    let rows = create_memo(move || {
        let laps = stopwatch.laps();
        let mut previous = Duration::ZERO;
        let mut rows: Vec<(usize, Duration, Duration)> = laps
            .iter()
            .enumerate()
            .map(|(index, total)| {
                let split = total.saturating_sub(previous);
                previous = *total;
                (index + 1, split, *total)
            })
            .collect();

        rows.reverse();
        rows
    });

    container()
        .width(fill())
        .layout(Flex::column().spacing(size::S1.px()))
        .children(move || {
            rows.get()
                .into_iter()
                .map(|(index, split, total)| {
                    container()
                        .width(fill())
                        .padding([size::S1.px(), size::S3.px()])
                        .layout(
                            Flex::row()
                                .main_alignment(MainAlignment::SpaceBetween)
                                .cross_alignment(CrossAlignment::Center),
                        )
                        .child(
                            typed(t!("tempo-lap-n", n = index), ty::LABEL_SMALL)
                                .color(t.colors.sub)
                                .nowrap(),
                        )
                        .child(
                            container()
                                .layout(centered_row(size::S2.px()))
                                .child(
                                    typed(duration::stopwatch(split), ty::LABEL_SMALL.tabular())
                                        .color(t.colors.text)
                                        .nowrap(),
                                )
                                .child(
                                    typed(duration::stopwatch(total), ty::LABEL_XSMALL.tabular())
                                        .color(t.colors.dim)
                                        .nowrap(),
                                ),
                        )
                })
                .collect::<Vec<_>>()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_stopwatch_is_stopped_at_zero() {
        let stopwatch = Stopwatch::new();
        let now = Instant::now();

        assert!(!stopwatch.running());
        assert_eq!(stopwatch.elapsed(now), Duration::ZERO);
        assert!(stopwatch.laps().is_empty());
    }

    #[test]
    fn a_started_stopwatch_counts_from_its_start() {
        let stopwatch = Stopwatch::new();
        let start = Instant::now();

        stopwatch.start(start);
        assert!(stopwatch.running());
        assert_eq!(
            stopwatch.elapsed(start + Duration::from_millis(1234)),
            Duration::from_millis(1234)
        );
    }

    #[test]
    fn a_paused_stopwatch_keeps_what_it_counted() {
        let stopwatch = Stopwatch::new();
        let start = Instant::now();

        stopwatch.start(start);
        stopwatch.pause(start + Duration::from_secs(2));

        assert!(!stopwatch.running());
        assert_eq!(
            stopwatch.elapsed(start + Duration::from_secs(99)),
            Duration::from_secs(2)
        );

        stopwatch.start(start + Duration::from_secs(100));
        assert_eq!(
            stopwatch.elapsed(start + Duration::from_secs(103)),
            Duration::from_secs(5)
        );
    }

    #[test]
    fn a_reset_forgets_the_time_and_the_laps() {
        let stopwatch = Stopwatch::new();
        let start = Instant::now();

        stopwatch.start(start);
        stopwatch.lap(start + Duration::from_secs(1));
        stopwatch.reset();

        assert!(!stopwatch.running());
        assert_eq!(stopwatch.elapsed(start), Duration::ZERO);
        assert!(stopwatch.laps().is_empty());
    }

    #[test]
    fn a_lap_keeps_the_elapsed_time() {
        let stopwatch = Stopwatch::new();
        let start = Instant::now();

        stopwatch.start(start);
        stopwatch.lap(start + Duration::from_secs(3));
        stopwatch.lap(start + Duration::from_secs(7));

        assert_eq!(
            stopwatch.laps(),
            [Duration::from_secs(3), Duration::from_secs(7)]
        );
    }
}
