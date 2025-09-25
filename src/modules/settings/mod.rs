use self::{
    audio::AudioMessage, bluetooth::BluetoothMessage, network::NetworkMessage, power::PowerMessage,
};
use super::{Module, OnModulePress};
use crate::{
    app,
    components::icons::{Icons, icon},
    config::{Position, SettingsModuleConfig},
    menu::MenuType,
    modules::settings::power::power_menu,
    outputs::Outputs,
    password_dialog,
    position_button::ButtonUIRef,
    services::{
        ReadOnlyService, Service, ServiceEvent,
        audio::{AudioCommand, AudioService},
        bluetooth::{BluetoothCommand, BluetoothService, BluetoothState},
        brightness::{BrightnessCommand, BrightnessService},
        idle_inhibitor::IdleInhibitorManager,
        network::{NetworkCommand, NetworkEvent, NetworkService},
        upower::{PowerProfileCommand, UPowerService},
    },
    style::{
        quick_settings_button_style, quick_settings_submenu_button_style, settings_button_style,
    },
};
use brightness::BrightnessMessage;
use iced::{
    Alignment, Background, Border, Element, Length, Padding, Subscription, Task, Theme,
    alignment::{Horizontal, Vertical},
    widget::{Column, Row, Space, button, column, container, horizontal_space, row, text},
    window::Id,
};
use log::info;
use upower::UPowerMessage;

pub mod audio;
pub mod bluetooth;
pub mod brightness;
pub mod network;
mod power;
mod upower;

