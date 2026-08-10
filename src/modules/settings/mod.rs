pub mod audio;
pub mod bluetooth;
pub mod brightness;
pub mod network;
pub mod power;

use guido::prelude::*;

use crate::components::buttons::icon_button;
use crate::components::{StaticIcon, bar_indicator, quick_setting};
use crate::config::{Config, SettingsFormat, SettingsIndicator};
use crate::services;
use crate::services::bluetooth::BluetoothState;
use crate::services::network::ActiveConnectionInfo;
use crate::services::upower::PowerProfile;
use crate::theme::ThemeColors;

#[derive(Clone, Copy, PartialEq)]
pub enum SubMenu {
    Sinks,
    Sources,
    WiFi,
    Bluetooth,
    Vpn,
    Power,
    Peripherals,
}

/// WiFi connection dialog replacing the settings menu content.
#[derive(Clone, PartialEq)]
pub enum NetworkDialog {
    /// Secured network: ask for a password.
    Password { ssid: String },
    /// Open network: confirm connecting without encryption.
    OpenNetwork { ssid: String },
}

pub struct SettingsSignals {
    pub audio_data: services::compat::ServiceSignal<services::audio::AudioService>,
    pub audio_svc: Service<services::audio::AudioCommand>,
    pub brightness_data: services::compat::ServiceSignal<services::brightness::BrightnessService>,
    pub brightness_svc: Service<services::brightness::BrightnessCommand>,
    pub network_data: services::compat::ServiceSignal<services::network::NetworkService>,
    pub network_svc: Service<services::network::NetworkCommand>,
    pub bluetooth_data: services::compat::ServiceSignal<services::bluetooth::BluetoothService>,
    pub bluetooth_svc: Service<services::bluetooth::BluetoothCommand>,
    pub upower_data: services::compat::ServiceSignal<services::upower::UPowerService>,
    pub upower_svc: Service<services::upower::UPowerCommand>,
    pub idle_inhibitor_data: services::idle_inhibitor::IdleInhibitorDataSignals,
    pub idle_inhibitor_svc: Service<services::idle_inhibitor::IdleInhibitorCmd>,
    pub submenu: RwSignal<Option<SubMenu>>,
    pub network_dialog: RwSignal<Option<NetworkDialog>>,
    pub dialog_password: RwSignal<String>,
    pub dialog_show_password: RwSignal<bool>,
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
            network_dialog: self.network_dialog,
            dialog_password: self.dialog_password,
            dialog_show_password: self.dialog_show_password,
        }
    }
}

pub fn create() -> SettingsSignals {
    let network_dialog = create_signal(None::<NetworkDialog>);
    let dialog_password = create_signal(String::new());
    let dialog_show_password = create_signal(false);

    let (audio_data, audio_svc) = services::compat::run_service::<services::audio::AudioService>();
    let (brightness_data, brightness_svc) =
        services::compat::run_service::<services::brightness::BrightnessService>();
    // A NetworkManager agent can ask for a password mid-connection; the
    // event opens the dialog (upstream: Action::RequestPasswordForSSID)
    let dialog_w = network_dialog.writer();
    let (network_data, network_svc) =
        services::compat::run_service_hooked::<services::network::NetworkService>(move |ev| {
            if let services::compat::ServiceEvent::Update(
                services::network::NetworkEvent::RequestPasswordForSSID(ssid),
            ) = ev
            {
                dialog_w.set(Some(NetworkDialog::Password { ssid: ssid.clone() }));
            }
        });
    let (bluetooth_data, bluetooth_svc) =
        services::compat::run_service::<services::bluetooth::BluetoothService>();
    let (upower_data, upower_svc) =
        services::compat::run_service::<services::upower::UPowerService>();
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
        network_dialog,
        dialog_password,
        dialog_show_password,
    }
}

