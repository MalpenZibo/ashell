use guido::prelude::*;

use crate::components::{StaticIcon, icon, quick_setting};
use crate::services::network::{
    ActiveConnectionInfo, KnownConnection, NetworkCmd, NetworkDataSignals,
};
use crate::theme::ThemeColors;

/// Bar indicator: WiFi icon + SSID
pub fn wifi_indicator(data: NetworkDataSignals) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let active = data.active_connections;
    let wifi_enabled = data.wifi_enabled;

    container()
        .layout(
            Flex::row()
                .spacing(4.0)
                .cross_alignment(CrossAlignment::Center),
        )
        .child(
            icon(move || wifi_icon(active, wifi_enabled))
                .color(theme.text)
                .font_size(14.0),
        )
}

fn wifi_icon(
    active: Signal<Vec<ActiveConnectionInfo>>,
    wifi_enabled: Signal<bool>,
) -> StaticIcon {
    if !wifi_enabled.get() {
        return StaticIcon::Wifi0;
    }
    active.with(|acs| {
        acs.iter()
            .find_map(|ac| match ac {
                ActiveConnectionInfo::WiFi { strength, .. } => Some(*strength),
                _ => None,
            })
            .map(|s| match s {
                0..=20 => StaticIcon::Wifi1,
                21..=40 => StaticIcon::Wifi2,
                41..=60 => StaticIcon::Wifi3,
                61..=80 => StaticIcon::Wifi4,
                _ => StaticIcon::Wifi5,
            })
            .unwrap_or(StaticIcon::Wifi0)
    })
}

/// WiFi quick setting tile
pub fn wifi_quick_setting(
    data: NetworkDataSignals,
    svc: Service<NetworkCmd>,
    on_submenu: impl Fn() + 'static,
) -> impl Widget {
    let wifi_enabled = data.wifi_enabled;
    let active = data.active_connections;
    let svc_toggle = svc.clone();

    quick_setting(
        move || {
            if wifi_enabled.get() {
                wifi_icon(active, wifi_enabled)
            } else {
                StaticIcon::Wifi0
            }
        },
        move || "Wi-Fi".to_string(),
        move || {
            if !wifi_enabled.get() {
                return "Off".to_string();
            }
            active.with(|acs| {
                acs.iter()
                    .find_map(|ac| match ac {
                        ActiveConnectionInfo::WiFi { name, .. } => Some(name.clone()),
                        _ => None,
                    })
                    .unwrap_or_default()
            })
        },
        move || wifi_enabled.get(),
        move || svc_toggle.send(NetworkCmd::ToggleWiFi(wifi_enabled.get())),
        Some(on_submenu),
    )
}

/// Airplane mode quick setting
pub fn airplane_quick_setting(
    data: NetworkDataSignals,
    svc: Service<NetworkCmd>,
) -> impl Widget {
    let airplane = data.airplane_mode;
    let svc_toggle = svc.clone();

    quick_setting(
        move || StaticIcon::Airplane,
        move || "Airplane".to_string(),
        move || {
            if airplane.get() {
                "On".to_string()
            } else {
                "Off".to_string()
            }
        },
        move || airplane.get(),
        move || svc_toggle.send(NetworkCmd::ToggleAirplaneMode(airplane.get())),
        None::<fn()>,
    )
}

/// VPN quick setting
pub fn vpn_quick_setting(
    data: NetworkDataSignals,
    svc: Service<NetworkCmd>,
    on_submenu: impl Fn() + 'static,
) -> impl Widget {
    let active = data.active_connections;
    let known = data.known_connections;
    let svc_toggle = svc.clone();

    quick_setting(
        move || StaticIcon::Vpn,
        move || "VPN".to_string(),
        move || {
            active.with(|acs| {
                acs.iter()
                    .find_map(|ac| match ac {
                        ActiveConnectionInfo::Vpn { name, .. } => Some(name.clone()),
                        _ => None,
                    })
                    .unwrap_or("Off".to_string())
            })
        },
        move || {
            active.with(|acs| {
                acs.iter().any(|ac| matches!(ac, ActiveConnectionInfo::Vpn { .. }))
            })
        },
        move || {
            // Toggle first known VPN — read state on main thread
            let vpn = known.with(|kc| {
                kc.iter().find_map(|k| match k {
                    KnownConnection::Vpn(v) => Some(v.clone()),
                    _ => None,
                })
            });
            if let Some(v) = vpn {
                let active_path = active.with(|acs| {
                    acs.iter().find_map(|ac| match ac {
                        ActiveConnectionInfo::Vpn { name, object_path } if *name == v.name => {
                            Some(object_path.clone())
                        }
                        _ => None,
                    })
                });
                svc_toggle.send(NetworkCmd::ToggleVpn(v, active_path));
            }
        },
        Some(on_submenu),
    )
}

