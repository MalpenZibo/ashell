use guido::prelude::*;

use crate::components::{ButtonKind, IconKind, StaticIcon, button, icon, quick_setting};
use crate::config::SettingsFormat;
use crate::modules::settings::SubMenu;
use crate::services::compat::ServiceSignal;
use crate::services::upower::{
    BatteryData, BatteryStatus, PowerProfile, UPowerCommand, UPowerService,
};

// Upstream BatteryData has no Default; the #[component] prop machinery
// needs one. Neutral zero-value, never rendered (props are always set).
impl Default for BatteryData {
    fn default() -> Self {
        Self {
            capacity: 0,
            status: BatteryStatus::Unknown,
            is_discharging: false,
        }
    }
}
use crate::theme::ThemeColors;
use crate::{IndicatorState, format_duration};

fn format_time_for_battery(battery: &BatteryData) -> String {
    match battery.status {
        BatteryStatus::Charging(duration) => {
            if battery.capacity >= 100 || duration.is_zero() {
                "100%".to_string()
            } else {
                format_duration(&duration)
            }
        }
        BatteryStatus::Discharging(duration) => {
            if battery.capacity >= 100 {
                "100%".to_string()
            } else if duration.is_zero() {
                "Calculating...".to_string()
            } else {
                format_duration(&duration)
            }
        }
        BatteryStatus::Full => "100%".to_string(),
        BatteryStatus::NotCharging | BatteryStatus::Unknown => {
            format!("{}%", battery.capacity)
        }
    }
}

pub(crate) fn battery_label(battery: &BatteryData, format: SettingsFormat) -> Option<String> {
    match format {
        SettingsFormat::Percentage | SettingsFormat::IconAndPercentage => {
            Some(format!("{}%", battery.capacity))
        }
        SettingsFormat::Time | SettingsFormat::IconAndTime => {
            Some(format_time_for_battery(battery))
        }
        SettingsFormat::Icon => None,
    }
}

pub(crate) fn battery_color(battery: &BatteryData, theme: &ThemeColors) -> Color {
    match battery.get_indicator_state() {
        IndicatorState::Success => theme.success,
        IndicatorState::Danger => theme.danger,
        _ => theme.text,
    }
}

/// Power profile quick setting
pub fn power_profile_quick_setting(
    data: ServiceSignal<UPowerService>,
    svc: Service<UPowerCommand>,
) -> impl Widget {
    let profile = create_memo(move || {
        data.with(|s| s.as_ref().map(|x| x.power_profile))
            .unwrap_or_default()
    });
    let svc_toggle = svc.clone();

    quick_setting()
        .kind(move || StaticIcon::from(profile.get()))
        .title(move || match profile.get() {
            PowerProfile::Balanced => "Balanced".to_string(),
            PowerProfile::Performance => "Performance".to_string(),
            PowerProfile::PowerSaver => "Power Saver".to_string(),
            PowerProfile::Unknown => "Unknown".to_string(),
        })
        .subtitle(String::new)
        .active(move || profile.get() != PowerProfile::Unknown)
        .on_toggle(move || svc_toggle.send(UPowerCommand::TogglePowerProfile))
}

#[component]
pub fn menu_indicator(battery: BatteryData, peripheral_icon: Option<IconKind>) -> impl Widget {
    let theme = expect_context::<ThemeColors>();

    let state = battery.get().get_indicator_state();

    let text_color = match state {
        IndicatorState::Success => theme.success,
        IndicatorState::Danger => theme.danger,
        _ => theme.text,
    };

    let battery_info = container()
        .layout(Flex::row().spacing(4))
        .maybe_child(
            peripheral_icon
                .get()
                .map(|ic| icon().kind(ic).color(text_color)),
        )
        .child(icon().kind(battery.get().get_icon()).color(text_color))
        .child(text(format!("{}%", battery.get().capacity)).color(text_color));

    let capacity = battery.get().capacity;

    container()
        .layout(Flex::row().spacing(4))
        .padding([4, 2])
        .child(battery_info)
        .maybe_child(match battery.get().status {
            BatteryStatus::Charging(remaining) if capacity < 95 => {
                Some(text(format!("Full in {}", format_duration(&remaining))).color(text_color))
            }
            BatteryStatus::Discharging(remaining) if capacity < 95 && !remaining.is_zero() => {
                Some(text(format!("Empty in {}", format_duration(&remaining))).color(text_color))
            }
            _ => None,
        })
}

