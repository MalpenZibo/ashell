use std::rc::Rc;
use std::time::Duration;

use ashell_services::bluetooth::BluetoothState;
use ashell_services::upower::{
    BatteryData, BatteryStatus, PeripheralDeviceKind, PowerProfile, UPowerData,
};
use guido::prelude::*;
use styletto::components::level::{Glyph, Level as Charge, Levels, level_indicator};
use styletto::foundations::tokens::{icon as icon_box, size};
use styletto::foundations::typography as ty;
use styletto::primitives::text::typed;
use styletto::{Icon, centered_row, icon, look};
use toml::{Table, Value};

use super::Settings;
use crate::config::{Item, ItemId};
use crate::modules::{ItemView, Style, shown, unknown_options};

/// ashell's: a warning below 30%, an alert below 20%.
pub(super) const THRESHOLDS: Thresholds = Thresholds {
    warn: 30,
    alert: 20,
};

const CHARGING: &str = "charging";
const DISCHARGING: &str = "discharging";
const NOT_CHARGING: &str = "not_charging";
const FULL: &str = "full";
const WARN: &str = "warn";
const ALERT: &str = "alert";
const ON: &str = "on";
const OFF: &str = "off";
const CONNECTED: &str = "connected";
const BALANCED: &str = "balanced";
const PERFORMANCE: &str = "performance";
const POWER_SAVER: &str = "power_saver";

/// The states an item can be in, for its `show`.
pub fn states(id: ItemId) -> &'static [&'static str] {
    match id {
        ItemId::Battery => &[CHARGING, DISCHARGING, NOT_CHARGING, FULL, WARN, ALERT],
        ItemId::BatteryTime => &[CHARGING, DISCHARGING],
        ItemId::PeripheralBattery => &[CHARGING, DISCHARGING, FULL, WARN, ALERT],
        ItemId::Bluetooth => &[ON, OFF, CONNECTED],
        ItemId::IdleInhibitor => &[ON, OFF],
        ItemId::PowerProfile => &[BALANCED, PERFORMANCE, POWER_SAVER],
        _ => &[],
    }
}

/// Those it is worth showing in.
pub fn notable(id: ItemId) -> &'static [&'static str] {
    match id {
        ItemId::Battery | ItemId::PeripheralBattery => &[WARN, ALERT, CHARGING],
        ItemId::BatteryTime => &[DISCHARGING],
        ItemId::Bluetooth => &[ON],
        ItemId::IdleInhibitor => &[ON],
        ItemId::PowerProfile => &[PERFORMANCE, POWER_SAVER],
        _ => &[],
    }
}

/// An item's view, and whether it shows.
type Drawn = (Container, Memo<bool>);

/// An item's own options, read from its table.
pub enum Options {
    Battery {
        style: Style,
        glyph: Glyph,
        thresholds: Thresholds,
    },
    BatteryTime,
    PeripheralBattery {
        style: Style,
        thresholds: Thresholds,
        kinds: Vec<PeripheralDeviceKind>,
    },
    PowerProfile,
    Bluetooth {
        style: Style,
    },
    IdleInhibitor,
    Brightness {
        style: Style,
    },
}

/// Where a battery's charge turns to a warning and to an alert: below
/// `warn_threshold` and below `alert_threshold`, as ashell's loads turn above
/// theirs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Thresholds {
    warn: i64,
    alert: i64,
}

impl Thresholds {
    fn level(self, capacity: i64) -> Charge {
        if capacity < self.alert {
            Charge::Alert
        } else if capacity < self.warn {
            Charge::Warn
        } else {
            Charge::Normal
        }
    }
}

const KINDS: [(&str, PeripheralDeviceKind); 4] = [
    ("keyboard", PeripheralDeviceKind::Keyboard),
    ("mouse", PeripheralDeviceKind::Mouse),
    ("headphones", PeripheralDeviceKind::Headphones),
    ("gamepad", PeripheralDeviceKind::Gamepad),
];