pub struct Settings {
    audio: Option<AudioService>,
    pub brightness: Option<BrightnessService>,
    network: Option<NetworkService>,
    bluetooth: Option<BluetoothService>,
    idle_inhibitor: Option<IdleInhibitorManager>,
    pub sub_menu: Option<SubMenu>,
    upower: Option<UPowerService>,
    pub password_dialog: Option<(String, String)>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            audio: None,
            brightness: None,
            network: None,
            bluetooth: None,
            idle_inhibitor: IdleInhibitorManager::new(),
            sub_menu: None,
            upower: None,
            password_dialog: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleMenu(Id, ButtonUIRef),
    UPower(UPowerMessage),
    Network(NetworkMessage),
    Bluetooth(BluetoothMessage),
    Audio(AudioMessage),
    Brightness(BrightnessMessage),
    ToggleInhibitIdle,
    Lock,
    Power(PowerMessage),
    ToggleSubMenu(SubMenu),
    PasswordDialog(password_dialog::Message),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SubMenu {
    Power,
    Sinks,
    Sources,
    Wifi,
    Vpn,
    Bluetooth,
}

impl Settings {
    pub fn update(
        &mut self,
        message: Message,
        config: &SettingsModuleConfig,
        outputs: &mut Outputs,
        main_config: &crate::config::Config,
    ) -> Task<crate::app::Message> {
        match message {
            Message::ToggleMenu(id, button_ui_ref) => {
                self.sub_menu = None;
                self.password_dialog = None;
                outputs.toggle_menu(id, MenuType::Settings, button_ui_ref, main_config)
            }
            Message::Audio(msg) => match msg {
                AudioMessage::Event(event) => match event {
                    ServiceEvent::Init(service) => {
                        self.audio = Some(service);
                        Task::none()
                    }
                    ServiceEvent::Update(data) => {
                        if let Some(audio) = self.audio.as_mut() {
                            audio.update(data);

                            if self.sub_menu == Some(SubMenu::Sinks) && audio.sinks.len() < 2 {
                                self.sub_menu = None;
                            }

                            if self.sub_menu == Some(SubMenu::Sources) && audio.sources.len() < 2 {
                                self.sub_menu = None;
                            }
                        }
                        Task::none()
                    }
                    ServiceEvent::Error(_) => Task::none(),
                },
                AudioMessage::ToggleSinkMute => {
                    if let Some(audio) = self.audio.as_mut() {
                        let _ = audio.command(AudioCommand::ToggleSinkMute);
                    }
                    Task::none()
                }
                AudioMessage::SinkVolumeChanged(value) => {
                    if let Some(audio) = self.audio.as_mut() {
                        let _ = audio.command(AudioCommand::SinkVolume(value));
                    }
                    Task::none()
                }
                AudioMessage::DefaultSinkChanged(name, port) => {
                    if let Some(audio) = self.audio.as_mut() {
                        let _ = audio.command(AudioCommand::DefaultSink(name, port));
                    }
                    Task::none()
                }
                AudioMessage::ToggleSourceMute => {
                    if let Some(audio) = self.audio.as_mut() {
                        let _ = audio.command(AudioCommand::ToggleSourceMute);
                    }
                    Task::none()
                }
                AudioMessage::SourceVolumeChanged(value) => {
                    if let Some(audio) = self.audio.as_mut() {
                        let _ = audio.command(AudioCommand::SourceVolume(value));
                    }
                    Task::none()
                }
                AudioMessage::DefaultSourceChanged(name, port) => {
                    if let Some(audio) = self.audio.as_mut() {
                        let _ = audio.command(AudioCommand::DefaultSource(name, port));
                    }
                    Task::none()
                }
                AudioMessage::SinksMore(id) => {
                    if let Some(cmd) = &config.audio_sinks_more_cmd {
                        crate::utils::launcher::execute_command(cmd.to_string());
                        outputs.close_menu(id, main_config)
                    } else {
                        Task::none()
                    }
                }
                AudioMessage::SourcesMore(id) => {
                    if let Some(cmd) = &config.audio_sources_more_cmd {
                        crate::utils::launcher::execute_command(cmd.to_string());
                        outputs.close_menu(id, main_config)
                    } else {
                        Task::none()
                    }
                }
            },
            Message::UPower(msg) => match msg {
                UPowerMessage::Event(event) => match event {
                    ServiceEvent::Init(service) => {
                        self.upower = Some(service);
                        Task::none()
                    }
                    ServiceEvent::Update(data) => {
                        if let Some(upower) = self.upower.as_mut() {
                            upower.update(data);
                        }
                        Task::none()
                    }
                    ServiceEvent::Error(_) => Task::none(),
                },
                UPowerMessage::TogglePowerProfile => match self.upower.as_mut() {
                    Some(upower) => upower.command(PowerProfileCommand::Toggle).map(|event| {
                        crate::app::Message::Settings(Message::UPower(UPowerMessage::Event(event)))
                    }),
                    _ => Task::none(),
                },
            },
            Message::Network(msg) => match msg {
                NetworkMessage::Event(event) => match event {
                    ServiceEvent::Init(service) => {
                        self.network = Some(service);
                        Task::none()
                    }
                    ServiceEvent::Update(NetworkEvent::RequestPasswordForSSID(ssid)) => {
                        self.password_dialog = Some((ssid, "".to_string()));
                        Task::none()
                    }
                    ServiceEvent::Update(data) => {
                        if let Some(network) = self.network.as_mut() {
                            network.update(data);
                        }
                        Task::none()
                    }
                    _ => Task::none(),
                },
                NetworkMessage::ToggleAirplaneMode => match self.network.as_mut() {
                    Some(network) => {
                        if self.sub_menu == Some(SubMenu::Wifi) {
                            self.sub_menu = None;
                        }

                        network
                            .command(NetworkCommand::ToggleAirplaneMode)
                            .map(|event| {
                                crate::app::Message::Settings(Message::Network(
                                    NetworkMessage::Event(event),
                                ))
                            })
                    }
                    _ => Task::none(),
                },
                NetworkMessage::ToggleWiFi => match self.network.as_mut() {
                    Some(network) => {
                        if self.sub_menu == Some(SubMenu::Wifi) {
                            self.sub_menu = None;
                        }
                        network.command(NetworkCommand::ToggleWiFi).map(|event| {
                            crate::app::Message::Settings(Message::Network(NetworkMessage::Event(
                                event,
                            )))
                        })
                    }
                    _ => Task::none(),
                },
                NetworkMessage::SelectAccessPoint(ac) => match self.network.as_mut() {
                    Some(network) => network
                        .command(NetworkCommand::SelectAccessPoint((ac, None)))
                        .map(|event| {
                            crate::app::Message::Settings(Message::Network(NetworkMessage::Event(
                                event,
                            )))
                        }),
                    _ => Task::none(),
                },
                NetworkMessage::RequestWiFiPassword(id, ssid) => {
                    info!("Requesting password for {ssid}");
                    self.password_dialog = Some((ssid, "".to_string()));
                    outputs.request_keyboard(id, main_config.menu_keyboard_focus)
                }
                NetworkMessage::ScanNearByWiFi => match self.network.as_mut() {
                    Some(network) => network
                        .command(NetworkCommand::ScanNearByWiFi)
                        .map(|event| {
                            crate::app::Message::Settings(Message::Network(NetworkMessage::Event(
                                event,
                            )))
                        }),
                    _ => Task::none(),
                },
                NetworkMessage::WiFiMore(id) => {
                    if let Some(cmd) = &config.wifi_more_cmd {
                        crate::utils::launcher::execute_command(cmd.to_string());
                        outputs.close_menu(id, main_config)
                    } else {
                        Task::none()
                    }
                }
                NetworkMessage::VpnMore(id) => {
                    if let Some(cmd) = &config.vpn_more_cmd {
                        crate::utils::launcher::execute_command(cmd.to_string());
                        outputs.close_menu(id, main_config)
                    } else {
                        Task::none()
                    }
                }
                NetworkMessage::ToggleVpn(vpn) => match self.network.as_mut() {
                    Some(network) => network
                        .command(NetworkCommand::ToggleVpn(vpn))
                        .map(|event| {
                            crate::app::Message::Settings(Message::Network(NetworkMessage::Event(
                                event,
                            )))
                        }),
                    _ => Task::none(),
                },
            },
            Message::Bluetooth(msg) => match msg {
                BluetoothMessage::Event(event) => match event {
                    ServiceEvent::Init(service) => {
                        self.bluetooth = Some(service);
                        Task::none()
                    }
                    ServiceEvent::Update(data) => {
                        if let Some(bluetooth) = self.bluetooth.as_mut() {
                            bluetooth.update(data);
                        }
                        Task::none()
                    }
                    _ => Task::none(),
                },
                BluetoothMessage::Toggle => match self.bluetooth.as_mut() {
                    Some(bluetooth) => {
                        if self.sub_menu == Some(SubMenu::Bluetooth) {
                            self.sub_menu = None;
                        }

                        bluetooth.command(BluetoothCommand::Toggle).map(|event| {
                            crate::app::Message::Settings(Message::Bluetooth(
                                BluetoothMessage::Event(event),
                            ))
                        })
                    }
                    _ => Task::none(),
                },
                BluetoothMessage::More(id) => {
                    if let Some(cmd) = &config.bluetooth_more_cmd {
                        crate::utils::launcher::execute_command(cmd.to_string());
                        outputs.close_menu(id, main_config)
                    } else {
                        Task::none()
                    }
                }
            },
            Message::Brightness(msg) => match msg {
                BrightnessMessage::Event(event) => match event {
                    ServiceEvent::Init(service) => {
                        self.brightness = Some(service);
                        Task::none()
                    }
                    ServiceEvent::Update(data) => {
                        if let Some(brightness) = self.brightness.as_mut() {
                            brightness.update(data);
                        }
                        Task::none()
                    }
                    _ => Task::none(),
                },
                BrightnessMessage::Change(value) => match self.brightness.as_mut() {
                    Some(brightness) => {
                        brightness
                            .command(BrightnessCommand::Set(value))
                            .map(|event| {
                                crate::app::Message::Settings(Message::Brightness(
                                    BrightnessMessage::Event(event),
                                ))
                            })
                    }
                    _ => Task::none(),
                },
            },
            Message::ToggleSubMenu(menu_type) => {
                if self.sub_menu == Some(menu_type) {
                    self.sub_menu.take();
                } else {
                    self.sub_menu.replace(menu_type);

                    if menu_type == SubMenu::Wifi {
                        if let Some(network) = self.network.as_mut() {
                            return network
                                .command(NetworkCommand::ScanNearByWiFi)
                                .map(|event| {
                                    crate::app::Message::Settings(Message::Network(
                                        NetworkMessage::Event(event),
                                    ))
                                });
                        }
                    }
                }

                Task::none()
            }
            Message::ToggleInhibitIdle => {
                if let Some(idle_inhibitor) = &mut self.idle_inhibitor {
                    idle_inhibitor.toggle();
                }
                Task::none()
            }
            Message::Lock => {
                if let Some(lock_cmd) = &config.lock_cmd {
                    crate::utils::launcher::execute_command(lock_cmd.to_string());
                }
                Task::none()
            }
            Message::Power(msg) => {
                msg.update();
                Task::none()
            }
            Message::PasswordDialog(msg) => match msg {
                password_dialog::Message::PasswordChanged(password) => {
                    if let Some((_, current_password)) = &mut self.password_dialog {
                        *current_password = password;
                    }

                    Task::none()
                }
                password_dialog::Message::DialogConfirmed(id) => {
                    if let Some((ssid, password)) = self.password_dialog.take() {
                        let network_command = match self.network.as_mut() {
                            Some(network) => {
                                let ap = network
                                    .wireless_access_points
                                    .iter()
                                    .find(|ap| ap.ssid == ssid)
                                    .cloned();
                                if let Some(ap) = ap {
                                    network
                                        .command(NetworkCommand::SelectAccessPoint((
                                            ap,
                                            Some(password),
                                        )))
                                        .map(|event| {
                                            crate::app::Message::Settings(Message::Network(
                                                NetworkMessage::Event(event),
                                            ))
                                        })
                                } else {
                                    Task::none()
                                }
                            }
                            _ => Task::none(),
                        };
                        Task::batch(vec![network_command, outputs.release_keyboard(id, main_config.menu_keyboard_focus)])
                    } else {
                        outputs.release_keyboard(id, main_config.menu_keyboard_focus)
                    }
                }
                password_dialog::Message::DialogCancelled(id) => {
                    self.password_dialog = None;

                    outputs.release_keyboard(id, main_config.menu_keyboard_focus)
                }
            },
        }
    }

