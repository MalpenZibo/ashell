//! The alarms: a time of day, the days it repeats on, and what happens when
//! it rings.

use std::io::Write;
use std::path::Path;
#[cfg(not(test))]
use std::path::PathBuf;
use std::process::{Command, Stdio};
#[cfg(not(test))]
use std::time::Duration;

use ashell_i18n::{chrono_locale, t};
use chrono::{
    Datelike, Duration as ChronoDuration, Local, NaiveDate, NaiveDateTime, Timelike, Weekday,
};
use guido::prelude::*;
use serde::{Deserialize, Serialize};
use styletto::components::button::{self, Fill, Variant, chip_button, icon_button};
use styletto::components::list;
use styletto::components::toggle::toggle;
use styletto::foundations::control::Size;
use styletto::foundations::tokens::{radius, size};
use styletto::foundations::typography as ty;
use styletto::primitives::text::{caption, typed};
use styletto::{Icon, centered_row, icon, ui};

use super::clock::Clock;

pub const DISMISS: &str = "dismiss_alarm";
pub const SNOOZE: &str = "snooze_alarm";

pub const ACTIONS: &[&str] = &[DISMISS, SNOOZE];

/// How long a snooze puts the alarm off.
const SNOOZE_MINUTES: u64 = 5;
/// The freedesktop sound, used only when no player can take the chime.
const SOUND: &str = "/usr/share/sounds/freedesktop/stereo/alarm-clock-elapsed.oga";
/// The days, Monday first, as the week is written.
const WEEK: [Weekday; 7] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];

/// One alarm: a time of day, the days it rings on, and whether it is on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Alarm {
    pub id: u64,
    pub hour: u32,
    pub minute: u32,
    /// Monday first. All false is a one-shot alarm, at the next time.
    pub days: [bool; 7],
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub label: String,
}

impl Alarm {
    /// The next time this alarm rings after `now`, if it ever does.
    pub fn next_after(&self, now: NaiveDateTime) -> Option<NaiveDateTime> {
        let repeats = self.days.iter().any(|day| *day);

        for offset in 0..8 {
            let date = now.date() + ChronoDuration::days(offset);
            let Some(at) = date.and_hms_opt(self.hour, self.minute, 0) else {
                continue;
            };

            if at <= now {
                continue;
            }

            let weekday = date.weekday().num_days_from_monday() as usize;
            if !repeats || self.days[weekday] {
                return Some(at);
            }
        }

        None
    }

    /// The time it is set for, as the panel writes it.
    pub fn time(&self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }

    /// How it repeats, in few words.
    pub fn repeat(&self) -> String {
        let on: Vec<usize> = self
            .days
            .iter()
            .enumerate()
            .filter_map(|(index, on)| on.then_some(index))
            .collect();

        match on.as_slice() {
            [] => t!("tempo-repeat-never"),
            [0, 1, 2, 3, 4, 5, 6] => t!("tempo-repeat-everyday"),
            [0, 1, 2, 3, 4] => t!("tempo-repeat-weekdays"),
            [5, 6] => t!("tempo-repeat-weekends"),
            days => days
                .iter()
                .map(|day| day_name(WEEK[*day]))
                .collect::<Vec<_>>()
                .join(" "),
        }
    }
}

/// The one-letter name of a day.
fn day_name(day: Weekday) -> String {
    NaiveDate::from_isoywd_opt(2000, 20, day)
        .expect("a week of 2000")
        .format_localized("%a", chrono_locale())
        .to_string()
        .chars()
        .next()
        .map(String::from)
        .unwrap_or_default()
}

/// The alarms, and the one that is ringing.
#[derive(Clone, Copy)]
pub struct Alarms {
    alarms: RwSignal<Vec<Alarm>>,
    ringing: RwSignal<Option<u64>>,
    /// The alarms that are due and waiting for the one ringing to be answered.
    queue: RwSignal<Vec<u64>>,
    /// When each alarm last rang, so a minute rings once.
    fired: RwSignal<Vec<(u64, NaiveDateTime)>>,
    /// The alarms put off, and the moment each rings again.
    snoozed: RwSignal<Vec<(u64, NaiveDateTime)>>,
    last_id: RwSignal<u64>,
}