/// WiFi submenu: list of known/available access points
pub fn wifi_submenu(
    data: NetworkDataSignals,
    svc: Service<NetworkCmd>,
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let known = data.known_connections;
    let aps = data.wireless_access_points;
    let scanning = data.scanning_nearby_wifi;

    container()
        .width(fill())
        .layout(Flex::column().spacing(4.0))
        .child(
            container()
                .width(fill())
                .layout(
                    Flex::row()
                        .main_alignment(MainAlignment::SpaceBetween)
                        .cross_alignment(CrossAlignment::Center),
                )
                .child(text("WiFi Networks").color(theme.text).font_size(14.0))
                .child({
                    let svc_scan = svc.clone();
                    let hovered = create_signal(false);
                    container()
                        .padding([4.0, 8.0])
                        .corner_radius(6.0)
                        .on_hover(move |h| hovered.set(h))
                        .on_click(move || svc_scan.send(NetworkCmd::ScanNearByWiFi))
                        .background(move || {
                            if hovered.get() {
                                Color::rgba(1.0, 1.0, 1.0, 0.1)
                            } else {
                                Color::TRANSPARENT
                            }
                        })
                        .child(move || {
                            Some(if scanning.get() {
                                icon(StaticIcon::Refresh).color(theme.text).font_size(12.0)
                            } else {
                                icon(StaticIcon::Refresh).color(theme.text).font_size(12.0)
                            })
                        })
                }),
        )
        .child(move || {
            let known_list = known.with(|k| k.clone());
            let ap_list = aps.with(|a| a.clone());
            let mut col = container()
                .width(fill())
                .height(at_most(250.0))
                .scrollable(ScrollAxis::Vertical)
                .layout(Flex::column().spacing(2.0));

            // Known connections first
            for kc in &known_list {
                if let KnownConnection::AccessPoint(ap) = kc {
                    let ssid = ap.ssid.clone();
                    let strength = ap.strength;
                    let ap_clone = ap.clone();
                    let svc = svc.clone();
                    let hovered = create_signal(false);
                    let is_connected = ap.state == crate::services::network::DeviceState::Activated;
                    col = col.child(
                        container()
                            .width(fill())
                            .padding([6.0, 8.0])
                            .corner_radius(8.0)
                            .on_hover(move |h| hovered.set(h))
                            .on_click(move || {
                                svc.send(NetworkCmd::SelectAccessPoint((ap_clone.clone(), None)));
                            })
                            .background(move || {
                                if is_connected {
                                    Color::rgba(1.0, 1.0, 1.0, 0.15)
                                } else if hovered.get() {
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
                            .child(
                                icon(strength_to_icon(strength, true))
                                    .color(theme.text)
                                    .font_size(14.0),
                            )
                            .child(text(ssid).color(theme.text).font_size(12.0)),
                    );
                }
            }

            // Then other visible APs not in known list
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
                let hovered = create_signal(false);
                col = col.child(
                    container()
                        .width(fill())
                        .padding([6.0, 8.0])
                        .corner_radius(8.0)
                        .on_hover(move |h| hovered.set(h))
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
                        .child(
                            icon(strength_to_icon(strength, is_public))
                                .color(theme.text)
                                .font_size(14.0),
                        )
                        .child(text(ssid).color(theme.text).font_size(12.0)),
                );
            }
            Some(col)
        })
}

/// VPN submenu: list of known VPNs with toggle
pub fn vpn_submenu(
    data: NetworkDataSignals,
    svc: Service<NetworkCmd>,
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let known = data.known_connections;
    let active = data.active_connections;

    container()
        .width(fill())
        .layout(Flex::column().spacing(4.0))
        .child(text("VPN").color(theme.text).font_size(14.0))
        .child(move || {
            let known_list = known.with(|k| k.clone());
            let active_list = active.with(|a| a.clone());
            let mut col = container()
                .width(fill())
                .layout(Flex::column().spacing(2.0));

            for kc in &known_list {
                if let KnownConnection::Vpn(vpn) = kc {
                    let name = vpn.name.clone();
                    let vpn_clone = vpn.clone();
                    let svc = svc.clone();
                    let hovered = create_signal(false);
                    let active_vpn_path = active_list
                        .iter()
                        .find_map(|ac| match ac {
                            ActiveConnectionInfo::Vpn { name: n, object_path } if *n == name => {
                                Some(object_path.clone())
                            }
                            _ => None,
                        });
                    let is_active = active_vpn_path.is_some();
                    col = col.child(
                        container()
                            .width(fill())
                            .padding([6.0, 8.0])
                            .corner_radius(8.0)
                            .on_hover(move |h| hovered.set(h))
                            .on_click(move || {
                                svc.send(NetworkCmd::ToggleVpn(vpn_clone.clone(), active_vpn_path.clone()));
                            })
                            .background(move || {
                                if is_active {
                                    Color::rgba(1.0, 1.0, 1.0, 0.15)
                                } else if hovered.get() {
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
                            .child(icon(StaticIcon::Vpn).color(theme.text).font_size(14.0))
                            .child(text(name).color(theme.text).font_size(12.0)),
                    );
                }
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