    pub fn menu_view(
        &self,
        id: Id,
        config: &SettingsModuleConfig,
        opacity: f32,
        position: Position,
    ) -> Element<Message> {
        if let Some((ssid, current_password)) = &self.password_dialog {
            password_dialog::view(id, ssid, current_password, opacity).map(Message::PasswordDialog)
        } else {
            let battery_data = self
                .upower
                .as_ref()
                .and_then(|upower| upower.battery)
                .map(|battery| battery.settings_indicator());
            let right_buttons = Row::new()
                .push_maybe(config.lock_cmd.as_ref().map(|_| {
                    button(icon(Icons::Lock))
                        .padding([8, 13])
                        .on_press(Message::Lock)
                        .style(settings_button_style(opacity))
                }))
                .push(
                    button(icon(if self.sub_menu == Some(SubMenu::Power) {
                        Icons::Close
                    } else {
                        Icons::Power
                    }))
                    .padding([8, 13])
                    .on_press(Message::ToggleSubMenu(SubMenu::Power))
                    .style(settings_button_style(opacity)),
                )
                .spacing(8);

            let header = Row::new()
                .push_maybe(battery_data)
                .push(Space::with_width(Length::Fill))
                .push(right_buttons)
                .spacing(8)
                .width(Length::Fill);

            let (sink_slider, source_slider) = self
                .audio
                .as_ref()
                .map(|a| a.audio_sliders(self.sub_menu, opacity))
                .unwrap_or((None, None));

            let wifi_setting_button = self.network.as_ref().and_then(|n| {
                n.get_wifi_quick_setting_button(
                    id,
                    self.sub_menu,
                    config.wifi_more_cmd.is_some(),
                    opacity,
                )
            });
            let quick_settings = quick_settings_section(
                vec![
                    wifi_setting_button,
                    self.bluetooth
                        .as_ref()
                        .filter(|b| b.state != BluetoothState::Unavailable)
                        .and_then(|b| {
                            b.get_quick_setting_button(
                                id,
                                self.sub_menu,
                                config.bluetooth_more_cmd.is_some(),
                                opacity,
                            )
                        }),
                    self.network.as_ref().and_then(|n| {
                        n.get_vpn_quick_setting_button(
                            id,
                            self.sub_menu,
                            config.vpn_more_cmd.is_some(),
                            opacity,
                        )
                    }),
                    self.network.as_ref().and_then(|n| {
                        if config.remove_airplane_btn {
                            None
                        } else {
                            Some(n.get_airplane_mode_quick_setting_button(opacity))
                        }
                    }),
                    self.idle_inhibitor.as_ref().and_then(|i| {
                        if config.remove_idle_btn {
                            None
                        } else {
                            Some((
                                quick_setting_button(
                                    if i.is_inhibited() {
                                        Icons::EyeOpened
                                    } else {
                                        Icons::EyeClosed
                                    },
                                    "Idle Inhibitor".to_string(),
                                    None,
                                    i.is_inhibited(),
                                    Message::ToggleInhibitIdle,
                                    None,
                                    opacity,
                                ),
                                None,
                            ))
                        }
                    }),
                    self.upower
                        .as_ref()
                        .and_then(|u| u.power_profile.get_quick_setting_button(opacity)),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
                opacity,
            );

            let (top_sink_slider, bottom_sink_slider) = match position {
                Position::Top => (sink_slider, None),
                Position::Bottom => (None, sink_slider),
            };
            let (top_source_slider, bottom_source_slider) = match position {
                Position::Top => (source_slider, None),
                Position::Bottom => (None, source_slider),
            };

            Column::new()
                .push(header)
                .push_maybe(
                    self.sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Power)
                        .map(|_| {
                            sub_menu_wrapper(
                                power_menu(opacity, config).map(Message::Power),
                                opacity,
                            )
                        }),
                )
                .push_maybe(top_sink_slider)
                .push_maybe(
                    self.sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Sinks)
                        .and_then(|_| {
                            self.audio.as_ref().map(|a| {
                                sub_menu_wrapper(
                                    a.sinks_submenu(
                                        id,
                                        config.audio_sinks_more_cmd.is_some(),
                                        opacity,
                                    ),
                                    opacity,
                                )
                            })
                        }),
                )
                .push_maybe(bottom_sink_slider)
                .push_maybe(top_source_slider)
                .push_maybe(
                    self.sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Sources)
                        .and_then(|_| {
                            self.audio.as_ref().map(|a| {
                                sub_menu_wrapper(
                                    a.sources_submenu(
                                        id,
                                        config.audio_sources_more_cmd.is_some(),
                                        opacity,
                                    ),
                                    opacity,
                                )
                            })
                        }),
                )
                .push_maybe(bottom_source_slider)
                .push_maybe(self.brightness.as_ref().map(|b| b.brightness_slider()))
                .push(quick_settings)
                .spacing(16)
                .into()
        }
    }
}