/// Bar view: indicators row driven by config.settings.indicators order & format
pub fn view(settings: SettingsSignals) -> impl Widget {
    let cfg = expect_context::<Config>();
    let theme = expect_context::<ThemeColors>();

    let mut row = container().layout(
        Flex::row()
            .spacing(8)
            .cross_alignment(CrossAlignment::Center),
    );

    for indicator in &cfg.settings.indicators {
        match indicator {
            SettingsIndicator::IdleInhibitor => {
                let inhibited = settings.idle_inhibitor_data.inhibited;
                row = row.child(move || {
                    if inhibited.get() {
                        Some(
                            bar_indicator()
                                .kind(StaticIcon::EyeOpened)
                                .color(theme.danger)
                                .format(SettingsFormat::Icon),
                        )
                    } else {
                        None
                    }
                });
            }
            SettingsIndicator::PowerProfile => {
                let upower = settings.upower_data;
                row = row.child(move || {
                    let profile = upower
                        .with(|s| s.as_ref().map(|x| x.power_profile))
                        .unwrap_or_default();
                    match profile {
                        PowerProfile::Performance => Some(
                            bar_indicator()
                                .kind(StaticIcon::Performance)
                                .color(theme.danger)
                                .format(SettingsFormat::Icon),
                        ),
                        PowerProfile::PowerSaver => Some(
                            bar_indicator()
                                .kind(StaticIcon::PowerSaver)
                                .color(theme.success)
                                .format(SettingsFormat::Icon),
                        ),
                        _ => None,
                    }
                });
            }
            SettingsIndicator::Audio => {
                row = row.child(audio::sink_indicator(
                    settings.audio_data,
                    cfg.settings.audio_indicator_format,
                ));
            }
            SettingsIndicator::Microphone => {
                row = row.child(audio::source_indicator(
                    settings.audio_data,
                    cfg.settings.microphone_indicator_format,
                ));
            }
            SettingsIndicator::Network => {
                row = row.child(network::wifi_indicator(
                    settings.network_data,
                    cfg.settings.network_indicator_format,
                ));
            }
            SettingsIndicator::Vpn => {
                let network = settings.network_data;
                row = row.child(move || {
                    let has_vpn = network.with(|s| {
                        s.as_ref().is_some_and(|x| {
                            x.active_connections
                                .iter()
                                .any(|ac| matches!(ac, ActiveConnectionInfo::Vpn { .. }))
                        })
                    });
                    if has_vpn {
                        Some(
                            bar_indicator()
                                .kind(StaticIcon::Vpn)
                                .color(theme.warning)
                                .format(SettingsFormat::Icon),
                        )
                    } else {
                        None
                    }
                });
            }
            SettingsIndicator::Bluetooth => {
                let bluetooth = settings.bluetooth_data;
                let format = cfg.settings.bluetooth_indicator_format;
                row = row.child(move || {
                    let (state, connected_count) = bluetooth.with(|s| {
                        s.as_ref()
                            .map(|x| {
                                (
                                    x.state.clone(),
                                    x.devices.iter().filter(|d| d.connected).count(),
                                )
                            })
                            .unwrap_or((BluetoothState::Unavailable, 0))
                    });
                    match state {
                        BluetoothState::Unavailable => None,
                        _ => {
                            let ic = if connected_count > 0 {
                                StaticIcon::BluetoothConnected
                            } else {
                                StaticIcon::Bluetooth
                            };
                            let label = if connected_count > 0 {
                                Some(format!("{connected_count}"))
                            } else {
                                None
                            };
                            Some(
                                bar_indicator()
                                    .kind(ic)
                                    .label(label)
                                    .color(theme.text)
                                    .format(format),
                            )
                        }
                    }
                });
            }
            SettingsIndicator::Battery => {
                let upower = settings.upower_data;
                let format = cfg.settings.battery_format;
                row = row.child(move || {
                    upower.with(|s| {
                        s.as_ref().and_then(|x| x.system_battery).map(|b| {
                            bar_indicator()
                                .kind(b.get_icon())
                                .label(power::battery_label(&b, format))
                                .color(power::battery_color(&b, &theme))
                                .format(format)
                        })
                    })
                });
            }
            SettingsIndicator::PeripheralBattery => {
                let upower = settings.upower_data;
                row = row.child(move || {
                    let periphs = upower.with(|s| {
                        s.as_ref()
                            .map(|x| x.peripherals.clone())
                            .unwrap_or_default()
                    });
                    if periphs.is_empty() {
                        return None;
                    }
                    let mut periph_row = container().layout(
                        Flex::row()
                            .spacing(8)
                            .cross_alignment(CrossAlignment::Center),
                    );
                    for p in &periphs {
                        let color = power::battery_color(&p.data, &theme);
                        periph_row = periph_row.child(
                            bar_indicator()
                                .kind(p.kind.get_icon())
                                .label(Some(format!("{}%", p.data.capacity)))
                                .color(color)
                                .format(SettingsFormat::IconAndPercentage),
                        );
                    }
                    Some(periph_row)
                });
            }
            SettingsIndicator::Brightness => {
                row = row.child(brightness::brightness_indicator(
                    settings.brightness_data,
                    cfg.settings.brightness_indicator_format,
                ));
            }
        }
    }

    row
}