impl Alarms {
    pub fn new() -> Self {
        // Tests draw the module, but must not read or write the alarms of
        // whoever is running them.
        #[cfg(test)]
        {
            Self::fixed()
        }

        #[cfg(not(test))]
        {
            let (stored, readable) = match load() {
                Read::Alarms(stored) => (stored, true),
                Read::None => (Vec::new(), true),
                Read::Unreadable => (Vec::new(), false),
            };

            let last_id = stored.iter().map(|alarm| alarm.id).max().unwrap_or(0) + 1;
            let state = Self {
                alarms: create_signal(stored),
                ringing: create_signal(None),
                queue: create_signal(Vec::new()),
                fired: create_signal(Vec::new()),
                snoozed: create_signal(Vec::new()),
                last_id: create_signal(last_id),
            };

            // The alarms are kept on disk as they change, but a file that
            // could not be read is left alone: writing what was read would
            // write nothing.
            if readable {
                create_effect(move || save(&state.alarms.get()));
            } else {
                log::warn!("the alarms are not kept until the file can be read");
            }

            // A tick a second is enough for a time of day.
            set_interval(Duration::from_secs(1), move || {
                state.check(Local::now().naive_local());
            });

            state
        }
    }

    /// No alarms, and no tick: for pictures of the module.
    #[cfg(test)]
    pub fn fixed() -> Self {
        Self {
            alarms: create_signal(Vec::new()),
            ringing: create_signal(None),
            queue: create_signal(Vec::new()),
            fired: create_signal(Vec::new()),
            snoozed: create_signal(Vec::new()),
            last_id: create_signal(1),
        }
    }

    /// The alarms, in the order they were added.
    pub fn list(self) -> Vec<Alarm> {
        self.alarms.get()
    }

    /// The next alarm to ring, and when.
    pub fn next(self, now: NaiveDateTime) -> Option<(Alarm, NaiveDateTime)> {
        self.alarms
            .get()
            .into_iter()
            .filter(|alarm| alarm.enabled)
            .filter_map(|alarm| alarm.next_after(now).map(|at| (alarm, at)))
            .min_by_key(|(_, at)| *at)
    }

    /// The alarm that is ringing, if one is.
    pub fn ringing(self) -> Option<Alarm> {
        let ringing = self.ringing.get();
        self.alarms
            .get()
            .into_iter()
            .find(|alarm| Some(alarm.id) == ringing)
    }

    /// A new id, for an alarm about to be added.
    pub fn new_id(self) -> u64 {
        let id = self.last_id.get_untracked();
        self.last_id.set(id + 1);
        id
    }

    pub fn add(self, alarm: Alarm) {
        self.alarms.update(|alarms| alarms.push(alarm));
    }

    pub fn remove(self, id: u64) {
        self.alarms
            .update(|alarms| alarms.retain(|alarm| alarm.id != id));
        self.ringing.update(|ringing| {
            if *ringing == Some(id) {
                *ringing = None;
            }
        });
        self.queue
            .update(|queue| queue.retain(|queued| *queued != id));
        self.snoozed
            .update(|snoozed| snoozed.retain(|(snoozed, _)| *snoozed != id));

        self.answer();
    }

    pub fn set_enabled(self, id: u64, enabled: bool) {
        self.alarms.update(|alarms| {
            if let Some(alarm) = alarms.iter_mut().find(|alarm| alarm.id == id) {
                alarm.enabled = enabled;
            }
        });

        if !enabled {
            self.queue
                .update(|queue| queue.retain(|queued| *queued != id));
            self.snoozed
                .update(|snoozed| snoozed.retain(|(snoozed, _)| *snoozed != id));
        }
    }

    pub fn dismiss(self) {
        self.ringing.set(None);
        self.answer();
    }

    /// Puts the ringing alarm off for a few minutes.
    pub fn snooze(self) {
        self.snooze_at(Local::now().naive_local());
    }

    fn snooze_at(self, now: NaiveDateTime) {
        let Some(id) = self.ringing.get_untracked() else {
            return;
        };

        let at = now + ChronoDuration::minutes(SNOOZE_MINUTES as i64);

        // A second snooze must not overwrite the first: each waits its turn.
        self.snoozed
            .update(|snoozed| snoozed.retain(|(snoozed, _)| *snoozed != id));
        self.snoozed.update(|snoozed| snoozed.push((id, at)));
        self.ringing.set(None);
        self.answer();
    }