impl Module for Settings {
    type ViewData<'a> = ();
    type SubscriptionData<'a> = ();

    fn view(
        &self,
        _: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        Some((
            Row::new()
                .push_maybe(
                    self.idle_inhibitor
                        .as_ref()
                        .filter(|i| i.is_inhibited())
                        .map(|_| {
                            container(icon(Icons::EyeOpened)).style(|theme: &Theme| {
                                container::Style {
                                    text_color: Some(theme.palette().danger),
                                    ..Default::default()
                                }
                            })
                        }),
                )
                .push_maybe(
                    self.upower
                        .as_ref()
                        .and_then(|p| p.power_profile.indicator()),
                )
                .push_maybe(self.audio.as_ref().and_then(|a| a.sink_indicator()))
                .push(
                    Row::new()
                        .push_maybe(
                            self.network
                                .as_ref()
                                .and_then(|n| n.get_connection_indicator()),
                        )
                        .push_maybe(self.network.as_ref().and_then(|n| n.get_vpn_indicator()))
                        .spacing(4),
                )
                .push_maybe(
                    self.upower
                        .as_ref()
                        .and_then(|upower| upower.battery)
                        .map(|battery| battery.indicator()),
                )
                .spacing(8)
                .into(),
            Some(OnModulePress::ToggleMenu(MenuType::Settings)),
        ))
    }