/// Menu view: full settings panel content
pub fn menu_view(
    settings: SettingsSignals,
    close_menu: impl Fn() + 'static + Clone,
) -> impl Widget {
    // The WiFi password / open-network dialog takes over the whole menu
    let dialog_settings = settings.clone();
    container().width(fill()).child(move || {
        let settings = dialog_settings.clone();
        let close_menu = close_menu.clone();
        match settings.network_dialog.get() {
            Some(dialog) => Some(network::network_dialog_view(settings, dialog).into_any()),
            None => Some(menu_body(settings, close_menu).into_any()),
        }
    })
}

fn menu_body(settings: SettingsSignals, close_menu: impl Fn() + 'static + Clone) -> impl Widget {
    let submenu = settings.submenu;

    let settings2 = settings.clone();
    let settings3 = settings.clone();
    let close_menu2 = close_menu.clone();

    let lock_cmd = with_context::<Config, _>(|c| c.settings.lock_cmd.clone()).unwrap();

    container()
        .width(fill())
        .layout(Flex::column().spacing(12))
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
                .child(power::battery_header(settings.upower_data, submenu))
                .child({
                    container()
                        .layout(
                            Flex::row()
                                .spacing(4)
                                .cross_alignment(CrossAlignment::Center),
                        )
                        .maybe_child(lock_cmd.map(|cmd| {
                            icon_button().icon(StaticIcon::Lock).on_click(move || {
                                crate::utils::launcher::execute_command(&cmd);
                                close();
                            })
                        }))
                        .child(
                            icon_button()
                                .icon(move || -> crate::components::IconKind {
                                    if submenu.get() == Some(SubMenu::Power) {
                                        StaticIcon::Close.into()
                                    } else {
                                        StaticIcon::Power.into()
                                    }
                                })
                                .on_click(move || {
                                    submenu.set(if submenu.get() == Some(SubMenu::Power) {
                                        None
                                    } else {
                                        Some(SubMenu::Power)
                                    });
                                }),
                        )
                })
        })
        // Power submenu (conditionally shown)
        .child(submenu_wrapper(
            move || submenu.get() == Some(SubMenu::Power),
            power::power_actions(close_menu2.clone()),
        ))
        // Peripherals submenu (conditionally shown)
        .child(submenu_wrapper(
            move || submenu.get() == Some(SubMenu::Peripherals),
            power::peripherals_view(settings.upower_data),
        ))
        // Audio: sink slider (with chevron for device selection)
        .child(audio::sink_slider(
            settings.audio_data,
            settings.audio_svc.clone(),
            submenu,
        ))
        // Sinks submenu
        .child(submenu_wrapper(
            move || submenu.get() == Some(SubMenu::Sinks),
            audio::sinks_submenu(settings.audio_data, settings.audio_svc.clone()),
        ))
        // Audio: source slider (with chevron for device selection)
        .child(audio::source_slider(
            settings.audio_data,
            settings.audio_svc.clone(),
            submenu,
        ))
        // Sources submenu
        .child(submenu_wrapper(
            move || submenu.get() == Some(SubMenu::Sources),
            audio::sources_submenu(settings.audio_data, settings.audio_svc.clone()),
        ))
        // Brightness slider
        .child(brightness::slider_view(
            settings.brightness_data,
            settings.brightness_svc.clone(),
        ))
        // Quick Settings Grid (2 columns)
        // Row 1: WiFi | Bluetooth
        .child(move || {
            let settings = settings2.clone();
            Some(
                container()
                    .width(fill())
                    .layout(Flex::column().spacing(8))
                    .child(
                        container()
                            .width(fill())
                            .layout(Flex::row().spacing(8))
                            .child(network::wifi_quick_setting(
                                settings.network_data,
                                settings.network_svc.clone(),
                                move || {
                                    submenu.set(if submenu.get() == Some(SubMenu::WiFi) {
                                        None
                                    } else {
                                        Some(SubMenu::WiFi)
                                    });
                                },
                                move || submenu.get() == Some(SubMenu::WiFi),
                            ))
                            .child(bluetooth::bt_quick_setting(
                                settings.bluetooth_data,
                                settings.bluetooth_svc.clone(),
                                move || {
                                    submenu.set(if submenu.get() == Some(SubMenu::Bluetooth) {
                                        None
                                    } else {
                                        Some(SubMenu::Bluetooth)
                                    });
                                },
                                move || submenu.get() == Some(SubMenu::Bluetooth),
                            )),
                    ),
            )
        })
        // WiFi submenu
        .child(submenu_wrapper(
            move || submenu.get() == Some(SubMenu::WiFi),
            network::wifi_submenu(
                settings3.network_data,
                settings3.network_svc.clone(),
                settings3.clone(),
            ),
        ))
        // Bluetooth submenu
        .child(submenu_wrapper(
            move || submenu.get() == Some(SubMenu::Bluetooth),
            bluetooth::bt_submenu(settings3.bluetooth_data, settings3.bluetooth_svc.clone()),
        ))
        // Row 2: VPN | Airplane
        .child({
            let net_data = settings3.network_data;
            let net_svc = settings3.network_svc.clone();
            move || {
                Some(
                    container()
                        .width(fill())
                        .layout(Flex::row().spacing(8))
                        .child(network::vpn_quick_setting(
                            net_data,
                            net_svc.clone(),
                            move || {
                                submenu.set(if submenu.get() == Some(SubMenu::Vpn) {
                                    None
                                } else {
                                    Some(SubMenu::Vpn)
                                });
                            },
                            move || submenu.get() == Some(SubMenu::Vpn),
                        ))
                        .child(network::airplane_quick_setting(net_data, net_svc.clone())),
                )
            }
        })
        // VPN submenu
        .child(submenu_wrapper(
            move || submenu.get() == Some(SubMenu::Vpn),
            network::vpn_submenu(settings3.network_data, settings3.network_svc.clone()),
        ))
        // Row 3: Idle Inhibitor | Power Profile
        .child({
            let inhibitor_data = settings3.idle_inhibitor_data;
            let inhibitor_svc = settings3.idle_inhibitor_svc.clone();
            let up_data = settings3.upower_data;
            let up_svc = settings3.upower_svc.clone();
            move || {
                let inhibitor_svc = inhibitor_svc.clone();
                Some(
                    container()
                        .width(fill())
                        .layout(Flex::row().spacing(8))
                        .child(idle_inhibitor_quick_setting(inhibitor_data, inhibitor_svc))
                        .child(power::power_profile_quick_setting(up_data, up_svc.clone())),
                )
            }
        })
}

