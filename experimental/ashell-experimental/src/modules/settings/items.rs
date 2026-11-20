use std::rc::Rc;

use ashell_services::bluetooth::BluetoothState;
use ashell_services::upower::{BatteryData, BatteryStatus, PeripheralDeviceKind, PowerProfile};
use guido::prelude::*;
use styletto::components::level::{Level as Charge, level_ink};
use styletto::components::progress::ring;
use styletto::foundations::tokens::{icon as icon_box, size};
use styletto::foundations::typography as ty;
use styletto::primitives::text::typed;
use styletto::{Icon, centered_row, icon, ui};
use toml::{Table, Value};

use super::Settings;
use crate::config::{Item, ItemId, Show};
use crate::modules::{ItemView, shown};

const NOTABLE_BELOW: i64 = 20;

/// An item's view, and whether it shows.
type Drawn = (Container, Memo<bool>);

/// An item's own options, read from its table.
pub enum Options {
    Battery {
        style: BatteryStyle,
        notable_below: i64,
    },
    PeripheralBattery {
        notable_below: i64,
    },
    PowerProfile,
    Bluetooth,
    IdleInhibitor,
    Brightness,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BatteryStyle {
    Ring,
    Percentage,
    RingPercentage,
}

impl Options {
    pub fn read(id: ItemId, table: &Table) -> (Self, Vec<String>) {
        let mut problems = Vec::new();
        let known: &[&str] = match id {
            ItemId::Battery => &["style", "notable_below"],
            ItemId::PeripheralBattery => &["notable_below"],
            _ => &[],
        };

        for key in table.keys() {
            if !known.contains(&key.as_str()) {
                problems.push(format!("{key}: unknown option of {}, ignored", id.name()));
            }
        }

        let options = match id {
            ItemId::Battery => Options::Battery {
                style: match table.get("style").map(Value::as_str) {
                    None | Some(Some("ring-percentage")) => BatteryStyle::RingPercentage,
                    Some(Some("ring")) => BatteryStyle::Ring,
                    Some(Some("percentage")) => BatteryStyle::Percentage,
                    Some(_) => {
                        problems.push(
                            "style: expected \"ring\", \"percentage\" or \"ring-percentage\""
                                .to_string(),
                        );
                        BatteryStyle::RingPercentage
                    }
                },
                notable_below: notable_below(&mut problems, table),
            },
            ItemId::PeripheralBattery => Options::PeripheralBattery {
                notable_below: notable_below(&mut problems, table),
            },
            ItemId::PowerProfile => Options::PowerProfile,
            ItemId::Bluetooth => Options::Bluetooth,
            ItemId::IdleInhibitor => Options::IdleInhibitor,
            _ => Options::Brightness,
        };

        (options, problems)
    }
}

fn notable_below(problems: &mut Vec<String>, table: &Table) -> i64 {
    match table.get("notable_below").map(Value::as_integer) {
        None => NOTABLE_BELOW,
        Some(Some(percent @ 1..=100)) => percent,
        Some(_) => {
            problems.push("notable_below: expected a percentage from 1 to 100".to_string());
            NOTABLE_BELOW
        }
    }
}

pub fn view(settings: Settings, item: &Item) -> ItemView {
    let show = item.show;
    let (options, _) = Options::read(item.id, &item.options);

    let (view, shown) = match options {
        Options::Battery {
            style,
            notable_below,
        } => battery(settings, show, style, notable_below),
        Options::PeripheralBattery { notable_below } => {
            peripheral_battery(settings, show, notable_below)
        }
        Options::PowerProfile => power_profile(settings, show),
        Options::Bluetooth => bluetooth(settings, show),
        Options::IdleInhibitor => idle_inhibitor(settings, show),
        Options::Brightness => brightness(settings, show),
    };

    ItemView {
        view,
        shown,
        act: Rc::new(move |action| settings.act(action)),
    }
}

fn label(text: impl Fn() -> String + 'static) -> Text {
    typed(text, ty::LABEL_SMALL.tabular())
        .color(ui().colors.sub)
        .nowrap()
}

fn charging(battery: &BatteryData) -> bool {
    matches!(battery.status, BatteryStatus::Charging(_))
}

/// How a battery stands, as ashell colours it: well on the charger, an alert
/// at 15% or less, a warning at 30% or less or while saving power.
fn battery_level(percent: f32, plugged: bool, saving: bool) -> Charge {
    match Charge::below(percent, 30.0, 15.0) {
        _ if plugged => Charge::Good,
        Charge::Normal if saving => Charge::Warn,
        level => level,
    }
}

fn battery(settings: Settings, show: Show, style: BatteryStyle, notable_below: i64) -> Drawn {
    let upower = settings.upower.data;

    let battery = create_memo(move || {
        upower.with(|data| {
            data.system_battery.map(|battery| {
                let plugged = data.on_battery == Some(false);
                let saving = data.power_profile == PowerProfile::PowerSaver;

                (battery, plugged, saving)
            })
        })
    });
    let percent = move || battery.get().map_or(0.0, |(b, _, _)| b.capacity as f32);
    let ink = move || {
        level_ink(
            battery
                .get()
                .map_or(Charge::Normal, |(b, plugged, saving)| {
                    battery_level(b.capacity as f32, plugged, saving)
                }),
        )
    };

    let view = container()
        .layout(centered_row(size::S1.px()))
        .child((style != BatteryStyle::Percentage).then(|| {
            container().child(
                ring(move || percent() / 100.0)
                    .color(ink)
                    .size(icon_box::SMALL),
            )
        }))
        .child((style != BatteryStyle::Ring).then(|| label(move || format!("{}%", percent()))));

    (
        view,
        shown(
            show,
            move || battery.with(Option::is_some),
            move || {
                battery
                    .get()
                    .is_some_and(|(b, _, _)| b.capacity < notable_below || charging(&b))
            },
        ),
    )
}

fn peripheral_battery(settings: Settings, show: Show, notable_below: i64) -> Drawn {
    let upower = settings.upower.data;

    let devices = create_memo(move || {
        upower.with(|data| {
            data.peripherals
                .iter()
                .map(|p| (p.kind, p.data))
                .collect::<Vec<_>>()
        })
    });
    let low = move |battery: &BatteryData| battery.capacity < notable_below;

    let view = container()
        .layout(centered_row(size::S2.px()))
        .children(move || {
            devices
                .get()
                .into_iter()
                .filter(|(_, battery)| show != Show::Notable || low(battery))
                .map(|(kind, battery)| {
                    let percent = battery.capacity as f32;
                    let ink = level_ink(battery_level(percent, charging(&battery), false));

                    container()
                        .layout(centered_row(size::S0_5.px()))
                        .child(icon(peripheral_icon(kind), icon_box::SMALL.px(), ink))
                        .child(label(move || format!("{percent}%")))
                })
                .collect::<Vec<_>>()
        });

    (
        view,
        shown(
            show,
            move || devices.with(|devices| !devices.is_empty()),
            move || devices.with(|devices| devices.iter().any(|(_, b)| low(b))),
        ),
    )
}

fn peripheral_icon(kind: PeripheralDeviceKind) -> Icon {
    match kind {
        PeripheralDeviceKind::Keyboard => Icon::Keyboard,
        PeripheralDeviceKind::Mouse => Icon::Mouse,
        PeripheralDeviceKind::Headphones => Icon::Headphones,
        PeripheralDeviceKind::Gamepad => Icon::Gamepad,
    }
}

fn power_profile(settings: Settings, show: Show) -> Drawn {
    let t = ui();
    let upower = settings.upower.data;

    let profile = create_memo(move || upower.with(|data| data.power_profile));
    let ink = move || match profile.get() {
        PowerProfile::Performance => t.colors.warning.text,
        PowerProfile::PowerSaver => t.colors.success.text,
        PowerProfile::Balanced | PowerProfile::Unknown => t.colors.text,
    };

    let view = container().child(icon(
        move || power_profile_icon(profile.get()),
        icon_box::SMALL.px(),
        ink,
    ));

    (
        view,
        shown(
            show,
            move || profile.get() != PowerProfile::Unknown,
            move || profile.get() != PowerProfile::Balanced,
        ),
    )
}

fn power_profile_icon(profile: PowerProfile) -> Icon {
    match profile {
        PowerProfile::Performance => Icon::Gauge,
        PowerProfile::PowerSaver => Icon::Leaf,
        PowerProfile::Balanced | PowerProfile::Unknown => Icon::Zap,
    }
}

fn bluetooth(settings: Settings, show: Show) -> Drawn {
    let t = ui();
    let data = settings.bluetooth.data;

    let state = create_memo(move || data.with(|data| data.state.clone()));
    let connected =
        create_memo(move || data.with(|data| data.devices.iter().filter(|d| d.connected).count()));

    let view = container()
        .layout(centered_row(size::S1.px()))
        .child(icon(
            move || match (state.get(), connected.get()) {
                (BluetoothState::Active, 0) => Icon::Bluetooth,
                (BluetoothState::Active, _) => Icon::BluetoothConnected,
                _ => Icon::BluetoothOff,
            },
            icon_box::SMALL.px(),
            move || {
                if state.get() == BluetoothState::Active {
                    t.colors.text
                } else {
                    t.colors.dim
                }
            },
        ))
        .child(move || (connected.get() > 0).then(|| label(move || connected.get().to_string())));

    (
        view,
        shown(
            show,
            move || state.get() != BluetoothState::Unavailable,
            move || connected.get() > 0,
        ),
    )
}

fn idle_inhibitor(settings: Settings, show: Show) -> Drawn {
    let t = ui();
    let inhibited = settings.idle.inhibited;

    let view = container().child(icon(Icon::Coffee, icon_box::SMALL.px(), move || {
        if inhibited.get() == Some(true) {
            t.colors.text
        } else {
            t.colors.dim
        }
    }));

    (
        view,
        shown(
            show,
            move || inhibited.get().is_some(),
            move || inhibited.get() == Some(true),
        ),
    )
}

fn brightness(settings: Settings, show: Show) -> Drawn {
    let t = ui();
    let data = settings.brightness.data;

    let percent = create_memo(move || {
        data.get()
            .map(|data| (data.current * 100).checked_div(data.max).unwrap_or(0))
    });

    let view = container()
        .layout(centered_row(size::S1.px()))
        .child(icon(Icon::Sun, icon_box::SMALL.px(), t.colors.text))
        .child(label(move || {
            percent.get().map(|p| format!("{p}%")).unwrap_or_default()
        }));

    (view, shown(show, move || percent.get().is_some(), || true))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(id: ItemId, text: &str) -> (Options, Vec<String>) {
        Options::read(id, &toml::from_str(text).unwrap())
    }

    #[test]
    fn the_battery_has_a_style_and_a_threshold() {
        let (options, problems) = read(
            ItemId::Battery,
            r#"style = "ring"
notable_below = 30"#,
        );

        assert!(problems.is_empty());
        assert!(matches!(
            options,
            Options::Battery {
                style: BatteryStyle::Ring,
                notable_below: 30
            }
        ));
    }

    #[test]
    fn wrong_options_fall_back_to_the_defaults() {
        let (options, problems) = read(
            ItemId::Battery,
            r#"style = "bar"
notable_below = 0"#,
        );

        assert_eq!(problems.len(), 2);
        assert!(matches!(
            options,
            Options::Battery {
                style: BatteryStyle::RingPercentage,
                notable_below: NOTABLE_BELOW
            }
        ));
    }

    #[test]
    fn items_without_options_name_what_they_ignore() {
        let (_, problems) = read(ItemId::Bluetooth, "notable_below = 10");

        assert_eq!(
            problems,
            ["notable_below: unknown option of bluetooth, ignored"]
        );
    }
}