    /// Rings the alarm that is due at `now` and has not rung this minute.
    pub fn check(self, now: NaiveDateTime) {
        // A day of minutes is enough to keep a minute from ringing twice.
        self.fired.update(|fired| {
            fired.retain(|(_, at)| at.date() >= now.date() - ChronoDuration::days(1));
        });

        // The alarms put off ring again at their own moment. Each is taken
        // from the list once it is due, and the earliest rings first; the
        // others wait their turn behind whatever is already ringing.
        let mut due: Vec<(u64, NaiveDateTime)> = self.snoozed.with_untracked(|snoozed| {
            snoozed
                .iter()
                .filter(|(_, at)| *at <= now)
                .copied()
                .collect()
        });

        if !due.is_empty() {
            due.sort_by_key(|(_, at)| *at);

            let ids: Vec<u64> = due.iter().map(|(id, _)| *id).collect();
            self.snoozed
                .update(|snoozed| snoozed.retain(|(id, at)| !(*at <= now && ids.contains(id))));
            for id in &ids {
                self.fired.update(|fired| fired.push((*id, now)));
            }
            for id in ids {
                self.ring(id);
            }
            return;
        }

        let fired = self.fired.get_untracked();
        let due: Vec<u64> = self
            .alarms
            .get_untracked()
            .into_iter()
            .filter(|alarm| {
                alarm.enabled
                    && alarm.hour == now.hour()
                    && alarm.minute == now.minute()
                    && (alarm.days.iter().all(|day| !day)
                        || alarm.days[now.weekday().num_days_from_monday() as usize])
                    && !fired.iter().any(|(id, at)| {
                        *id == alarm.id
                            && at.date() == now.date()
                            && at.hour() == now.hour()
                            && at.minute() == now.minute()
                    })
            })
            .map(|alarm| alarm.id)
            .collect();

        if due.is_empty() {
            return;
        }

        for id in &due {
            self.fired.update(|fired| fired.push((*id, now)));
        }

        // A one-shot alarm is spent, whether it rings now or waits.
        for id in &due {
            let one_shot = self.alarms.with_untracked(|alarms| {
                alarms
                    .iter()
                    .any(|alarm| alarm.id == *id && alarm.days.iter().all(|day| !day))
            });

            if one_shot {
                self.set_enabled(*id, false);
            }
        }

        // The first rings; the others wait for it to be answered.
        let mut due = due.into_iter();
        let Some(first) = due.next() else {
            return;
        };

        if self.ringing.get_untracked().is_some() {
            // Another alarm is ringing: queue them in due order, the first
            // ahead of the rest so it is answered first.
            self.queue.update(|queue| {
                for id in std::iter::once(first).chain(due) {
                    if !queue.contains(&id) {
                        queue.push(id);
                    }
                }
            });
        } else {
            // Nothing is ringing: the first rings now, the rest wait behind
            // it. The first is not queued, so it cannot ring twice.
            self.queue.update(|queue| {
                for id in due {
                    if !queue.contains(&id) {
                        queue.push(id);
                    }
                }
            });

            self.ring(first);
        }
    }

    /// Rings `id`, unless another alarm is already ringing.
    fn ring(self, id: u64) {
        if self.ringing.get_untracked().is_some() {
            self.queue.update(|queue| {
                if !queue.contains(&id) {
                    queue.push(id);
                }
            });
            return;
        }

        let Some(alarm) = self
            .alarms
            .with_untracked(|alarms| alarms.iter().find(|alarm| alarm.id == id).cloned())
        else {
            return;
        };

        self.ringing.set(Some(id));
        alert(&alarm);
    }

    /// Rings the next alarm waiting, if none is ringing.
    fn answer(self) {
        if self.ringing.get_untracked().is_some() {
            return;
        }

        let Some(id) = self.queue.with_untracked(|queue| queue.first().copied()) else {
            return;
        };

        self.queue
            .update(|queue| queue.retain(|queued| *queued != id));

        let now = Local::now().naive_local();
        self.fired.update(|fired| fired.push((id, now)));
        self.ring(id);
    }

    pub fn act(self, action: &str) {
        match action {
            DISMISS => self.dismiss(),
            SNOOZE => self.snooze(),
            _ => {}
        }
    }
}

impl Default for Alarms {
    fn default() -> Self {
        Self::new()
    }
}

/// The file the alarms are kept in, under the state directory.
#[cfg(not(test))]
fn path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .filter(|home| !home.is_empty())
        .map(PathBuf::from)
        .or_else(|| Some(std::env::home_dir()?.join(".local/state")))?;

    Some(base.join("ashell").join("alarms.toml"))
}

