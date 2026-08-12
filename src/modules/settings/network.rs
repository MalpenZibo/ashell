use guido::prelude::*;

use crate::components::{
    ButtonKind, ButtonSize, IconKind, StaticIcon, bar_indicator, buttons::icon_button,
    quick_setting, selectable_item, toggle_button,
};
use crate::config::SettingsFormat;
use crate::services::compat::ServiceSignal;
use crate::services::network::{
    ActiveConnectionInfo, KnownConnection, NetworkCommand, NetworkService, dbus::DeviceState,
};
use crate::theme::ThemeColors;

fn wifi_strength(acs: &[ActiveConnectionInfo]) -> Option<u8> {
    acs.iter().find_map(|ac| match ac {
        ActiveConnectionInfo::WiFi { strength, .. } => Some(*strength),
        _ => None,
    })
}

fn wifi_icon(acs: &[ActiveConnectionInfo], wifi_enabled: bool) -> StaticIcon {
    if !wifi_enabled {
        return StaticIcon::Wifi0;
    }
    wifi_strength(acs)
        .map(|s| match s {
            0..=20 => StaticIcon::Wifi1,
            21..=40 => StaticIcon::Wifi2,
            41..=60 => StaticIcon::Wifi3,
            61..=80 => StaticIcon::Wifi4,
            _ => StaticIcon::Wifi5,
        })
        .unwrap_or(StaticIcon::Wifi0)
}

fn has_active_vpn(acs: &[ActiveConnectionInfo]) -> bool {
    acs.iter()
        .any(|ac| matches!(ac, ActiveConnectionInfo::Vpn { .. }))
}

/// Bar indicator: WiFi icon and/or signal strength %
pub fn wifi_indicator(data: ServiceSignal<NetworkService>, format: SettingsFormat) -> impl Widget {
    let theme = expect_context::<ThemeColors>();

    bar_indicator()
        .kind(move || -> IconKind {
            data.with(|s| {
                s.as_ref()
                    .map(|x| wifi_icon(&x.active_connections, x.wifi_enabled))
                    .unwrap_or(StaticIcon::Wifi0)
            })
            .into()
        })
        .label(move || {
            Some(
                data.with(|s| {
                    s.as_ref()
                        .and_then(|x| wifi_strength(&x.active_connections))
                })
                .map(|s| format!("{s}%"))
                .unwrap_or_else(|| "0%".to_string()),
            )
        })
        .color(theme.text)
        .format(format)
}

/// WiFi quick setting tile
pub fn wifi_quick_setting(
    data: ServiceSignal<NetworkService>,
    svc: Service<NetworkCommand>,
    on_submenu: impl Fn() + 'static,
    expanded: impl Fn() -> bool + 'static,
) -> impl Widget {
    let svc_toggle = svc;
    let wifi_enabled = move || data.with(|s| s.as_ref().is_some_and(|x| x.wifi_enabled));

    quick_setting()
        .kind(move || {
            data.with(|s| {
                s.as_ref()
                    .filter(|x| x.wifi_enabled)
                    .map(|x| wifi_icon(&x.active_connections, x.wifi_enabled))
                    .unwrap_or(StaticIcon::Wifi0)
            })
        })
        .title(move || "Wi-Fi".to_string())
        .subtitle(move || {
            data.with(|s| {
                s.as_ref()
                    .filter(|x| x.wifi_enabled)
                    .and_then(|x| {
                        x.active_connections.iter().find_map(|ac| match ac {
                            ActiveConnectionInfo::WiFi { name, .. } => Some(name.clone()),
                            _ => None,
                        })
                    })
                    .unwrap_or_default()
            })
        })
        .active(wifi_enabled)
        .on_toggle(move || svc_toggle.send(NetworkCommand::ToggleWiFi))
        .on_submenu(on_submenu)
        .expanded(expanded)
}

