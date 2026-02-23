use guido::prelude::*;

use crate::components::{StaticIcon, icon, quick_setting};
use crate::services::upower::{
    BatteryStatus, PowerProfile, UPowerCmd, UPowerDataSignals,
};
use crate::theme;

/// Bar indicator: battery icon + percentage
pub fn battery_indicator(data: UPowerDataSignals) -> impl Widget {
    let battery = data.system_battery;

    container()
        .child(move || {
            battery.with(|bat| {
                bat.map(|b| {
                    container()
                        .layout(
                            Flex::row()
                                .spacing(4.0)
                                .cross_alignment(CrossAlignment::Center),
                        )
                        .child(icon(b.get_icon()).color(theme::TEXT).font_size(14.0))
                        .child(
                            text(format!("{}%", b.capacity))
                                .color(theme::TEXT)
                                .font_size(13.0),
                        )
                })
            })
        })
}

/// Power profile quick setting
pub fn power_profile_quick_setting(
    data: UPowerDataSignals,
    svc: Service<UPowerCmd>,
) -> impl Widget {
    let profile = data.power_profile;
    let svc_toggle = svc.clone();

    quick_setting(
        move || StaticIcon::from(profile.get()),
        move || "Power Saver".to_string(),
        move || match profile.get() {
            PowerProfile::Balanced => "Balanced".to_string(),
            PowerProfile::Performance => "Performance".to_string(),
            PowerProfile::PowerSaver => "Power Saver".to_string(),
            PowerProfile::Unknown => "Unknown".to_string(),
        },
        move || profile.get() != PowerProfile::Unknown,
        move || svc_toggle.send(UPowerCmd::TogglePowerProfile),
        None::<fn()>,
    )
}

/// Battery section in menu header
pub fn battery_header(data: UPowerDataSignals) -> impl Widget {
    let battery = data.system_battery;

    container()
        .layout(
            Flex::row()
                .spacing(6.0)
                .cross_alignment(CrossAlignment::Center),
        )
        .child(move || {
            battery.with(|bat| {
                bat.map(|b| {
                    container()
                        .layout(
                            Flex::row()
                                .spacing(4.0)
                                .cross_alignment(CrossAlignment::Center),
                        )
                        .child(icon(b.get_icon()).color(theme::TEXT).font_size(16.0))
                        .child(
                            text(format!("{}%", b.capacity))
                                .color(theme::TEXT)
                                .font_size(14.0),
                        )
                        .child(move || {
                            let status_text = match b.status {
                                BatteryStatus::Charging(dur) => {
                                    let secs = dur.as_secs();
                                    if secs > 0 {
                                        let h = secs / 3600;
                                        let m = (secs % 3600) / 60;
                                        Some(format!("{h}h {m}m to full"))
                                    } else {
                                        Some("Charging".to_string())
                                    }
                                }
                                BatteryStatus::Discharging(dur) => {
                                    let secs = dur.as_secs();
                                    if secs > 0 {
                                        let h = secs / 3600;
                                        let m = (secs % 3600) / 60;
                                        Some(format!("{h}h {m}m remaining"))
                                    } else {
                                        None
                                    }
                                }
                                BatteryStatus::Full => Some("Full".to_string()),
                            };
                            status_text.map(|s| text(s).color(theme::LAVENDER).font_size(11.0))
                        })
                })
            })
        })
}

/// Peripherals section in menu
pub fn peripherals_view(data: UPowerDataSignals) -> impl Widget {
    let peripherals = data.peripherals;

    container()
        .width(fill())
        .child(move || {
            let periphs = peripherals.with(|p| p.clone());
            if periphs.is_empty() {
                return Some(container());
            }
            let mut col = container()
                .width(fill())
                .layout(Flex::column().spacing(4.0));
            for p in &periphs {
                let name = p.name.clone();
                let ic = p.kind.get_icon();
                let pct = p.data.capacity;
                col = col.child(
                    container()
                        .width(fill())
                        .padding([4.0, 8.0])
                        .layout(
                            Flex::row()
                                .spacing(8.0)
                                .cross_alignment(CrossAlignment::Center),
                        )
                        .child(icon(ic).color(theme::TEXT).font_size(14.0))
                        .child(text(name).color(theme::TEXT).font_size(12.0))
                        .child(
                            text(format!("{pct}%"))
                                .color(theme::LAVENDER)
                                .font_size(12.0),
                        ),
                );
            }
            Some(col)
        })
}

/// Power actions menu
pub fn power_actions(close_menu: impl Fn() + 'static + Clone) -> impl Widget {
    let close1 = close_menu.clone();
    let close2 = close_menu.clone();
    let close3 = close_menu.clone();
    let close4 = close_menu.clone();
    let close5 = close_menu.clone();

    container()
        .width(fill())
        .layout(Flex::column().spacing(2.0))
        .child(power_action_button(StaticIcon::Suspend, "Suspend", move || {
            let _ = std::process::Command::new("systemctl")
                .arg("suspend")
                .spawn();
            close1();
        }))
        .child(power_action_button(StaticIcon::Hibernate, "Hibernate", move || {
            let _ = std::process::Command::new("systemctl")
                .arg("hibernate")
                .spawn();
            close2();
        }))
        .child(power_action_button(StaticIcon::Reboot, "Reboot", move || {
            let _ = std::process::Command::new("systemctl")
                .arg("reboot")
                .spawn();
            close3();
        }))
        .child(power_action_button(StaticIcon::Power, "Shutdown", move || {
            let _ = std::process::Command::new("systemctl")
                .arg("poweroff")
                .spawn();
            close4();
        }))
        .child(power_action_button(StaticIcon::Logout, "Logout", move || {
            let _ = std::process::Command::new("loginctl")
                .arg("terminate-user")
                .arg(std::env::var("USER").unwrap_or_default())
                .spawn();
            close5();
        }))
}

fn power_action_button(
    ic: StaticIcon,
    label: &'static str,
    on_click: impl Fn() + 'static,
) -> impl Widget {
    let hovered = create_signal(false);
    container()
        .width(fill())
        .padding([6.0, 8.0])
        .corner_radius(8.0)
        .on_hover(move |h| hovered.set(h))
        .on_click(move || on_click())
        .background(move || {
            if hovered.get() {
                Color::rgba(1.0, 1.0, 1.0, 0.1)
            } else {
                Color::TRANSPARENT
            }
        })
        .layout(
            Flex::row()
                .spacing(8.0)
                .cross_alignment(CrossAlignment::Center),
        )
        .child(icon(ic).color(theme::TEXT).font_size(14.0))
        .child(text(label).color(theme::TEXT).font_size(14.0))
}
