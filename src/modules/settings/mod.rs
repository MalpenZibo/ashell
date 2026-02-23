pub mod audio;
pub mod bluetooth;
pub mod brightness;
pub mod network;
pub mod power;

use guido::prelude::*;

use crate::components::{StaticIcon, icon, quick_setting};
use crate::services;
use crate::theme;

#[derive(Clone, Copy, PartialEq)]
pub enum SubMenu {
    Sinks,
    Sources,
    WiFi,
    Bluetooth,
    Vpn,
    Power,
}

pub struct SettingsSignals {
    pub audio_data: services::audio::AudioDataSignals,
    pub audio_svc: Service<services::audio::AudioCmd>,
    pub brightness_data: services::brightness::BrightnessDataSignals,
    pub brightness_svc: Service<services::brightness::BrightnessCmd>,
    pub network_data: services::network::NetworkDataSignals,
    pub network_svc: Service<services::network::NetworkCmd>,
    pub bluetooth_data: services::bluetooth::BluetoothDataSignals,
    pub bluetooth_svc: Service<services::bluetooth::BluetoothCmd>,
    pub upower_data: services::upower::UPowerDataSignals,
    pub upower_svc: Service<services::upower::UPowerCmd>,
    pub idle_inhibitor_data: services::idle_inhibitor::IdleInhibitorDataSignals,
    pub idle_inhibitor_svc: Service<services::idle_inhibitor::IdleInhibitorCmd>,
    pub submenu: Signal<Option<SubMenu>>,
}

impl Clone for SettingsSignals {
    fn clone(&self) -> Self {
        Self {
            audio_data: self.audio_data,
            audio_svc: self.audio_svc.clone(),
            brightness_data: self.brightness_data,
            brightness_svc: self.brightness_svc.clone(),
            network_data: self.network_data,
            network_svc: self.network_svc.clone(),
            bluetooth_data: self.bluetooth_data,
            bluetooth_svc: self.bluetooth_svc.clone(),
            upower_data: self.upower_data,
            upower_svc: self.upower_svc.clone(),
            idle_inhibitor_data: self.idle_inhibitor_data,
            idle_inhibitor_svc: self.idle_inhibitor_svc.clone(),
            submenu: self.submenu,
        }
    }
}

pub fn create() -> SettingsSignals {
    let (audio_data, audio_svc) = services::audio::create();
    let (brightness_data, brightness_svc) = services::brightness::create();
    let (network_data, network_svc) = services::network::create();
    let (bluetooth_data, bluetooth_svc) = services::bluetooth::create();
    let (upower_data, upower_svc) = services::upower::create();
    let (idle_inhibitor_data, idle_inhibitor_svc) = services::idle_inhibitor::create();
    let submenu = create_signal(None::<SubMenu>);

    SettingsSignals {
        audio_data,
        audio_svc,
        brightness_data,
        brightness_svc,
        network_data,
        network_svc,
        bluetooth_data,
        bluetooth_svc,
        upower_data,
        upower_svc,
        idle_inhibitor_data,
        idle_inhibitor_svc,
        submenu,
    }
}

/// Bar view: [Speaker% | Wifi | BT | Battery%]
pub fn view(settings: SettingsSignals) -> impl Widget {
    container()
        .layout(
            Flex::row()
                .spacing(10.0)
                .cross_alignment(CrossAlignment::Center),
        )
        .child(audio::sink_indicator(settings.audio_data))
        .child(network::wifi_indicator(settings.network_data))
        .child(bluetooth::bt_indicator(settings.bluetooth_data))
        .child(power::battery_indicator(settings.upower_data))
}