#[derive(Default, Serialize, Deserialize)]
#[cfg(not(test))]
struct Stored {
    #[serde(default)]
    alarm: Vec<Alarm>,
}

/// What was found where the alarms are kept.
#[cfg(not(test))]
enum Read {
    /// The alarms that were read.
    Alarms(Vec<Alarm>),
    /// There is nothing kept yet.
    None,
    /// There is a file, but it could not be read or understood.
    Unreadable,
}

#[cfg(not(test))]
fn load() -> Read {
    let Some(path) = path() else {
        return Read::None;
    };

    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Read::None,
        Err(err) => {
            log::warn!("{}: the alarms were not read: {err}", path.display());
            return Read::Unreadable;
        }
    };

    match toml::from_str::<Stored>(&text) {
        Ok(stored) => Read::Alarms(stored.alarm),
        Err(err) => {
            // A file we cannot understand would otherwise silently disable
            // saving for the whole session. Move it aside so a fresh one is
            // written, and the user's old alarms are still there to recover.
            log::warn!("{}: the alarms were not understood: {err}", path.display());
            let aside = path.with_extension("toml.corrupt");
            if let Err(err) = std::fs::rename(&path, &aside) {
                log::warn!(
                    "{}: the unreadable alarms were not moved aside: {err}",
                    path.display()
                );
                return Read::Unreadable;
            }
            Read::None
        }
    }
}

#[cfg(not(test))]
fn save(alarms: &[Alarm]) {
    let Some(path) = path() else {
        return;
    };

    // An empty first run does not need a file.
    if alarms.is_empty() && !path.exists() {
        return;
    }

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let stored = Stored {
        alarm: alarms.to_vec(),
    };

    let text = match toml::to_string_pretty(&stored) {
        Ok(text) => text,
        Err(err) => {
            log::warn!("the alarms were not written: {err}");
            return;
        }
    };

    // Written beside the file and moved over it, so a crash cannot leave
    // half of the alarms. The scratch file is synced before the rename and
    // the directory after it, so a power loss cannot leave the rename
    // pointing at data that never reached the disk.
    let scratch = path.with_extension("toml.new");

    let written = std::fs::File::create(&scratch)
        .and_then(|mut file| {
            use std::io::Write as _;
            file.write_all(text.as_bytes())?;
            file.sync_all()
        })
        .is_ok();

    if !written {
        log::warn!("{}: the alarms were not saved", scratch.display());
        return;
    }

    if let Err(err) = std::fs::rename(&scratch, &path) {
        log::warn!("{}: the alarms were not saved: {err}", path.display());
        return;
    }

    // Make the rename itself durable.
    if let Some(parent) = path.parent()
        && let Ok(dir) = std::fs::File::open(parent)
    {
        let _ = dir.sync_all();
    }
}

/// Rings: a desktop notification, and a sound through the first player
/// there is.
fn alert(alarm: &Alarm) {
    let title = if alarm.label.is_empty() {
        t!("tempo-alarm")
    } else {
        alarm.label.clone()
    };

    notify(&title, &alarm.time());
}

/// A notification and a sound, for an alarm or a timer that is done.
pub(super) fn notify(title: &str, body: &str) {
    // Tests ring alarms to check them; they should stay quiet.
    if cfg!(test) {
        return;
    }

    let title = title.to_string();
    let body = body.to_string();

    // The sound must not wait for the notification daemon.
    std::thread::spawn(play_sound);
    std::thread::spawn(move || {
        let _ = Command::new("notify-send")
            .args(["-u", "critical", "-a", "ashell", &title, &body])
            .status();
    });
}

/// Plays the alarm sound through the first thing that works: PipeWire,
/// PulseAudio, the sound theme, the freedesktop file, then the terminal
/// bell.
fn play_sound() {
    // The chime never changes, so it is synthesized once, not on every ring.
    static CHIME: std::sync::LazyLock<Vec<u8>> = std::sync::LazyLock::new(chime_wav);
    let wav = &*CHIME;

    // PipeWire and PulseAudio both read a WAV from their standard input, so
    // the sound works without a sound file or a theme.
    if play_wav("pw-play", &["-"], wav) {
        log::debug!("played the alarm sound with pw-play");
        return;
    }

    if play_wav("paplay", &[], wav) {
        log::debug!("played the alarm sound with paplay");
        return;
    }

    if play_wav("aplay", &["-t", "wav", "-"], wav) {
        log::debug!("played the alarm sound with aplay");
        return;
    }

    let themed = Command::new("canberra-gtk-play")
        .args(["-i", "alarm-clock-elapsed"])
        .status()
        .is_ok_and(|status| status.success());

    if themed {
        return;
    }

    if Path::new(SOUND).exists() {
        for player in ["paplay", "aplay"] {
            let played = Command::new(player)
                .arg(SOUND)
                .status()
                .is_ok_and(|status| status.success());

            if played {
                return;
            }
        }
    }

    let _ = Command::new("sh").arg("-c").arg("printf '\\a'").status();
}