fn submenu_wrapper(
    visible: impl Fn() -> bool + 'static,
    content: impl Widget + 'static,
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();

    container()
        .width(fill())
        .height(move || {
            if visible() {
                Length::default()
            } else {
                Length::from(0)
            }
        })
        .overflow(Overflow::Hidden)
        .animate_height(
            Transition::spring(SpringConfig::SNAPPY)
                .reverse(Transition::new(200, TimingFunction::EaseOut)),
        )
        .child(
            container()
                .width(fill())
                .padding(12)
                .corner_radius(16)
                .background(theme.background.lighter(0.05))
                .border(1, theme.background.darker(0.2))
                .child(content),
        )
}

fn idle_inhibitor_quick_setting(
    data: services::idle_inhibitor::IdleInhibitorDataSignals,
    svc: Service<services::idle_inhibitor::IdleInhibitorCmd>,
) -> impl Widget {
    let inhibited = data.inhibited;
    let svc_toggle = svc.clone();

    quick_setting()
        .kind(move || {
            if inhibited.get() {
                StaticIcon::EyeOpened
            } else {
                StaticIcon::EyeClosed
            }
        })
        .title(move || "Idle Inhibitor".to_string())
        .subtitle(String::new)
        .active(move || inhibited.get())
        .on_toggle(move || svc_toggle.send(services::idle_inhibitor::IdleInhibitorCmd::Toggle))
}