/// Airplane mode quick setting
pub fn airplane_quick_setting(
    data: ServiceSignal<NetworkService>,
    svc: Service<NetworkCommand>,
) -> impl Widget {
    let airplane = move || data.with(|s| s.as_ref().is_some_and(|x| x.airplane_mode));
    let svc_toggle = svc;

    quick_setting()
        .kind(move || StaticIcon::Airplane)
        .title(move || "Airplane".to_string())
        .subtitle(move || {
            if airplane() {
                "On".to_string()
            } else {
                "Off".to_string()
            }
        })
        .active(airplane)
        .on_toggle(move || svc_toggle.send(NetworkCommand::ToggleAirplaneMode))
}

/// VPN quick setting
///
/// Mimics ashell behavior:
/// - Inactive: clicking the tile opens the VPN submenu (no chevron shown)
/// - Active: clicking the tile toggles VPN off, chevron opens submenu
pub fn vpn_quick_setting(
    data: ServiceSignal<NetworkService>,
    svc: Service<NetworkCommand>,
    on_submenu: impl Fn() + 'static + Clone,
    expanded: impl Fn() -> bool + 'static,
) -> impl Widget {
    let svc_toggle = svc;
    let on_submenu_for_toggle = on_submenu.clone();

    quick_setting()
        .kind(move || StaticIcon::Vpn)
        .title(move || "VPN".to_string())
        .subtitle(move || {
            data.with(|s| {
                s.as_ref()
                    .and_then(|x| {
                        x.active_connections.iter().find_map(|ac| match ac {
                            ActiveConnectionInfo::Vpn { name, .. } => Some(name.clone()),
                            _ => None,
                        })
                    })
                    .unwrap_or_default()
            })
        })
        .active(move || {
            data.with(|s| {
                s.as_ref()
                    .is_some_and(|x| has_active_vpn(&x.active_connections))
            })
        })
        .on_toggle(move || {
            // Toggle the first ACTIVE vpn off (upstream semantics), not the
            // first known one
            let vpn = data.with(|s| {
                let x = s.as_ref()?;
                let active_name = x.active_connections.iter().find_map(|ac| match ac {
                    ActiveConnectionInfo::Vpn { name, .. } => Some(name.clone()),
                    _ => None,
                })?;
                x.known_connections.iter().find_map(|k| match k {
                    KnownConnection::Vpn(v) if v.name == active_name => Some(v.clone()),
                    _ => None,
                })
            });
            match vpn {
                // Active: toggle it off
                Some(v) => svc_toggle.send(NetworkCommand::ToggleVpn(v)),
                // Inactive: open the submenu
                None => on_submenu_for_toggle(),
            }
        })
        .on_submenu(on_submenu)
        .expanded(expanded)
}