    fn subscription(&self, _: Self::SubscriptionData<'_>) -> Option<Subscription<app::Message>> {
        Some(
            Subscription::batch(vec![
                UPowerService::subscribe()
                    .map(|event| Message::UPower(UPowerMessage::Event(event))),
                AudioService::subscribe().map(|evenet| Message::Audio(AudioMessage::Event(evenet))),
                BrightnessService::subscribe()
                    .map(|event| Message::Brightness(BrightnessMessage::Event(event))),
                NetworkService::subscribe()
                    .map(|event| Message::Network(NetworkMessage::Event(event))),
                BluetoothService::subscribe()
                    .map(|event| Message::Bluetooth(BluetoothMessage::Event(event))),
            ])
            .map(app::Message::Settings),
        )
    }
}

fn quick_settings_section<'a>(
    buttons: Vec<(Element<'a, Message>, Option<Element<'a, Message>>)>,
    opacity: f32,
) -> Element<'a, Message> {
    let mut section = column!().spacing(8);

    let mut before: Option<(Element<'a, Message>, Option<Element<'a, Message>>)> = None;

    for (button, menu) in buttons.into_iter() {
        match before.take() {
            Some((before_button, before_menu)) => {
                section = section.push(row![before_button, button].width(Length::Fill).spacing(8));

                if let Some(menu) = before_menu {
                    section = section.push(sub_menu_wrapper(menu, opacity));
                }

                if let Some(menu) = menu {
                    section = section.push(sub_menu_wrapper(menu, opacity));
                }
            }
            _ => {
                before = Some((button, menu));
            }
        }
    }

    if let Some((before_button, before_menu)) = before.take() {
        section = section.push(
            row![before_button, horizontal_space()]
                .width(Length::Fill)
                .spacing(8),
        );

        if let Some(menu) = before_menu {
            section = section.push(sub_menu_wrapper(menu, opacity));
        }
    }

    section.into()
}