/// Writes `wav` to a player's standard input and waits for it.
fn play_wav(player: &str, args: &[&str], wav: &[u8]) -> bool {
    let Ok(mut child) = Command::new(player)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };

    let Some(mut stdin) = child.stdin.take() else {
        let _ = child.wait();
        return false;
    };

    let written = stdin.write_all(wav).is_ok();
    drop(stdin);

    written && child.wait().is_ok_and(|status| status.success())
}

/// A short two-tone chime, three times, as a mono 16-bit WAV.
fn chime_wav() -> Vec<u8> {
    const RATE: u32 = 44_100;

    let mut samples: Vec<i16> = Vec::new();

    for _ in 0..3 {
        for (frequency, millis) in [(880.0, 150), (660.0, 250)] {
            let count = RATE as usize * millis as usize / 1000;

            for index in 0..count {
                let t = index as f32 / RATE as f32;
                let length = count as f32 / RATE as f32;
                let envelope = (t / 0.01).min(1.0) * ((length - t) / 0.05).min(1.0);
                let tone = (2.0 * std::f32::consts::PI * frequency * t).sin();
                let sample = (0.35 * envelope * tone).clamp(-1.0, 1.0);

                samples.push((sample * i16::MAX as f32) as i16);
            }
        }

        // A breath between the chimes.
        samples.extend(std::iter::repeat_n(0, RATE as usize / 4));
    }

    let data_len = (samples.len() * 2) as u32;
    let mut wav = Vec::with_capacity(44 + data_len as usize);

    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&RATE.to_le_bytes());
    wav.extend_from_slice(&(RATE * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());

    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }

    wav
}

pub fn view(clock: Clock) -> Container {
    let alarms = clock.alarms;
    let editing = create_signal(false);

    container()
        .width(fill())
        .padding([size::S2.px(), size::S4.px(), size::S4.px(), size::S4.px()])
        .layout(Flex::column().spacing(size::S2.px()))
        .child(ringing(alarms))
        .child(list(alarms))
        .child(move || {
            (!editing.get()).then(|| {
                button::button()
                    .label(t!("tempo-add-alarm"))
                    .icon(Icon::Plus)
                    .variant(Variant::Accent)
                    .size(Size::Md)
                    .on_click(move || editing.set(true))
            })
        })
        .child(move || editing.get().then(|| editor(alarms, editing)))
}

/// The banner for a ringing alarm, with a way to put it off.
fn ringing(alarms: Alarms) -> Container {
    let t = ui();

    container().child(move || {
        let Some(alarm) = alarms.ringing() else {
            return container();
        };

        container()
            .width(fill())
            .padding(size::S3.px())
            .corners(radius::MEDIUM.corners())
            .gradient(LinearGradient::diagonal(
                t.colors.accent.with_alpha(0.32),
                t.colors.accent.with_alpha(0.06),
            ))
            .layout(Flex::column().spacing(size::S2.px()))
            .child(
                container()
                    .layout(centered_row(size::S2.px()))
                    .child(icon(Icon::Bell, 16.0, t.colors.text))
                    .child(
                        typed(
                            if alarm.label.is_empty() {
                                t!("tempo-alarm")
                            } else {
                                alarm.label.clone()
                            },
                            ty::TITLE_XSMALL.em(),
                        )
                        .color(t.colors.text)
                        .nowrap(),
                    ),
            )
            .child(caption(alarm.time()))
            .child(
                container()
                    .layout(centered_row(size::S2.px()))
                    .child(
                        chip_button(t!("tempo-snooze"))
                            .icon(Some(Icon::Moon))
                            .on_click(move || alarms.snooze()),
                    )
                    .child(
                        chip_button(t!("tempo-dismiss"))
                            .icon(Some(Icon::Check))
                            .on_click(move || alarms.dismiss()),
                    ),
            )
    })
}