/// WiFi submenu: list of known/available access points
pub fn wifi_submenu(
    data: ServiceSignal<NetworkService>,
    svc: Service<NetworkCommand>,
    settings: super::SettingsSignals,
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();

    container()
        .width(fill())
        .layout(Flex::column().spacing(4))
        .child(
            container()
                .width(fill())
                .layout(
                    Flex::row()
                        .main_alignment(MainAlignment::SpaceBetween)
                        .cross_alignment(CrossAlignment::Center),
                )
                .child(text("WiFi Networks").color(theme.text).font_size(14))
                .child({
                    let svc_scan = svc;
                    icon_button()
                        .icon(StaticIcon::Refresh)
                        .size(ButtonSize::Small)
                        .kind(ButtonKind::Transparent)
                        .on_click(move || svc_scan.send(NetworkCommand::ScanNearByWiFi))
                }),
        )
        .child(move || {
            let (known_list, ap_list) = data.with(|s| {
                s.as_ref()
                    .map(|x| {
                        (
                            x.known_connections.clone(),
                            x.wireless_access_points.clone(),
                        )
                    })
                    .unwrap_or_default()
            });
            let mut col = container()
                .width(fill())
                .height(at_most(250))
                .scrollable(ScrollAxis::Vertical)
                .layout(Flex::column().spacing(2));

            // Known connections first
            for kc in &known_list {
                if let KnownConnection::AccessPoint(ap) = kc {
                    let ssid = ap.ssid.clone();
                    let strength = ap.strength;
                    let ap_clone = ap.clone();
                    let is_connected = ap.state == DeviceState::Activated;
                    col = col.child(
                        selectable_item()
                            .kind(strength_to_icon(strength, true))
                            .label(ssid)
                            .selected(is_connected)
                            .on_click(move || {
                                svc.send(NetworkCommand::SelectAccessPoint((
                                    ap_clone.clone(),
                                    None,
                                )));
                            }),
                    );
                }
            }

            // Then other visible APs not in known list; connecting asks for
            // a password (secured) or confirmation (open network)
            let known_ssids: Vec<_> = known_list
                .iter()
                .filter_map(|kc| match kc {
                    KnownConnection::AccessPoint(ap) => Some(ap.ssid.clone()),
                    _ => None,
                })
                .collect();
            for ap in &ap_list {
                if known_ssids.contains(&ap.ssid) || ap.ssid.is_empty() {
                    continue;
                }
                let ssid = ap.ssid.clone();
                let strength = ap.strength;
                let is_public = ap.public;
                let dialog = settings.network_dialog;
                let password = settings.dialog_password;
                let show_password = settings.dialog_show_password;
                col = col.child(
                    selectable_item()
                        .kind(strength_to_icon(strength, is_public))
                        .label(ssid.clone())
                        .selected(false)
                        .on_click(move || {
                            password.set(String::new());
                            show_password.set(false);
                            dialog.set(Some(if is_public {
                                super::NetworkDialog::OpenNetwork { ssid: ssid.clone() }
                            } else {
                                super::NetworkDialog::Password { ssid: ssid.clone() }
                            }));
                        }),
                );
            }
            Some(col)
        })
}

/// VPN submenu: list of known VPNs with toggle switches
pub fn vpn_submenu(
    data: ServiceSignal<NetworkService>,
    svc: Service<NetworkCommand>,
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();

    // Reactive: added/removed VPN profiles appear while the submenu is open
    container().width(fill()).child(move || {
        let mut known_list: Vec<_> = data.with(|s| {
            s.as_ref()
                .map(|x| {
                    x.known_connections
                        .iter()
                        .filter_map(|k| match k {
                            KnownConnection::Vpn(v) => Some(v.clone()),
                            _ => None,
                        })
                        .collect()
                })
                .unwrap_or_default()
        });
        known_list.sort_by_key(|v| v.name.clone());

        let mut col = container()
            .width(fill())
            .height(at_most(300))
            .scrollable(ScrollAxis::Vertical)
            .layout(Flex::column().spacing(4));

        for vpn in known_list {
            let vpn_name = vpn.name.clone();
            let vpn_clone = vpn.clone();
                let name_for_active = vpn_name.clone();

            col = col.child(
                container()
                    .width(fill())
                    .height(32)
                    .padding([0, 8])
                    .layout(
                        Flex::row()
                            .main_alignment(MainAlignment::SpaceBetween)
                            .cross_alignment(CrossAlignment::Center),
                    )
                    .child(text(vpn_name).color(theme.text).font_size(12))
                    .child(
                        toggle_button()
                            .active(move || {
                                data.with(|s| {
                                    s.as_ref().is_some_and(|x| {
                                        x.active_connections.iter().any(|ac| matches!(
                                            ac,
                                            ActiveConnectionInfo::Vpn { name, .. } if *name == name_for_active
                                        ))
                                    })
                                })
                            })
                            .on_toggle(move || {
                                svc.send(NetworkCommand::ToggleVpn(vpn_clone.clone()));
                            }),
                    ),
            );
        }
        Some(col)
    })
}

fn strength_to_icon(strength: u8, public: bool) -> StaticIcon {
    if public {
        match strength {
            0..=20 => StaticIcon::Wifi1,
            21..=40 => StaticIcon::Wifi2,
            41..=60 => StaticIcon::Wifi3,
            61..=80 => StaticIcon::Wifi4,
            _ => StaticIcon::Wifi5,
        }
    } else {
        match strength {
            0..=20 => StaticIcon::WifiLock1,
            21..=40 => StaticIcon::WifiLock2,
            41..=60 => StaticIcon::WifiLock3,
            61..=80 => StaticIcon::WifiLock4,
            _ => StaticIcon::WifiLock5,
        }
    }
}