fn sub_menu_wrapper<Msg: 'static>(content: Element<Msg>, opacity: f32) -> Element<Msg> {
    container(content)
        .style(move |theme: &Theme| container::Style {
            background: Background::Color(
                theme
                    .extended_palette()
                    .secondary
                    .strong
                    .color
                    .scale_alpha(opacity),
            )
            .into(),
            border: Border::default().rounded(16),
            ..container::Style::default()
        })
        .padding(16)
        .width(Length::Fill)
        .into()
}

fn quick_setting_button<'a, Msg: Clone + 'static>(
    icon_type: Icons,
    title: String,
    subtitle: Option<String>,
    active: bool,
    on_press: Msg,
    with_submenu: Option<(SubMenu, Option<SubMenu>, Msg)>,
    opacity: f32,
) -> Element<'a, Msg> {
    let main_content = row!(
        icon(icon_type).size(20),
        Column::new()
            .push(text(title).size(12))
            .push_maybe(subtitle.map(|s| text(s).size(10)))
            .spacing(4)
    )
    .spacing(8)
    .padding(Padding::ZERO.left(4))
    .width(Length::Fill)
    .align_y(Alignment::Center);

    button(
        Row::new()
            .push(main_content)
            .push_maybe(with_submenu.map(|(menu_type, submenu, msg)| {
                button(
                    container(icon(if Some(menu_type) == submenu {
                        Icons::Close
                    } else {
                        Icons::RightChevron
                    }))
                    .align_y(Vertical::Center)
                    .align_x(Horizontal::Center),
                )
                .padding([4, if Some(menu_type) == submenu { 9 } else { 12 }])
                .style(quick_settings_submenu_button_style(active, opacity))
                .width(Length::Shrink)
                .height(Length::Shrink)
                .on_press(msg)
            }))
            .spacing(4)
            .align_y(Alignment::Center)
            .height(Length::Fill),
    )
    .padding([4, 8])
    .on_press(on_press)
    .height(Length::Fill)
    .width(Length::Fill)
    .style(quick_settings_button_style(active, opacity))
    .width(Length::Fill)
    .height(Length::Fixed(50.))
    .into()
}
