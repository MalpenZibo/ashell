//! The `settings` panel, as the prototype's: the battery and the power
//! buttons, the brightness, the toggles, and the batteries of other devices;
//! and the pages they open: power, the power profiles and Bluetooth.

use ashell_i18n::t;
use ashell_services::bluetooth::{BluetoothCommand, BluetoothDevice, BluetoothState};
use ashell_services::upower::{
    BatteryData, BatteryStatus, PowerProfile, UPowerCommand, UPowerData,
};
use guido::prelude::*;
use styletto::components::button::button;
use styletto::components::level::{Level, Levels, level_indicator};
use styletto::components::list::{
    back, empty, page_header, pages, row, scroll_list, section, separator,
};
use styletto::components::slider::slider_row;
use styletto::components::tag::{Tone, tag};
use styletto::components::tile::{small_tile, tile};
use styletto::foundations::control::Size;
use styletto::foundations::tokens::size;
use styletto::foundations::typography as ty;
use styletto::primitives::text::{caption, typed};
use styletto::{Icon, icon, look};

use super::{Settings, items};
use crate::command;
use crate::modules::configurator::Shown;
use crate::modules::{ClosePanel, Modules};

/// As the prototype's, following the density.
pub fn width() -> f32 {
    size::scaled(392.0)
}

/// How tall a page's list grows before it scrolls, following the density.
fn list_height() -> f32 {
    size::scaled(320.0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    Main,
    Power,
    PowerProfiles,
    Bluetooth,
}

pub fn view(settings: Settings) -> Container {
    let stack = create_signal(vec![Page::Main]);

    // At its own height while the panel grows to it: laid out at the growing
    // height, a page's list would shrink and show its scrollbar.
    container()
        .width(width())
        .height(natural())
        .padding(size::S4.px())
        .child(pages(stack, move |page| match page {
            Page::Main => main(settings, stack),
            Page::Power => power(settings, stack),
            Page::PowerProfiles => power_profiles(settings, stack),
            Page::Bluetooth => bluetooth(settings, stack),
        }))
}

/// Between the main page's groups.
fn sections() -> f32 {
    size::S6.px()
}

/// Between a page's head and its list, whose rows are padded already: as far
/// apart as the main page's groups look.
fn below_head() -> f32 {
    size::S4.px()
}

fn main(settings: Settings, stack: RwSignal<Vec<Page>>) -> Container {
    let upower = settings.upower.data;

    let any_peripheral = create_memo(move || upower.with(|data| !data.peripherals.is_empty()));

    column(sections())
        .child(header(settings, stack))
        .child(brightness(settings))
        .child(tiles(settings, stack))
        .child(move || any_peripheral.get().then(|| peripherals(settings)))
}

fn push(stack: RwSignal<Vec<Page>>, page: Page) {
    stack.update(|pages| pages.push(page));
}

/// A page's head: back, the title with what stands now under it, and at the
/// end the feature's icon: a switch when the page can turn it on and off, a
/// mark when it cannot.
fn head<M>(
    stack: RwSignal<Vec<Page>>,
    title: String,
    status: impl IntoSignal<String, M>,
    end: impl Widget + 'static,
) -> impl Widget {
    page_header(title)
        .subtitle(status)
        .on_back(back(stack))
        .trailing(end)
}

/// The feature's icon at a page's head, when nothing there turns it on or
/// off: bare, where a switch's icon would be.
fn mark<M>(kind: impl IntoSignal<Icon, M>) -> Container {
    container()
        .width(Size::Lg.height())
        .layout(Flex::column().center())
        .child(icon(kind, Size::Lg.icon(), look().color(|c| c.sub)))
}

/// The feature's icon at a page's head as a switch, filled while on.
fn switch<M>(
    kind: impl IntoSignal<Icon, M>,
    on: Memo<bool>,
    flip: impl Fn() + 'static,
) -> impl Widget {
    let kind = kind.into_signal();

    button()
        .icon(move || Some(kind.get()))
        .size(Size::Lg)
        .active(on)
        .on_click(flip)
}

fn column(spacing: f32) -> Container {
    container()
        .width(fill())
        .layout(Flex::column().spacing(spacing))
}

/// The battery, then the button to the config window, the lock button and
/// the one to the power page.
fn header(settings: Settings, stack: RwSignal<Vec<Page>>) -> Container {
    let close = expect_context::<ClosePanel>();
    let modules = expect_context::<Modules>();
    let lock = create_memo(move || settings.config.with(|c| c.lock.clone()));
    // Worth a page with more than the lock, which has its own button here.
    let has_power = create_memo(move || {
        settings
            .config
            .with(|c| power_commands(c).len() > usize::from(c.lock.is_some()))
    });

    container()
        .width(fill())
        .layout(styletto::centered_row(size::S2.px()))
        .child(
            container()
                .width(fill())
                .layout(Flex::row())
                .child(battery(settings)),
        )
        .child({
            let close = close.clone();

            button().icon(Icon::Settings).on_click(move || {
                close.close();
                modules.configure(Shown::Settings);
            })
        })
        .child(move || {
            lock.get().map(|lock| {
                let close = close.clone();

                button()
                    .icon(Icon::Lock)
                    .on_click(move || run(&lock, &close))
            })
        })
        .child(move || {
            has_power.get().then(|| {
                button()
                    .icon(Icon::Power)
                    .on_click(move || push(stack, Page::Power))
            })
        })
}

/// Runs a power button's `command`, the panel closing behind it.
fn run(command: &str, close: &ClosePanel) {
    command::run(command.to_string());
    close.close();
}

/// The battery as the panel shows it.
#[derive(Clone, PartialEq)]
struct Battery {
    capacity: i64,
    level: Level,
    plugged: bool,
    note: String,
}

/// The battery, changing only with what the panel shows of it.
fn battery_state(settings: Settings) -> Memo<Option<Battery>> {
    let upower = settings.upower.data;

    create_memo(move || {
        upower.with(|data| {
            data.system_battery.map(|battery| Battery {
                capacity: battery.capacity,
                level: items::battery_level(
                    &battery,
                    items::THRESHOLDS,
                    items::held(data, &battery),
                ),
                plugged: items::plugged(data),
                note: charge_note(data, &battery),
            })
        })
    })
}

/// One of the battery's fields, its default while there is none.
fn field<T: Default + 'static>(
    state: Memo<Option<Battery>>,
    read: fn(&Battery) -> T,
) -> impl Fn() -> T + Copy + 'static {
    move || state.with(|battery| battery.as_ref().map(read).unwrap_or_default())
}