impl Options {
    pub fn read(id: ItemId, table: &Table) -> (Self, Vec<String>) {
        let mut problems = Vec::new();
        let known: &[&str] = match id {
            ItemId::Battery => &["style", "glyph", "warn_threshold", "alert_threshold"],
            ItemId::PeripheralBattery => &["style", "warn_threshold", "alert_threshold", "kinds"],
            ItemId::Bluetooth | ItemId::Brightness => &["style"],
            _ => &[],
        };
        unknown_options(id, table, known, &mut problems);

        let options = match id {
            ItemId::Battery => Options::Battery {
                style: Style::read(table, Style::IconValue, &mut problems),
                glyph: match table.get("glyph").map(Value::as_str) {
                    None | Some(Some("icon")) => Glyph::Icon,
                    Some(Some("ring")) => Glyph::Ring,
                    Some(_) => {
                        problems.push(r#"glyph: expected "icon" or "ring""#.to_string());
                        Glyph::Icon
                    }
                },
                thresholds: thresholds(&mut problems, table),
            },
            ItemId::BatteryTime => Options::BatteryTime,
            ItemId::PeripheralBattery => Options::PeripheralBattery {
                style: Style::read(table, Style::Icon, &mut problems),
                thresholds: thresholds(&mut problems, table),
                kinds: kinds(&mut problems, table),
            },
            ItemId::PowerProfile => Options::PowerProfile,
            ItemId::Bluetooth => Options::Bluetooth {
                style: Style::read(table, Style::Icon, &mut problems),
            },
            ItemId::IdleInhibitor => Options::IdleInhibitor,
            _ => Options::Brightness {
                style: Style::read(table, Style::Icon, &mut problems),
            },
        };

        (options, problems)
    }
}

/// A warning above the alert, or the alert alone, as ashell keeps them.
fn thresholds(problems: &mut Vec<String>, table: &Table) -> Thresholds {
    let mut percent = |key: &str, default: i64| match table.get(key).map(Value::as_integer) {
        None => default,
        Some(Some(percent @ 1..=100)) => percent,
        Some(_) => {
            problems.push(format!("{key}: expected a percentage from 1 to 100"));
            default
        }
    };
    let warn = percent("warn_threshold", THRESHOLDS.warn);
    let alert = percent("alert_threshold", THRESHOLDS.alert);

    if warn <= alert {
        problems.push(format!(
            "warn_threshold: {warn} is not above alert_threshold {alert}, only the alert is kept"
        ));
    }

    Thresholds {
        warn: warn.max(alert),
        alert,
    }
}

/// The kinds of device to show, every one while unset.
fn kinds(problems: &mut Vec<String>, table: &Table) -> Vec<PeripheralDeviceKind> {
    let every = || KINDS.iter().map(|(_, kind)| *kind).collect();
    let Some(value) = table.get("kinds") else {
        return every();
    };
    let Some(names) = value.as_array() else {
        problems.push("kinds: expected a list of kinds of device".to_string());
        return every();
    };

    names
        .iter()
        .filter_map(|name| {
            let kind = name
                .as_str()
                .and_then(|name| KINDS.iter().find(|(known, _)| *known == name));

            if kind.is_none() {
                problems.push(format!(
                    r#"kinds: {name} is not "keyboard", "mouse", "headphones" or "gamepad""#
                ));
            }

            kind.map(|(_, kind)| *kind)
        })
        .collect()
}

pub fn view(settings: Settings, item: &Item) -> ItemView {
    let (options, _) = Options::read(item.id, &item.options);

    let (view, shown) = match options {
        Options::Battery {
            style,
            glyph,
            thresholds,
        } => battery(settings, item, style, glyph, thresholds),
        Options::BatteryTime => battery_time(settings, item),
        Options::PeripheralBattery {
            style,
            thresholds,
            kinds,
        } => peripheral_battery(settings, item, style, thresholds, kinds),
        Options::PowerProfile => power_profile(settings, item),
        Options::Bluetooth { style } => bluetooth(settings, item, style),
        Options::IdleInhibitor => idle_inhibitor(settings, item),
        Options::Brightness { style } => brightness(settings, item, style),
    };

    ItemView {
        view,
        shown,
        act: Rc::new(move |action| settings.act(action)),
    }
}

fn label(text: impl Fn() -> String + 'static) -> Text {
    typed(text, ty::LABEL_SMALL.tabular())
        .color(look().eased(|c| c.sub))
        .nowrap()
}

/// A battery without its time estimate, which changes far more often than
/// anything drawn from the rest.
fn without_estimate(mut battery: BatteryData) -> BatteryData {
    battery.status = match battery.status {
        BatteryStatus::Charging(_) => BatteryStatus::Charging(Duration::ZERO),
        BatteryStatus::Discharging(_) => BatteryStatus::Discharging(Duration::ZERO),
        status => status,
    };
    battery
}

fn charging(battery: &BatteryData) -> bool {
    matches!(battery.status, BatteryStatus::Charging(_))
}

/// Whether a battery is in `state`.
fn battery_is(battery: &BatteryData, thresholds: Thresholds, state: &str) -> bool {
    match state {
        CHARGING => charging(battery),
        DISCHARGING => matches!(battery.status, BatteryStatus::Discharging(_)),
        NOT_CHARGING => battery.status == BatteryStatus::NotCharging,
        FULL => battery.status == BatteryStatus::Full,
        WARN => battery_level(battery, thresholds, false) == Charge::Warn,
        ALERT => battery_level(battery, thresholds, false) == Charge::Alert,
        _ => false,
    }
}

/// How a battery stands: well while charging or held by a charge limit, its
/// warning or alert while discharging.
pub(super) fn battery_level(battery: &BatteryData, thresholds: Thresholds, held: bool) -> Charge {
    if charging(battery) || held {
        Charge::Good
    } else if battery.is_discharging {
        thresholds.level(battery.capacity)
    } else {
        Charge::Normal
    }
}

fn battery(
    settings: Settings,
    item: &Item,
    style: Style,
    glyph: Glyph,
    thresholds: Thresholds,
) -> Drawn {
    let upower = settings.upower.data;

    let battery = create_memo(move || {
        upower.with(|data| {
            data.system_battery
                .map(|battery| (without_estimate(battery), held(data, &battery)))
        })
    });
    let percent = move || battery.get().map_or(0.0, |(b, _)| b.capacity as f32);
    let level = move || {
        battery.get().map_or(Charge::Normal, |(b, held)| {
            battery_level(&b, thresholds, held)
        })
    };
    let is = move |state: &str| {
        battery
            .get()
            .is_some_and(|(b, _)| battery_is(&b, thresholds, state))
    };

    // A style without an icon has no ring either.
    let glyph = if style.icon() { glyph } else { Glyph::Icon };

    let view = container().child(
        level_indicator(levels(style, Levels::BATTERY), percent)
            .glyph(glyph)
            .icon(move || (style.icon() && is(CHARGING)).then_some(Icon::BatteryCharging))
            .level(level)
            .percent(style.value()),
    );

    (view, shown(item, move || battery.with(Option::is_some), is))
}

/// How long until the battery is full or empty, while there is an estimate.
fn battery_time(settings: Settings, item: &Item) -> Drawn {
    let upower = settings.upower.data;

    let battery = create_memo(move || upower.with(|data| data.system_battery));
    let left = create_memo(move || upower.with(time_left));
    let is = move |state: &str| {
        battery
            .get()
            .is_some_and(|battery| battery_is(&battery, THRESHOLDS, state))
    };

    let view = container().child(label(move || left.get().map(span).unwrap_or_default()));

    (view, shown(item, move || left.get().is_some(), is))
}

/// How long until the battery is full or empty, as the panel says it: the
/// charger, not the battery's state, tells charging from draining. On the
/// charger with the charge held, as near a full charge, the firmware lets the
/// battery drain a little and UPower estimates days to empty: none then.
pub(super) fn time_left(data: &UPowerData) -> Option<Duration> {
    match data.system_battery?.status {
        BatteryStatus::Charging(left) if !left.is_zero() => Some(left),
        BatteryStatus::Discharging(left) if !left.is_zero() && !plugged(data) => Some(left),
        _ => None,
    }
}

/// On the charger, whatever the battery says it is doing.
pub(super) fn plugged(data: &UPowerData) -> bool {
    data.on_battery == Some(false)
}

/// Kept from charging by a charge limit: a success, as ashell colours it.
pub(super) fn held(data: &UPowerData, battery: &BatteryData) -> bool {
    data.charge_limit.is_some_and(|limit| limit.enabled) && !battery.is_discharging
}

/// "1h 12m", "45m".
pub(super) fn span(span: Duration) -> String {
    let minutes = span.as_secs() / 60;

    match (minutes / 60, minutes % 60) {
        (0, m) => format!("{m}m"),
        (h, m) => format!("{h}h {m}m"),
    }
}

/// The devices of `kinds` that report a battery, each as `show` lets it: a
/// device worth showing is one that is low.
fn peripheral_battery(
    settings: Settings,
    item: &Item,
    style: Style,
    thresholds: Thresholds,
    kinds: Vec<PeripheralDeviceKind>,
) -> Drawn {
    let upower = settings.upower.data;
    let show = item.show.clone();
    let notable = notable(item.id);

    let devices = create_memo(move || {
        upower.with(|data| {
            data.peripherals
                .iter()
                .filter(|p| kinds.contains(&p.kind))
                .map(|p| (p.path.clone(), p.kind, without_estimate(p.data)))
                .filter(|(_, _, battery)| {
                    let is = |state: &str| battery_is(battery, thresholds, state);

                    show.admits(notable, is)
                })
                .collect::<Vec<_>>()
        })
    });

    // A device's row is made again only when it changes.
    let view = container()
        .layout(centered_row(size::S2.px()))
        .children(keyed(
            move || devices.get(),
            |(path, _, _)| path.clone(),
            move |(_, kind, battery)| {
                level_indicator(levels(style, Levels(&[])), battery.capacity as f32)
                    .icon(style.icon().then_some(peripheral_icon(kind)))
                    .level(battery_level(&battery, thresholds, false))
                    .percent(style.value())
            },
        ));

    (
        view,
        create_memo(move || devices.with(|devices| !devices.is_empty())),
    )
}

/// `levels` while the style has an icon, none otherwise.
fn levels(style: Style, levels: Levels) -> Levels {
    if style.icon() { levels } else { Levels(&[]) }
}

pub(super) fn peripheral_icon(kind: PeripheralDeviceKind) -> Icon {
    match kind {
        PeripheralDeviceKind::Keyboard => Icon::Keyboard,
        PeripheralDeviceKind::Mouse => Icon::Mouse,
        PeripheralDeviceKind::Headphones => Icon::Headphones,
        PeripheralDeviceKind::Gamepad => Icon::Gamepad,
    }
}

fn power_profile(settings: Settings, item: &Item) -> Drawn {
    let upower = settings.upower.data;

    let profile = create_memo(move || upower.with(|data| data.power_profile));
    let ink = look().color(move |c| match profile.get() {
        PowerProfile::Performance => c.warning.text,
        PowerProfile::PowerSaver => c.success.text,
        PowerProfile::Balanced | PowerProfile::Unknown => c.text,
    });
    let is = move |state: &str| {
        matches!(
            (state, profile.get()),
            (BALANCED, PowerProfile::Balanced)
                | (PERFORMANCE, PowerProfile::Performance)
                | (POWER_SAVER, PowerProfile::PowerSaver)
        )
    };

    let view = container().child(icon(
        move || power_profile_icon(profile.get()),
        icon_box::SMALL.px(),
        ink,
    ));

    (
        view,
        shown(item, move || profile.get() != PowerProfile::Unknown, is),
    )
}

pub(super) fn power_profile_icon(profile: PowerProfile) -> Icon {
    match profile {
        PowerProfile::Performance => Icon::Gauge,
        PowerProfile::PowerSaver => Icon::Leaf,
        PowerProfile::Balanced | PowerProfile::Unknown => Icon::Zap,
    }
}

/// Bluetooth's icon, on or off.
pub(super) fn bluetooth_icon(on: bool) -> Icon {
    if on {
        Icon::Bluetooth
    } else {
        Icon::BluetoothOff
    }
}

/// On or off, and how many devices are connected: its value.
fn bluetooth(settings: Settings, item: &Item, style: Style) -> Drawn {
    let data = settings.bluetooth.data;

    let state = create_memo(move || data.with(|data| data.state.clone()));
    let connected =
        create_memo(move || data.with(|data| data.devices.iter().filter(|d| d.connected).count()));
    let any = create_memo(move || connected.get() > 0);
    let is = move |name: &str| {
        let on = state.get() == BluetoothState::Active;

        match name {
            ON => on,
            OFF => !on,
            CONNECTED => any.get(),
            _ => false,
        }
    };

    let view = container()
        .layout(centered_row(size::S1.px()))
        .child(style.icon().then(|| {
            icon(
                move || match (state.get(), connected.get()) {
                    (BluetoothState::Active, 0) => Icon::Bluetooth,
                    (BluetoothState::Active, _) => Icon::BluetoothConnected,
                    _ => Icon::BluetoothOff,
                },
                icon_box::SMALL.px(),
                look().color(move |c| {
                    if state.get() == BluetoothState::Active {
                        c.text
                    } else {
                        c.dim
                    }
                }),
            )
        }))
        .child(move || {
            // Beside the icon only while there are some; alone, always.
            (style.value() && (any.get() || !style.icon()))
                .then(|| label(move || connected.get().to_string()))
        });

    (
        view,
        shown(item, move || state.get() != BluetoothState::Unavailable, is),
    )
}

fn idle_inhibitor(settings: Settings, item: &Item) -> Drawn {
    let inhibited = settings.idle.inhibited;

    let is = move |state: &str| match state {
        ON => inhibited.get() == Some(true),
        OFF => inhibited.get() == Some(false),
        _ => false,
    };

    let view = container().child(icon(
        Icon::Coffee,
        icon_box::SMALL.px(),
        look().color(move |c| {
            if inhibited.get() == Some(true) {
                c.text
            } else {
                c.dim
            }
        }),
    ));

    (view, shown(item, move || inhibited.get().is_some(), is))
}

fn brightness(settings: Settings, item: &Item, style: Style) -> Drawn {
    let data = settings.brightness.data;

    let percent = create_memo(move || {
        data.get()
            .map(|data| (data.current * 100).checked_div(data.max).unwrap_or(0))
    });

    let view =
        container()
            .layout(centered_row(size::S1.px()))
            .child(
                style
                    .icon()
                    .then(|| icon(Icon::Sun, icon_box::SMALL.px(), look().color(|c| c.text))),
            )
            .child(style.value().then(|| {
                label(move || percent.get().map(|p| format!("{p}%")).unwrap_or_default())
            }));

    (
        view,
        shown(item, move || percent.get().is_some(), |_| false),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(id: ItemId, text: &str) -> (Options, Vec<String>) {
        Options::read(id, &toml::from_str(text).unwrap())
    }

    #[test]
    fn a_battery_held_on_the_charger_has_no_time_left() {
        let data = |on_battery, status| UPowerData {
            on_battery: Some(on_battery),
            system_battery: Some(at(100, status)),
            ..UPowerData::default()
        };
        let days = Duration::from_secs(140 * 24 * 3600);
        let hour = Duration::from_secs(3600);

        assert_eq!(
            time_left(&data(false, BatteryStatus::Discharging(days))),
            None
        );
        assert_eq!(
            time_left(&data(true, BatteryStatus::Discharging(hour))),
            Some(hour)
        );
        assert_eq!(
            time_left(&data(false, BatteryStatus::Charging(hour))),
            Some(hour)
        );
        assert_eq!(
            time_left(&data(true, BatteryStatus::Discharging(Duration::ZERO))),
            None
        );
    }

    #[test]
    fn the_battery_has_a_format_a_glyph_and_thresholds() {
        let (options, problems) = read(
            ItemId::Battery,
            r#"style = "icon"
glyph = "ring"
warn_threshold = 40
alert_threshold = 10"#,
        );

        assert!(problems.is_empty());
        assert!(matches!(
            options,
            Options::Battery {
                style: Style::Icon,
                glyph: Glyph::Ring,
                thresholds: Thresholds {
                    warn: 40,
                    alert: 10
                },
            }
        ));
    }

    #[test]
    fn wrong_options_fall_back_to_ashells() {
        let (options, problems) = read(
            ItemId::Battery,
            r#"style = "both"
glyph = "bar"
warn_threshold = 0"#,
        );

        assert_eq!(problems.len(), 3);
        assert!(matches!(
            options,
            Options::Battery {
                style: Style::IconValue,
                glyph: Glyph::Icon,
                thresholds: THRESHOLDS,
            }
        ));
    }

    #[test]
    fn a_warning_not_above_the_alert_leaves_the_alert_alone() {
        let (options, problems) = read(ItemId::PeripheralBattery, "alert_threshold = 40");

        assert_eq!(
            problems,
            ["warn_threshold: 30 is not above alert_threshold 40, only the alert is kept"]
        );
        assert!(matches!(
            options,
            Options::PeripheralBattery {
                thresholds: Thresholds {
                    warn: 40,
                    alert: 40
                },
                ..
            }
        ));
    }

    #[test]
    fn peripherals_are_chosen_by_kind() {
        let (options, problems) = read(ItemId::PeripheralBattery, r#"kinds = ["mouse", "phone"]"#);

        assert_eq!(
            problems,
            [r#"kinds: "phone" is not "keyboard", "mouse", "headphones" or "gamepad""#]
        );
        assert!(matches!(
            options,
            Options::PeripheralBattery { style: Style::Icon, kinds, .. }
                if kinds == [PeripheralDeviceKind::Mouse]
        ));
    }

    #[test]
    fn items_without_options_name_what_they_ignore() {
        let (_, problems) = read(ItemId::PowerProfile, "alert_threshold = 10");

        assert_eq!(
            problems,
            ["alert_threshold: unknown option of power_profile, ignored"]
        );
    }

    use crate::modules::settings::sample::battery as at;

    const UNPLUGGED: BatteryStatus = BatteryStatus::Discharging(Duration::ZERO);

    #[test]
    fn a_battery_warns_below_one_threshold_and_alerts_below_the_other() {
        assert!(battery_is(&at(25, UNPLUGGED), THRESHOLDS, WARN));
        assert!(!battery_is(&at(25, UNPLUGGED), THRESHOLDS, ALERT));
        assert!(battery_is(&at(19, UNPLUGGED), THRESHOLDS, ALERT));
        assert!(!battery_is(&at(19, UNPLUGGED), THRESHOLDS, WARN));
        assert!(!battery_is(&at(30, UNPLUGGED), THRESHOLDS, WARN));
        assert!(battery_is(&at(30, UNPLUGGED), THRESHOLDS, DISCHARGING));
        assert!(!battery_is(&at(30, UNPLUGGED), THRESHOLDS, FULL));
    }

    #[test]
    fn a_battery_on_the_charger_is_neither_low_nor_alarming() {
        let charging = BatteryStatus::Charging(Duration::ZERO);

        assert!(!battery_is(&at(15, charging), THRESHOLDS, WARN));
        assert!(!battery_is(&at(15, charging), THRESHOLDS, ALERT));
        assert!(!battery_is(
            &at(15, BatteryStatus::NotCharging),
            THRESHOLDS,
            ALERT
        ));
        assert!(battery_is(&at(15, charging), THRESHOLDS, CHARGING));
    }

    #[test]
    fn a_battery_is_well_while_charging_and_its_level_while_discharging() {
        let charging = BatteryStatus::Charging(Duration::ZERO);

        assert_eq!(
            battery_level(&at(25, UNPLUGGED), THRESHOLDS, false),
            Charge::Warn
        );
        assert_eq!(
            battery_level(&at(19, UNPLUGGED), THRESHOLDS, false),
            Charge::Alert
        );
        assert_eq!(
            battery_level(&at(50, UNPLUGGED), THRESHOLDS, false),
            Charge::Normal
        );
        assert_eq!(
            battery_level(&at(10, charging), THRESHOLDS, false),
            Charge::Good
        );
        assert_eq!(
            battery_level(&at(10, BatteryStatus::NotCharging), THRESHOLDS, false),
            Charge::Normal
        );
        assert_eq!(
            battery_level(&at(80, BatteryStatus::NotCharging), THRESHOLDS, true),
            Charge::Good
        );
        assert_eq!(
            battery_level(&at(100, BatteryStatus::Full), THRESHOLDS, false),
            Charge::Normal
        );
    }

    #[test]
    fn a_span_is_in_hours_and_minutes() {
        assert_eq!(span(Duration::from_secs(45 * 60)), "45m");
        assert_eq!(span(Duration::from_secs(72 * 60)), "1h 12m");
    }
}