/// The alarms, each with its time, its repeat, a switch and a way to delete
/// it; or an empty list.
fn list(alarms: Alarms) -> Container {
    container().child(move || {
        let list = alarms.list();

        if list.is_empty() {
            return container().child(list::empty(Icon::Bell, t!("tempo-no-alarms")));
        }

        container()
            .width(fill())
            .layout(Flex::column().spacing(size::S1.px()))
            .children(
                list.into_iter()
                    .map(|alarm| row(alarms, alarm))
                    .collect::<Vec<_>>(),
            )
    })
}

fn row(alarms: Alarms, alarm: Alarm) -> Container {
    let t = ui();
    let enabled = alarm.enabled;
    let time = alarm.time();
    let repeat = alarm.repeat();
    let id = alarm.id;

    container()
        .width(fill())
        .padding([size::S2.px(), size::S3.px()])
        .corners(radius::SMALL.corners())
        .background(Color::TRANSPARENT)
        .when_hovered(move |s| s.background(t.colors.surface))
        .layout(centered_row(size::S3.px()))
        .child(
            container()
                .width(fill())
                .layout(Flex::column().spacing(size::S0_5.px()))
                .child(
                    typed(time, ty::TITLE_SMALL.tabular())
                        .color(if enabled { t.colors.text } else { t.colors.dim })
                        .nowrap(),
                )
                .child(typed(repeat, ty::LABEL_SMALL).color(t.colors.sub).nowrap()),
        )
        .child(toggle(enabled).on_toggle(move |on| alarms.set_enabled(id, on)))
        .child(
            icon_button(Icon::X)
                .size(Size::Sm)
                .fill(Fill::Ghost)
                .on_click(move || alarms.remove(id)),
        )
}