/// How full the battery is, and how long until full or empty.
fn battery(settings: Settings) -> Container {
    let state = battery_state(settings);
    let present = create_memo(move || state.with(Option::is_some));
    let plugged = field(state, |b| b.plugged);

    container().child(move || {
        present.get().then(|| {
            tag()
                .size(Size::Md)
                .child(
                    level_indicator(Levels::BATTERY, field(state, |b| b.capacity as f32))
                        .level(field(state, |b| b.level))
                        .icon(move || plugged().then_some(Icon::BatteryCharging))
                        .percent(true)
                        .size(Size::Md),
                )
                .child(caption(field(state, |b| b.note.clone())))
        })
    })
}

/// How full the battery is and what it is doing, for the power page's head.
fn battery_status(settings: Settings) -> Memo<String> {
    let state = battery_state(settings);

    create_memo(move || {
        state.with(|battery| match battery {
            Some(b) if b.note.is_empty() => format!("{}%", b.capacity),
            Some(b) => format!("{}% · {}", b.capacity, b.note),
            None => String::new(),
        })
    })
}

/// What the battery is doing, as ashell says it, with the item's time left.
fn charge_note(data: &UPowerData, battery: &BatteryData) -> String {
    let plugged = items::plugged(data);

    match (battery.status, items::time_left(data)) {
        (BatteryStatus::Charging(_), Some(left)) => {
            t!("settings-power-full-in", duration = items::span(left))
        }
        (BatteryStatus::Charging(_), None) => t!("settings-power-status-charging"),
        _ if matches!(battery.status, BatteryStatus::Full)
            || plugged && battery.capacity >= 100 =>
        {
            t!("settings-power-status-full")
        }
        _ if plugged => t!("settings-power-status-not-charging"),
        (_, Some(left)) => t!("settings-power-empty-in", duration = items::span(left)),
        _ => String::new(),
    }
}

/// A power button: one that ends the session comes after the others.
struct PowerButton {
    icon: Icon,
    label: String,
    command: String,
    ends: bool,
}