/// Menu view: full settings panel content
pub fn menu_view(
    settings: SettingsSignals,
    close_menu: impl Fn() + 'static + Clone,
) -> impl Widget {
    let submenu = settings.submenu;

    let settings2 = settings.clone();
    let settings3 = settings.clone();
    let close_menu2 = close_menu.clone();

    container()
        .width(fill())
        .layout(Flex::column().spacing(12.0))
        // Header: battery info + power buttons
        .child({
            let close = close_menu.clone();
            container()
                .width(fill())
                .layout(
                    Flex::row()
                        .main_alignment(MainAlignment::SpaceBetween)
                        .cross_alignment(CrossAlignment::Center),
                )
                .child(power::battery_header(settings.upower_data))
                .child({
                    container()
                        .layout(
                            Flex::row()
                                .spacing(4.0)
                                .cross_alignment(CrossAlignment::Center),
                        )
                        .child(header_icon_button(StaticIcon::Lock, move || {
                            let _ = std::process::Command::new("loginctl")
                                .arg("lock-session")
                                .spawn();
                            close();
                        }))
                        .child(header_icon_button(StaticIcon::Power, move || {
                            submenu.set(
                                if submenu.get() == Some(SubMenu::Power) {
                                    None
                                } else {
                                    Some(SubMenu::Power)
                                },
                            );
                        }))
                })
        })
        // Power submenu (conditionally shown)
        .child(move || {
            if submenu.get() == Some(SubMenu::Power) {
                Some(power::power_actions(close_menu2.clone()))
            } else {
                None
            }
        })
        // Audio: sink slider (with chevron for device selection)
        .child(audio::sink_slider(
            settings.audio_data,
            settings.audio_svc.clone(),
            submenu,
        ))
        // Sinks submenu
        .child({
            let audio_data = settings.audio_data;
            let audio_svc = settings.audio_svc.clone();
            move || {
                if submenu.get() == Some(SubMenu::Sinks) {
                    Some(audio::sinks_submenu(audio_data, audio_svc.clone()))
                } else {
                    None
                }
            }
        })
        // Audio: source slider (with chevron for device selection)
        .child(audio::source_slider(
            settings.audio_data,
            settings.audio_svc.clone(),
            submenu,
        ))
        // Sources submenu
        .child({
            let audio_data = settings.audio_data;
            let audio_svc = settings.audio_svc.clone();
            move || {
                if submenu.get() == Some(SubMenu::Sources) {
                    Some(audio::sources_submenu(audio_data, audio_svc.clone()))
                } else {
                    None
                }
            }
        })
        // Brightness slider
        .child(brightness::slider_view(
            settings.brightness_data,
            settings.brightness_svc.clone(),
        ))
        // Quick Settings Grid (2 columns)
        // Row 1: WiFi | Bluetooth
        .child(move || {
            let settings = settings2.clone();
            Some(container()
                .width(fill())
                .layout(Flex::column().spacing(8.0))
                .child(
                    container()
                        .width(fill())
                        .layout(Flex::row().spacing(8.0))
                        .child(network::wifi_quick_setting(
                            settings.network_data,
                            settings.network_svc.clone(),
                            move || {
                                submenu.set(
                                    if submenu.get() == Some(SubMenu::WiFi) {
                                        None
                                    } else {
                                        Some(SubMenu::WiFi)
                                    },
                                );
                            },
                        ))
                        .child(bluetooth::bt_quick_setting(
                            settings.bluetooth_data,
                            settings.bluetooth_svc.clone(),
                            move || {
                                submenu.set(
                                    if submenu.get() == Some(SubMenu::Bluetooth) {
                                        None
                                    } else {
                                        Some(SubMenu::Bluetooth)
                                    },
                                );
                            },
                        )),
                ))
        })
        // WiFi submenu
        .child({
            let net_data = settings3.network_data;
            let net_svc = settings3.network_svc.clone();
            move || {
                if submenu.get() == Some(SubMenu::WiFi) {
                    Some(network::wifi_submenu(net_data, net_svc.clone()))
                } else {
                    None
                }
            }
        })
        // Bluetooth submenu
        .child({
            let bt_data = settings3.bluetooth_data;
            let bt_svc = settings3.bluetooth_svc.clone();
            move || {
                if submenu.get() == Some(SubMenu::Bluetooth) {
                    Some(bluetooth::bt_submenu(bt_data, bt_svc.clone()))
                } else {
                    None
                }
            }
        })
        // Row 2: VPN | Airplane
        .child({
            let net_data = settings3.network_data;
            let net_svc = settings3.network_svc.clone();
            move || {
                Some(container()
                    .width(fill())
                    .layout(Flex::row().spacing(8.0))
                    .child(network::vpn_quick_setting(
                        net_data,
                        net_svc.clone(),
                        move || {
                            submenu.set(
                                if submenu.get() == Some(SubMenu::Vpn) {
                                    None
                                } else {
                                    Some(SubMenu::Vpn)
                                },
                            );
                        },
                    ))
                    .child(network::airplane_quick_setting(net_data, net_svc.clone())))
            }
        })
        // VPN submenu
        .child({
            let net_data = settings3.network_data;
            let net_svc = settings3.network_svc.clone();
            move || {
                if submenu.get() == Some(SubMenu::Vpn) {
                    Some(network::vpn_submenu(net_data, net_svc.clone()))
                } else {
                    None
                }
            }
        })
        // Row 3: Idle Inhibitor | Power Profile
        .child({
            let inhibitor_data = settings3.idle_inhibitor_data;
            let inhibitor_svc = settings3.idle_inhibitor_svc.clone();
            let up_data = settings3.upower_data;
            let up_svc = settings3.upower_svc.clone();
            move || {
                let inhibitor_svc = inhibitor_svc.clone();
                Some(container()
                    .width(fill())
                    .layout(Flex::row().spacing(8.0))
                    .child(idle_inhibitor_quick_setting(inhibitor_data, inhibitor_svc))
                    .child(power::power_profile_quick_setting(up_data, up_svc.clone())))
            }
        })
        // Peripherals
        .child(move || {
            let periphs = settings3.upower_data.peripherals.with(|p| p.clone());
            if periphs.is_empty() {
                None
            } else {
                Some(
                    container()
                        .width(fill())
                        .layout(Flex::column().spacing(4.0))
                        .child(divider())
                        .child(power::peripherals_view(settings3.upower_data)),
                )
            }
        })
}

fn divider() -> impl Widget {
    container()
        .width(fill())
        .height(1.0)
        .background(Color::rgba(1.0, 1.0, 1.0, 0.15))
}

fn header_icon_button(
    ic: StaticIcon,
    on_click: impl Fn() + 'static,
) -> impl Widget {
    let hovered = create_signal(false);
    container()
        .padding(6.0)
        .corner_radius(8.0)
        .on_hover(move |h| hovered.set(h))
        .on_click(move || on_click())
        .background(move || {
            if hovered.get() {
                Color::rgba(1.0, 1.0, 1.0, 0.15)
            } else {
                Color::rgba(1.0, 1.0, 1.0, 0.08)
            }
        })
        .child(icon(ic).color(theme::TEXT).font_size(16.0))
}

fn idle_inhibitor_quick_setting(
    data: services::idle_inhibitor::IdleInhibitorDataSignals,
    svc: Service<services::idle_inhibitor::IdleInhibitorCmd>,
) -> impl Widget {
    let inhibited = data.inhibited;
    let svc_toggle = svc.clone();

    quick_setting(
        move || {
            if inhibited.get() {
                StaticIcon::EyeOpened
            } else {
                StaticIcon::EyeClosed
            }
        },
        move || "Idle Inhibitor".to_string(),
        move || String::new(),
        move || inhibited.get(),
        move || svc_toggle.send(services::idle_inhibitor::IdleInhibitorCmd::Toggle),
        None::<fn()>,
    )
}