/// The form for a new alarm: its time, its days, and a way to keep it.
fn editor(alarms: Alarms, editing: RwSignal<bool>) -> Container {
    let t = ui();

    // A new alarm is an hour from now, rounded to the next five minutes.
    let start = Local::now().naive_local() + ChronoDuration::hours(1);
    let minute = (start.minute() / 5) * 5;
    let hour = create_signal(start.hour());
    let minute = create_signal(minute);
    let days = create_signal([false; 7]);

    let step = move |hours: i64, minutes: i64| {
        let total =
            hour.get_untracked() as i64 * 60 + minute.get_untracked() as i64 + hours * 60 + minutes;
        let total = total.rem_euclid(24 * 60);
        hour.set((total / 60) as u32);
        minute.set((total % 60) as u32);
    };

    let time = create_memo(move || format!("{:02}:{:02}", hour.get(), minute.get()));

    container()
        .width(fill())
        .padding(size::S3.px())
        .corners(radius::MEDIUM.corners())
        .background(t.colors.surface)
        .layout(Flex::column().spacing(size::S2.px()))
        .child(
            container()
                .layout(centered_row(size::S2.px()))
                .child(
                    icon_button(Icon::ChevronDown)
                        .size(Size::Sm)
                        .fill(Fill::Ghost)
                        .on_click(move || step(0, -5)),
                )
                .child(
                    typed(time, ty::DISPLAY_SMALL.em().tabular())
                        .color(t.colors.text)
                        .nowrap(),
                )
                .child(
                    icon_button(Icon::ChevronUp)
                        .size(Size::Sm)
                        .fill(Fill::Ghost)
                        .on_click(move || step(0, 5)),
                )
                .child(
                    icon_button(Icon::ChevronLeft)
                        .size(Size::Sm)
                        .fill(Fill::Ghost)
                        .on_click(move || step(-1, 0)),
                )
                .child(
                    icon_button(Icon::ChevronRight)
                        .size(Size::Sm)
                        .fill(Fill::Ghost)
                        .on_click(move || step(1, 0)),
                ),
        )
        .child(
            container().layout(centered_row(size::S1.px())).children(
                (0..7)
                    .map(|index| {
                        chip_button(day_name(WEEK[index]))
                            .active(move || days.get()[index])
                            .on_click(move || days.update(|days| days[index] = !days[index]))
                    })
                    .collect::<Vec<_>>(),
            ),
        )
        .child(
            container()
                .layout(centered_row(size::S2.px()))
                .child(
                    button::button()
                        .label(t!("tempo-save"))
                        .icon(Icon::Check)
                        .variant(Variant::Accent)
                        .size(Size::Md)
                        .on_click(move || {
                            alarms.add(Alarm {
                                id: alarms.new_id(),
                                hour: hour.get_untracked(),
                                minute: minute.get_untracked(),
                                days: days.get_untracked(),
                                enabled: true,
                                label: String::new(),
                            });
                            editing.set(false);
                        }),
                )
                .child(
                    button::button()
                        .label(t!("tempo-cancel"))
                        .variant(Variant::Neutral)
                        .size(Size::Md)
                        .on_click(move || editing.set(false)),
                ),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(date: &str, time: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(&format!("{date}T{time}"), "%FT%R").unwrap()
    }

    fn alarm(hour: u32, minute: u32, days: [bool; 7]) -> Alarm {
        Alarm {
            id: 1,
            hour,
            minute,
            days,
            enabled: true,
            label: String::new(),
        }
    }

    #[test]
    fn a_one_shot_alarm_rings_at_its_next_time() {
        let alarm = alarm(7, 30, [false; 7]);

        assert_eq!(
            alarm.next_after(at("2026-11-21", "06:00")),
            Some(at("2026-11-21", "07:30"))
        );
        assert_eq!(
            alarm.next_after(at("2026-11-21", "08:00")),
            Some(at("2026-11-22", "07:30"))
        );
    }

    #[test]
    fn a_weekly_alarm_rings_only_on_its_days() {
        // 2026-11-21 is a Saturday.
        let alarm = alarm(7, 30, [true, false, false, false, false, false, false]);

        assert_eq!(
            alarm.next_after(at("2026-11-21", "06:00")),
            Some(at("2026-11-23", "07:30"))
        );
    }

    #[test]
    #[ignore = "plays the alarm sound"]
    fn the_chime_can_be_heard() {
        play_sound();
    }

    #[test]
    fn the_chime_is_a_wav() {
        let wav = chime_wav();

        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(&wav[36..40], b"data");

        let data_len = u32::from_le_bytes(wav[40..44].try_into().unwrap()) as usize;
        assert_eq!(wav.len(), 44 + data_len);
    }

    #[test]
    fn the_repeat_is_read_from_the_days() {
        crate::i18n::init_for("en-US", "en-US");

        assert_eq!(alarm(7, 0, [false; 7]).repeat(), "Never");
        assert_eq!(alarm(7, 0, [true; 7]).repeat(), "Every day");
        assert_eq!(
            alarm(7, 0, [true, true, true, true, true, false, false]).repeat(),
            "Weekdays"
        );
        assert_eq!(
            alarm(7, 0, [false, false, false, false, false, true, true]).repeat(),
            "Weekends"
        );
        assert_eq!(
            alarm(7, 0, [true, false, true, false, false, false, false]).repeat(),
            "M W"
        );
    }

    #[test]
    fn an_alarm_that_is_on_is_the_next_one() {
        // Build the state without a scope: the signals only.
        let state = Alarms::fixed();
        state.add(alarm(7, 30, [false; 7]));
        state.add(Alarm {
            id: 2,
            hour: 8,
            minute: 0,
            days: [false; 7],
            enabled: false,
            label: String::new(),
        });

        let (next, at_time) = state
            .next(at("2026-11-21", "06:00"))
            .expect("the first alarm");
        assert_eq!(next.id, 1);
        assert_eq!(at_time, at("2026-11-21", "07:30"));
    }

    #[test]
    fn a_ringing_alarm_is_the_one_that_is_due() {
        crate::i18n::init_for("en-US", "en-US");

        let state = Alarms::fixed();
        state.add(alarm(7, 30, [false; 7]));

        // 2026-11-21 is a Saturday; a one-shot rings whatever the day.
        state.check(at("2026-11-21", "07:30"));
        assert_eq!(state.ringing().map(|alarm| alarm.id), Some(1));

        // A one-shot is spent.
        assert!(!state.list()[0].enabled);

        // The same minute does not ring twice.
        state.dismiss();
        state.check(at("2026-11-21", "07:30"));
        assert!(state.ringing().is_none());
    }

    #[test]
    fn a_repeating_alarm_rings_only_on_its_day() {
        crate::i18n::init_for("en-US", "en-US");

        let state = Alarms::fixed();
        state.add(alarm(
            7,
            30,
            [true, false, false, false, false, false, false],
        ));

        // A Friday: not its day.
        state.check(at("2026-11-20", "07:30"));
        assert!(state.ringing().is_none());

        // The Monday after.
        state.check(at("2026-11-23", "07:30"));
        assert_eq!(state.ringing().map(|alarm| alarm.id), Some(1));
    }

    #[test]
    fn an_alarm_can_be_turned_off_and_on() {
        let state = Alarms::fixed();
        state.add(alarm(7, 30, [false; 7]));

        state.set_enabled(1, false);
        assert!(state.next(at("2026-11-21", "06:00")).is_none());

        state.set_enabled(1, true);
        assert!(state.next(at("2026-11-21", "06:00")).is_some());
    }

    #[test]
    fn a_snoozed_alarm_rings_again_without_being_kept() {
        crate::i18n::init_for("en-US", "en-US");

        let state = Alarms::fixed();
        state.add(alarm(7, 30, [false; 7]));

        state.check(at("2026-11-21", "07:30"));
        assert_eq!(state.ringing().map(|alarm| alarm.id), Some(1));

        state.snooze_at(at("2026-11-21", "07:30"));
        assert!(state.ringing().is_none());
        assert_eq!(state.list().len(), 1, "a snooze keeps no alarm");

        state.check(at("2026-11-21", "07:34"));
        assert!(state.ringing().is_none(), "not yet");

        state.check(at("2026-11-21", "07:35"));
        assert_eq!(state.ringing().map(|alarm| alarm.id), Some(1));
    }

    #[test]
    fn a_second_alarm_waits_for_the_first_to_be_answered() {
        crate::i18n::init_for("en-US", "en-US");

        let state = Alarms::fixed();
        state.add(alarm(7, 30, [false; 7]));
        state.add(Alarm {
            id: 2,
            hour: 7,
            minute: 30,
            days: [false; 7],
            enabled: true,
            label: String::new(),
        });

        state.check(at("2026-11-21", "07:30"));
        assert_eq!(state.ringing().map(|alarm| alarm.id), Some(1));

        state.dismiss();
        assert_eq!(
            state.ringing().map(|alarm| alarm.id),
            Some(2),
            "the one waiting rings"
        );
    }

    #[test]
    fn the_first_of_several_due_alarms_rings_first() {
        crate::i18n::init_for("en-US", "en-US");

        let state = Alarms::fixed();
        state.add(alarm(7, 30, [false; 7]));
        state.add(Alarm {
            id: 2,
            hour: 7,
            minute: 30,
            days: [false; 7],
            enabled: true,
            label: String::new(),
        });

        // A third alarm is already ringing, so the two due ones must queue in
        // due order: the first ahead of the second.
        state.add(Alarm {
            id: 3,
            hour: 6,
            minute: 0,
            days: [false; 7],
            enabled: true,
            label: String::new(),
        });
        state.check(at("2026-11-21", "06:00"));
        assert_eq!(state.ringing().map(|alarm| alarm.id), Some(3));

        state.check(at("2026-11-21", "07:30"));
        assert_eq!(state.ringing().map(|alarm| alarm.id), Some(3));

        state.dismiss();
        assert_eq!(
            state.ringing().map(|alarm| alarm.id),
            Some(1),
            "the first due rings first"
        );

        state.dismiss();
        assert_eq!(
            state.ringing().map(|alarm| alarm.id),
            Some(2),
            "then the second"
        );
    }

    #[test]
    fn a_second_snooze_does_not_lose_the_first() {
        crate::i18n::init_for("en-US", "en-US");

        let state = Alarms::fixed();
        state.add(alarm(7, 30, [false; 7]));
        state.add(Alarm {
            id: 2,
            hour: 7,
            minute: 30,
            days: [false; 7],
            enabled: true,
            label: String::new(),
        });

        // Both are due at 07:30: the first rings, the second waits.
        state.check(at("2026-11-21", "07:30"));
        assert_eq!(state.ringing().map(|alarm| alarm.id), Some(1));

        // Snooze the first; the second then rings and is snoozed too.
        state.snooze_at(at("2026-11-21", "07:30"));
        assert_eq!(state.ringing().map(|alarm| alarm.id), Some(2));
        state.snooze_at(at("2026-11-21", "07:30"));
        assert!(state.ringing().is_none());

        // Both come back at 07:35, the first ahead of the second.
        state.check(at("2026-11-21", "07:35"));
        assert_eq!(
            state.ringing().map(|alarm| alarm.id),
            Some(1),
            "the first snoozed rings first"
        );
        state.dismiss();
        assert_eq!(
            state.ringing().map(|alarm| alarm.id),
            Some(2),
            "the second snoozed is not lost"
        );
    }
}