/// The power buttons that have a command, in the prototype's order.
fn power_commands(config: &super::SettingsConfig) -> Vec<PowerButton> {
    [
        (Icon::Lock, t!("settings-power-lock"), &config.lock, false),
        (
            Icon::Moon,
            t!("settings-power-suspend"),
            &config.suspend,
            false,
        ),
        (
            Icon::HardDrive,
            t!("settings-power-hibernate"),
            &config.hibernate,
            false,
        ),
        (
            Icon::LogOut,
            t!("settings-power-logout"),
            &config.logout,
            false,
        ),
        (
            Icon::Refresh,
            t!("settings-power-reboot"),
            &config.reboot,
            true,
        ),
        (
            Icon::Power,
            t!("settings-power-shutdown"),
            &config.shutdown,
            true,
        ),
    ]
    .into_iter()
    .filter_map(|(icon, label, command, ends)| {
        command.clone().map(|command| PowerButton {
            icon,
            label,
            command,
            ends,
        })
    })
    .collect()
}

/// The power buttons, a row each; those that end the session after a line.
fn power(settings: Settings, stack: RwSignal<Vec<Page>>) -> Container {
    let close = expect_context::<ClosePanel>();
    let button = move |power: PowerButton| {
        let close = close.clone();

        row(power.label)
            .icon(power.icon)
            .tone(if power.ends {
                Tone::Danger
            } else {
                Tone::Neutral
            })
            .on_click(move || run(&power.command, &close))
    };

    column(below_head())
        .child(head(
            stack,
            t!("settings-power"),
            battery_status(settings),
            mark(Icon::Power),
        ))
        .child(scroll_list(list_height()).children(move || {
            let buttons = settings.config.with(power_commands);
            let line = buttons.iter().position(|b| b.ends).filter(|&at| at > 0);

            buttons
                .into_iter()
                .enumerate()
                .flat_map(|(at, power)| {
                    (Some(at) == line)
                        .then(|| separator().into_any())
                        .into_iter()
                        .chain([button(power).into_any()])
                })
                .collect::<Vec<_>>()
        }))
}