/// Battery/peripheral indicator in menu header.
pub fn battery_header(
    data: ServiceSignal<UPowerService>,
    submenu: RwSignal<Option<SubMenu>>,
) -> impl Widget {
    container().child(move || -> Option<AnyWidget> {
        data.with(|s| {
            let s = s.as_ref()?;
            s.system_battery
                .clone()
                .map(|b| {
                    let indicator = menu_indicator().battery(b);
                    let has_peripherals = !s.peripherals.is_empty();

                    if has_peripherals {
                        button()
                            .content(indicator)
                            .on_click(move || {
                                submenu.set(if submenu.get() == Some(SubMenu::Peripherals) {
                                    None
                                } else {
                                    Some(SubMenu::Peripherals)
                                });
                            })
                            .into_any()
                    } else {
                        indicator.into_any()
                    }
                })
                .or_else(|| {
                    let periphs = &s.peripherals;
                    periphs.first().map(|p| {
                        let indicator = menu_indicator()
                            .battery(p.data.clone())
                            .peripheral_icon(Some(p.kind.get_icon().into()));

                        if periphs.len() > 1 {
                            button()
                                .content(indicator)
                                .on_click(move || {
                                    submenu.set(if submenu.get() == Some(SubMenu::Peripherals) {
                                        None
                                    } else {
                                        Some(SubMenu::Peripherals)
                                    });
                                })
                                .into_any()
                        } else {
                            indicator.into_any()
                        }
                    })
                })
        })
    })
}

/// Peripherals section in menu
pub fn peripherals_view(data: ServiceSignal<UPowerService>) -> impl Widget {
    let theme = expect_context::<ThemeColors>();

    container().width(fill()).child(move || {
        let periphs = data.with(|s| {
            s.as_ref()
                .map(|x| x.peripherals.clone())
                .unwrap_or_default()
        });
        if periphs.is_empty() {
            return Some(container());
        }
        let mut col = container().width(fill()).layout(Flex::column().spacing(4));
        for p in &periphs {
            let name = p.name.clone();
            let kind = p.kind.get_icon();
            let pct = p.data.capacity;
            col = col.child(
                container()
                    .width(fill())
                    .padding([4, 8])
                    .layout(
                        Flex::row()
                            .spacing(8)
                            .cross_alignment(CrossAlignment::Center),
                    )
                    .child(icon().kind(kind).color(theme.text).font_size(14))
                    .child(text(name).color(theme.text).font_size(12))
                    .child(text(format!("{pct}%")).color(theme.primary).font_size(12)),
            );
        }
        Some(col)
    })
}

/// Power actions menu. Commands come from config and run through
/// `bash -c` (upstream launcher semantics); the menu stays open, like
/// upstream. Hibernate only appears when hibernate_cmd is set.
pub fn power_actions(close_menu: impl Fn() + 'static + Clone) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let cfg = with_context::<crate::config::Config, _>(|c| c.settings.clone()).unwrap();
    // Upstream keeps the menu open after power actions
    let _ = close_menu;

    container()
        .width(fill())
        .layout(Flex::column().spacing(2))
        .child(power_action_btn(
            theme,
            StaticIcon::Suspend,
            crate::t!("settings-power-suspend"),
            cfg.suspend_cmd.clone(),
        ))
        .maybe_child(cfg.hibernate_cmd.clone().map(|cmd| {
            power_action_btn(
                theme,
                StaticIcon::Hibernate,
                crate::t!("settings-power-hibernate"),
                cmd,
            )
        }))
        .child(power_action_btn(
            theme,
            StaticIcon::Reboot,
            crate::t!("settings-power-reboot"),
            cfg.reboot_cmd.clone(),
        ))
        .child(power_action_btn(
            theme,
            StaticIcon::Power,
            crate::t!("settings-power-shutdown"),
            cfg.shutdown_cmd.clone(),
        ))
        .child(crate::components::divider())
        .child(power_action_btn(
            theme,
            StaticIcon::Logout,
            crate::t!("settings-power-logout"),
            cfg.logout_cmd.clone(),
        ))
}

fn power_action_btn(theme: ThemeColors, ic: StaticIcon, label: String, cmd: String) -> impl Widget {
    button()
        .kind(ButtonKind::Transparent)
        .fill_width(true)
        .content(
            container()
                .layout(
                    Flex::row()
                        .spacing(8)
                        .cross_alignment(CrossAlignment::Center),
                )
                .child(icon().kind(ic).color(theme.text).font_size(14))
                .child(text(label).color(theme.text).font_size(14)),
        )
        .on_click(move || crate::utils::launcher::execute_command(&cmd))
}