/// Full-menu takeover for connecting to a new network: password entry for
/// secured APs, a warning for open ones (upstream password_dialog).
pub fn network_dialog_view(
    settings: super::SettingsSignals,
    dialog: super::NetworkDialog,
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let data = settings.network_data;
    let svc = settings.network_svc;
    let dialog_sig = settings.network_dialog;
    let password = settings.dialog_password;
    let show_password = settings.dialog_show_password;

    let (title, body, ssid, warning_only) = match &dialog {
        super::NetworkDialog::Password { ssid } => (
            crate::t!("password-dialog-authentication-required-title"),
            crate::t!("password-dialog-insert-password", ssid = ssid.clone()),
            ssid.clone(),
            false,
        ),
        super::NetworkDialog::OpenNetwork { ssid } => (
            crate::t!("password-dialog-open-network-title"),
            crate::t!("password-dialog-open-network-warning", ssid = ssid.clone()),
            ssid.clone(),
            true,
        ),
    };

    let connect = {
        let ssid = ssid.clone();
        move || {
            let pw = password.with(|p| p.clone());
            let pw = if warning_only || pw.is_empty() {
                None
            } else {
                Some(pw)
            };
            let ap = data.with(|s| {
                s.as_ref().and_then(|x| {
                    x.wireless_access_points
                        .iter()
                        .find(|ap| ap.ssid == ssid)
                        .cloned()
                })
            });
            match ap {
                Some(ap) => svc.send(NetworkCommand::SelectAccessPoint((ap, pw))),
                None => log::warn!(
                    "Unable to confirm dialog: access point '{ssid}' no longer available"
                ),
            }
            password.set(String::new());
            dialog_sig.set(None);
        }
    };
    let connect_submit = connect.clone();

    let mut col = container()
        .width(fill())
        .layout(Flex::column().spacing(12))
        .child(text(title).color(theme.text).font_size(16))
        .child(text(body).color(theme.text).font_size(12));

    if !warning_only {
        col = col.child(
            container()
                .width(fill())
                .layout(
                    Flex::row()
                        .spacing(8)
                        .cross_alignment(CrossAlignment::Center),
                )
                .child(
                    container()
                        .width(fill())
                        .padding([6, 8])
                        .corner_radius(8)
                        .background(theme.background.lighter(0.1))
                        // Rebuilt on visibility toggle; the text itself lives
                        // in the signal so it survives
                        .child(move || {
                            let connect_submit = connect_submit.clone();
                            Some(
                                text_input(password)
                                    .password(!show_password.get())
                                    .text_color(theme.text)
                                    .font_size(13)
                                    .on_submit(move |_| connect_submit()),
                            )
                        }),
                )
                .child(
                    icon_button()
                        .icon(move || -> IconKind {
                            if show_password.get() {
                                StaticIcon::EyeOpened.into()
                            } else {
                                StaticIcon::EyeClosed.into()
                            }
                        })
                        .kind(ButtonKind::Transparent)
                        .on_click(move || show_password.update(|v| *v = !*v)),
                ),
        );
    }

    col.child(
        container()
            .width(fill())
            .layout(
                Flex::row()
                    .spacing(8)
                    .main_alignment(MainAlignment::End)
                    .cross_alignment(CrossAlignment::Center),
            )
            .child(
                crate::components::button()
                    .kind(ButtonKind::Transparent)
                    .content(
                        text(crate::t!("password-dialog-cancel"))
                            .color(theme.text)
                            .font_size(13),
                    )
                    .on_click(move || {
                        password.set(String::new());
                        dialog_sig.set(None);
                    }),
            )
            .child(
                crate::components::button()
                    .content(
                        text(crate::t!("password-dialog-confirm"))
                            .color(theme.text)
                            .font_size(13),
                    )
                    .on_click(move || {
                        if warning_only || !password.with(|p| p.is_empty()) {
                            connect();
                        }
                    }),
            ),
    )
}