/// The brightness, while there is a backlight.
fn brightness(settings: Settings) -> Container {
    let data = settings.brightness.data;

    let percent = create_memo(move || {
        data.get()
            .map(|data| data.current as f32 * 100.0 / data.max.max(1) as f32)
    });
    let present = create_memo(move || percent.get().is_some());

    container().width(fill()).child(move || {
        present.get().then(|| {
            slider_row(Icon::Sun, move || percent.get().unwrap_or(0.0))
                .on_change(move |percent| settings.set_brightness(percent))
        })
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Tile {
    Bluetooth,
    PowerProfile,
    IdleInhibitor,
}

/// Bluetooth, the power profile and the idle inhibitor, each while its
/// service is there, two to a row. As ashell's, one missing leaves no gap:
/// the next takes its place.
fn tiles(settings: Settings, stack: RwSignal<Vec<Page>>) -> Container {
    let bluetooth = settings.bluetooth.data;
    let upower = settings.upower.data;
    let inhibited = settings.idle.inhibited;

    let present = create_memo(move || {
        [
            bluetooth
                .with(|data| data.state != BluetoothState::Unavailable)
                .then_some(Tile::Bluetooth),
            upower
                .with(|data| data.power_profile != PowerProfile::Unknown)
                .then_some(Tile::PowerProfile),
            inhibited.get().is_some().then_some(Tile::IdleInhibitor),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
    });

    let view = move |tile| match tile {
        Tile::Bluetooth => bluetooth_tile(settings, stack),
        Tile::PowerProfile => power_profile(settings, stack),
        Tile::IdleInhibitor => idle_inhibitor(settings),
    };

    container().width(fill()).child(move || {
        column(size::S2.px()).children(
            present
                .get()
                .chunks(2)
                .map(|row| split(row.iter().map(|&tile| view(tile))))
                .collect::<Vec<_>>(),
        )
    })
}

/// `parts` side by side in two equal shares, the second left empty when
/// there is one.
fn split(parts: impl IntoIterator<Item = Container>) -> Container {
    let mut parts: Vec<_> = parts.into_iter().collect();
    if parts.len() == 1 {
        parts.push(container().width(fill()));
    }

    container()
        .width(fill())
        .layout(Flex::row().spacing(size::S2.px()))
        .children(parts)
}

/// The icon turns it on or off, the rest opens its page. Under its name, the
/// devices connected, or whether it is on.
fn bluetooth_tile(settings: Settings, stack: RwSignal<Vec<Page>>) -> Container {
    let data = settings.bluetooth.data;

    let active = create_memo(move || data.with(|data| data.state == BluetoothState::Active));
    let summary = bluetooth_summary(settings);

    container().width(fill()).child(
        tile(Icon::Bluetooth, t!("settings-bluetooth"))
            .subtitle(move || summary.get())
            .active(active)
            .on_toggle(move |_| settings.bluetooth.commands.send(BluetoothCommand::Toggle))
            .on_open(move || push(stack, Page::Bluetooth)),
    )
}

/// The devices connected, or whether Bluetooth is on.
fn bluetooth_summary(settings: Settings) -> Memo<String> {
    let data = settings.bluetooth.data;

    create_memo(move || {
        data.with(|data| match data.state {
            BluetoothState::Active => {
                let connected: Vec<_> = data
                    .devices
                    .iter()
                    .filter(|d| d.connected)
                    .map(|d| d.name.as_str())
                    .collect();

                if connected.is_empty() {
                    t!("settings-on")
                } else {
                    connected.join(", ")
                }
            }
            _ => t!("settings-off"),
        })
    })
}

/// Named after the profile in use, on while it is not balanced: the icon
/// goes to the next profile, the rest opens the page of them all.
fn power_profile(settings: Settings, stack: RwSignal<Vec<Page>>) -> Container {
    let upower = settings.upower.data;

    let profile = create_memo(move || upower.with(|data| data.power_profile));

    container().width(fill()).child(
        tile(
            move || items::power_profile_icon(profile.get()),
            move || profile_name(profile.get()),
        )
        .subtitle(t!("settings-power-mode"))
        .active(move || profile.get() != PowerProfile::Balanced)
        .on_toggle(move |_| {
            settings
                .upower
                .commands
                .send(UPowerCommand::TogglePowerProfile)
        })
        .on_open(move || push(stack, Page::PowerProfiles)),
    )
}

fn profile_name(profile: PowerProfile) -> String {
    match profile {
        PowerProfile::Performance => t!("settings-power-profile-performance"),
        PowerProfile::PowerSaver => t!("settings-power-profile-power-saver"),
        PowerProfile::Balanced | PowerProfile::Unknown => t!("settings-power-profile-balanced"),
    }
}

/// The three profiles, each said as GNOME says it, the one in use checked:
/// a click sets another.
fn power_profiles(settings: Settings, stack: RwSignal<Vec<Page>>) -> Container {
    let upower = settings.upower.data;

    let current = create_memo(move || upower.with(|data| data.power_profile));

    column(below_head())
        .child(head(
            stack,
            t!("settings-power-mode"),
            move || profile_name(current.get()),
            mark(move || items::power_profile_icon(current.get())),
        ))
        .child(
            scroll_list(list_height()).children(
                [
                    (
                        PowerProfile::PowerSaver,
                        t!("settings-power-profile-power-saver-description"),
                    ),
                    (
                        PowerProfile::Balanced,
                        t!("settings-power-profile-balanced-description"),
                    ),
                    (
                        PowerProfile::Performance,
                        t!("settings-power-profile-performance-description"),
                    ),
                ]
                .into_iter()
                .map(|(profile, description)| {
                    row(profile_name(profile))
                        .icon(items::power_profile_icon(profile))
                        .subtitle(description)
                        .checked(move || current.get() == profile)
                        .on_click(move || {
                            settings
                                .upower
                                .commands
                                .send(UPowerCommand::SetPowerProfile(profile))
                        })
                })
                .collect::<Vec<_>>(),
            ),
        )
}

fn idle_inhibitor(settings: Settings) -> Container {
    let inhibited = settings.idle.inhibited;

    // A small tile: half its share of the row.
    split([container().width(fill()).child(
        small_tile(Icon::Coffee)
            .active(move || inhibited.get() == Some(true))
            .on_toggle(move |_| settings.idle.toggle.send(())),
    )])
}

/// The devices: those known, a click connecting or disconnecting one, and
/// those found nearby, a click pairing one. Opening it with Bluetooth on, or
/// turning it on here, looks for devices.
fn bluetooth(settings: Settings, stack: RwSignal<Vec<Page>>) -> Container {
    let data = settings.bluetooth.data;

    let active = create_memo(move || data.with(|data| data.state == BluetoothState::Active));
    let none = create_memo(move || data.with(|data| data.devices.is_empty()));
    let paired = move |paired: bool| {
        move || {
            data.with(|data| {
                data.devices
                    .iter()
                    .filter(|d| d.paired == paired)
                    .cloned()
                    .collect::<Vec<_>>()
            })
        }
    };
    let discovering = create_memo(move || data.with(|data| data.discovering));

    // Not again while a search runs: BlueZ turns the second one down.
    create_effect(move || {
        if active.get() && !data.with_untracked(|data| data.discovering) {
            settings
                .bluetooth
                .commands
                .send(BluetoothCommand::StartDiscovery);
        }
    });

    let header = head(
        stack,
        t!("settings-bluetooth"),
        bluetooth_summary(settings),
        switch(
            move || items::bluetooth_icon(active.get()),
            active,
            move || settings.bluetooth.commands.send(BluetoothCommand::Toggle),
        ),
    );

    column(below_head()).child(header).child(move || {
        if !active.get() {
            return empty(Icon::BluetoothOff, t!("settings-bluetooth-off")).into_any();
        }
        if none.get() && !discovering.get() {
            return empty(Icon::Bluetooth, t!("settings-bluetooth-no-devices")).into_any();
        }

        scroll_list(list_height())
            .child(section(t!("settings-bluetooth-known-devices")))
            .children(keyed(
                paired(true),
                |device| device.path.clone(),
                move |device| known_device(settings, device),
            ))
            .child(section(move || {
                if discovering.get() {
                    t!("settings-scanning")
                } else {
                    t!("settings-bluetooth-available")
                }
            }))
            .children(keyed(
                paired(false),
                |device| device.path.clone(),
                move |device| nearby_device(settings, device),
            ))
            .into_any()
    })
}

/// A paired device: a click connects or disconnects it.
fn known_device(settings: Settings, device: BluetoothDevice) -> impl Widget {
    let path = device.path.clone();
    let command = if device.connected {
        BluetoothCommand::DisconnectDevice(path)
    } else {
        BluetoothCommand::ConnectDevice(path)
    };
    let note = match (device.connected, device.battery) {
        (true, Some(level)) => format!("{} · {level}%", t!("settings-bluetooth-connected")),
        (true, None) => t!("settings-bluetooth-connected"),
        (false, _) => t!("settings-bluetooth-disconnected"),
    };

    row(device.name)
        .icon(Icon::Bluetooth)
        .subtitle(note)
        .selected(device.connected)
        .on_click(move || settings.bluetooth.commands.send(command.clone()))
}

/// A device found nearby: a click pairs it.
fn nearby_device(settings: Settings, device: BluetoothDevice) -> impl Widget {
    let path = device.path;

    row(device.name)
        .icon(Icon::Bluetooth)
        .subtitle(t!("settings-bluetooth-pair"))
        .on_click(move || {
            settings
                .bluetooth
                .commands
                .send(BluetoothCommand::PairDevice(path.clone()))
        })
}

/// A chip for each device that reports a battery: its name and how full.
fn peripherals(settings: Settings) -> Container {
    let upower = settings.upower.data;

    let devices = create_memo(move || upower.with(|data| data.peripherals.clone()));

    container()
        .width(fill())
        .layout(Flex::row().spacing(size::S2.px()).wrap(true))
        .children(keyed(
            move || devices.get(),
            |device| device.path.clone(),
            |device| {
                tag()
                    .size(Size::Sm)
                    .child(
                        level_indicator(Levels(&[]), device.data.capacity as f32)
                            .icon(Some(items::peripheral_icon(device.kind)))
                            .level(items::battery_level(&device.data, items::THRESHOLDS, false)),
                    )
                    .child(
                        typed(
                            format!("{} · {}%", device.name, device.data.capacity),
                            ty::LABEL_XSMALL,
                        )
                        .color(look().eased(|c| c.sub))
                        .nowrap(),
                    )
            },
        ))
}
