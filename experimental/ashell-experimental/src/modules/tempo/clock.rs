//! The calendar panel: its tabs, and what each one shows.

use std::time::{Duration, Instant};

use ashell_i18n::t;
use guido::prelude::*;
use styletto::foundations::control::Size;
use styletto::foundations::tokens::{icon as icon_box, radius, size};
use styletto::foundations::typography as ty;
use styletto::primitives::text::typed;
use styletto::{Icon, centered_row, icon, ui};

use super::{Tempo, alarms, calendar, stopwatch, timer};

/// The panel's width, as wide as the calendar it started as.
pub const WIDTH: f32 = calendar::WIDTH;

/// What the calendar panel shows: the calendar it always had, and the three
/// parts of a clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Calendar,
    Alarms,
    Timer,
    Stopwatch,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Calendar, Tab::Alarms, Tab::Timer, Tab::Stopwatch];

    fn icon(self) -> Icon {
        match self {
            Tab::Calendar => Icon::Calendar,
            Tab::Alarms => Icon::Bell,
            Tab::Timer => Icon::Gauge,
            Tab::Stopwatch => Icon::Refresh,
        }
    }

    fn label(self) -> String {
        match self {
            Tab::Calendar => t!("tempo-tab-calendar"),
            Tab::Alarms => t!("tempo-tab-alarms"),
            Tab::Timer => t!("tempo-tab-timer"),
            Tab::Stopwatch => t!("tempo-tab-stopwatch"),
        }
    }
}

/// How often the clock's running parts are drawn.
const TICK: Duration = Duration::from_millis(50);

/// How often the stopwatch is drawn while it runs: it counts to the
/// hundredth.
const STOPWATCH_TICK: Duration = Duration::from_millis(10);

/// The panel's state: which tab is shown, and the parts that run.
#[derive(Clone, Copy)]
pub struct Clock {
    pub tab: RwSignal<Tab>,
    /// The moment the running parts were last drawn at.
    pub tick: RwSignal<Instant>,
    pub alarms: alarms::Alarms,
    pub timer: timer::Timer,
    pub stopwatch: stopwatch::Stopwatch,
}

impl Clock {
    pub fn new() -> Self {
        let timer = timer::Timer::new();
        let stopwatch = stopwatch::Stopwatch::new();
        let running = create_memo(move || timer.running() || stopwatch.running());
        let clock = Self {
            tab: create_signal(Tab::Calendar),
            tick: create_signal(Instant::now()),
            alarms: alarms::Alarms::new(),
            timer,
            stopwatch,
        };

        // Ticks only while something runs: a stopped timer or stopwatch
        // draws nothing new. The stopwatch counts to the hundredth, so it is
        // drawn ten times as often as the timer.
        create_effect(move || {
            if !running.get() {
                return;
            }

            let period = if stopwatch.running() {
                STOPWATCH_TICK
            } else {
                TICK
            };

            set_interval(period, move || clock.beat(Instant::now()));
        });

        clock
    }

    /// One tick of the running parts.
    pub fn beat(self, now: Instant) {
        self.tick.set(now);

        let was_finished = self.timer.finished_untracked();
        self.timer.check(now);

        if !was_finished && self.timer.finished_untracked() {
            timer::Timer::announce();
        }
    }

    /// The panel at rest, for pictures of the module.
    #[cfg(test)]
    pub fn fixed() -> Self {
        Self {
            tab: create_signal(Tab::Calendar),
            tick: create_signal(Instant::now()),
            alarms: alarms::Alarms::fixed(),
            timer: timer::Timer::fixed(),
            stopwatch: stopwatch::Stopwatch::fixed(),
        }
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::new()
    }
}

pub fn view(tempo: Tempo) -> Container {
    let tab = tempo.clock.tab;

    container()
        .width(WIDTH)
        .layout(Flex::column())
        .child(tabs(tab))
        .child(content(tempo, tab))
}

/// The tab row, each tab a pill on the panel's own surface.
fn tabs(active: RwSignal<Tab>) -> Container {
    let t = ui();

    container()
        .width(fill())
        .padding([size::S3.px(), size::S4.px(), size::S0_5.px(), size::S4.px()])
        .layout(centered_row(size::S1.px()))
        .children(Tab::ALL.into_iter().map(|tab| {
            let on = move || active.get() == tab;
            let ink = move || {
                if on() { t.colors.text } else { t.colors.sub }
            };

            container()
                .height(Size::Sm.height())
                .padding([0.0, size::S3.px()])
                .corners(radius::pill(Size::Sm.height()))
                .background(move || {
                    if on() {
                        t.colors.surface2
                    } else {
                        Color::TRANSPARENT
                    }
                })
                .when_hovered(move |s| {
                    s.background(move || {
                        if on() {
                            t.colors.surface2
                        } else {
                            t.colors.surface
                        }
                    })
                })
                .layout(centered_row(size::S1.px()))
                .child(icon(tab.icon(), icon_box::XSMALL.px(), ink))
                .child(typed(tab.label(), ty::LABEL_SMALL).color(ink).nowrap())
                .on_click(move || active.set(tab))
        }))
}

fn content(tempo: Tempo, tab: RwSignal<Tab>) -> Container {
    container().width(fill()).child(move || match tab.get() {
        Tab::Calendar => calendar::view(tempo),
        Tab::Alarms => alarms::view(tempo.clock),
        Tab::Timer => timer::view(tempo.clock),
        Tab::Stopwatch => stopwatch::view(tempo.clock),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn a_timer_that_reaches_zero_is_finished_and_announced_once() {
        let clock = Clock::fixed();
        let start = Instant::now();

        clock.timer.start(start);
        assert!(!clock.timer.finished_untracked());

        clock.beat(start + Duration::from_secs(301));
        assert!(clock.timer.finished_untracked());

        // A second beat does not announce again.
        clock.beat(start + Duration::from_secs(302));
        assert!(clock.timer.finished_untracked());
    }
}
