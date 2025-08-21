use self::{network::NetworkMessage, power::PowerMessage};
use crate::{
    components::icons::{Icons, icon},
    config::{Position, SettingsModuleConfig},
    menu::MenuType,
    modules::settings::{audio::AudioSettings, bluetooth::BluetoothSettings, power::power_menu},
    outputs::Outputs,
    password_dialog,
    position_button::ButtonUIRef,
    services::{
        ReadOnlyService, Service, ServiceEvent,
        brightness::{BrightnessCommand, BrightnessService},
        idle_inhibitor::IdleInhibitorManager,
        network::{NetworkCommand, NetworkEvent, NetworkService},
        upower::{PowerProfileCommand, UPowerService},
    },
    theme::AshellTheme,
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
    config: SettingsModuleConfig,
    audio: AudioSettings,
    pub brightness: Option<BrightnessService>,
    network: Option<NetworkService>,
    bluetooth: BluetoothSettings,
    idle_inhibitor: Option<IdleInhibitorManager>,
    pub sub_menu: Option<SubMenu>,
    upower: Option<UPowerService>,
    pub password_dialog: Option<(String, String)>,
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleMenu(Id, ButtonUIRef),
    UPower(UPowerMessage),
    Network(NetworkMessage),
    Bluetooth(bluetooth::Message),
    Audio(audio::Message),
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
    pub fn new(config: SettingsModuleConfig) -> Self {
        Settings {
            audio: AudioSettings::new(
                config.audio_sinks_more_cmd.clone(),
                config.audio_sources_more_cmd.clone(),
            ),
            brightness: None,
            network: None,
            bluetooth: BluetoothSettings::new(config.bluetooth_more_cmd.clone()),
            idle_inhibitor: IdleInhibitorManager::new(),
            sub_menu: None,
            upower: None,
            password_dialog: None,
            config,
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        config: &SettingsModuleConfig,
        outputs: &mut Outputs,
    ) -> Task<crate::app::Message> {
        match message {
            Message::ToggleMenu(id, button_ui_ref) => {
                self.sub_menu = None;
                self.password_dialog = None;
                outputs.toggle_menu(id, MenuType::Settings, button_ui_ref)
            }
            Message::Audio(msg) => match self.audio.update(msg) {
                audio::Action::None => Task::none(),
                audio::Action::ToggleSinksMenu => {
                    if self.sub_menu == Some(SubMenu::Sinks) {
                        self.sub_menu.take();
                    } else {
                        self.sub_menu.replace(SubMenu::Sinks);
                    }
                    Task::none()
                }
                audio::Action::ToggleSourcesMenu => {
                    if self.sub_menu == Some(SubMenu::Sources) {
                        self.sub_menu.take();
                    } else {
                        self.sub_menu.replace(SubMenu::Sources);
                    }
                    Task::none()
                }
                audio::Action::CloseSubMenu => {
                    if self.sub_menu == Some(SubMenu::Sinks)
                        || self.sub_menu == Some(SubMenu::Sources)
                    {
                        self.sub_menu.take();
                    }
                    Task::none()
                }
                audio::Action::CloseMenu(id) => outputs.close_menu(id),
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
                    outputs.request_keyboard(id)
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
                        outputs.close_menu(id)
                    } else {
                        Task::none()
                    }
                }
                NetworkMessage::VpnMore(id) => {
                    if let Some(cmd) = &config.vpn_more_cmd {
                        crate::utils::launcher::execute_command(cmd.to_string());
                        outputs.close_menu(id)
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
            Message::Bluetooth(msg) => match self.bluetooth.update(msg) {
                bluetooth::Action::None => Task::none(),
                bluetooth::Action::ToggleBluetoothMenu => {
                    if self.sub_menu == Some(SubMenu::Bluetooth) {
                        self.sub_menu.take();
                    } else {
                        self.sub_menu.replace(SubMenu::Bluetooth);
                    }
                    Task::none()
                }
                bluetooth::Action::CloseSubMenu(task) => {
                    if self.sub_menu == Some(SubMenu::Bluetooth) {
                        self.sub_menu.take();
                    }

                    task.map(|msg| crate::app::Message::Settings(Message::Bluetooth(msg)))
                }
                bluetooth::Action::CloseMenu(id) => outputs.close_menu(id),
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
                        Task::batch(vec![network_command, outputs.release_keyboard(id)])
                    } else {
                        outputs.release_keyboard(id)
                    }
                }
                password_dialog::Message::DialogCancelled(id) => {
                    self.password_dialog = None;

                    outputs.release_keyboard(id)
                }
            },
        }
    }

    pub fn menu_view<'a>(
        &'a self,
        id: Id,
        theme: &'a AshellTheme,
        position: Position,
    ) -> Element<'a, Message> {
        if let Some((ssid, current_password)) = &self.password_dialog {
            password_dialog::view(id, ssid, current_password, theme.opacity)
                .map(Message::PasswordDialog)
        } else {
            let battery_data = self
                .upower
                .as_ref()
                .and_then(|upower| upower.battery)
                .map(|battery| battery.settings_indicator());
            let right_buttons = Row::new()
                .push_maybe(self.config.lock_cmd.as_ref().map(|_| {
                    button(icon(Icons::Lock))
                        .padding([theme.space.xs, theme.space.sm + 1])
                        .on_press(Message::Lock)
                        .style(theme.settings_button_style())
                }))
                .push(
                    button(icon(if self.sub_menu == Some(SubMenu::Power) {
                        Icons::Close
                    } else {
                        Icons::Power
                    }))
                    .padding([theme.space.xs, theme.space.sm + 1])
                    .on_press(Message::ToggleSubMenu(SubMenu::Power))
                    .style(theme.settings_button_style()),
                )
                .spacing(theme.space.xs);

            let header = Row::new()
                .push_maybe(battery_data)
                .push(Space::with_width(Length::Fill))
                .push(right_buttons)
                .spacing(theme.space.xs)
                .width(Length::Fill);

            let (sink_slider, source_slider) = self.audio.sliders(theme, self.sub_menu);

            let wifi_setting_button = self.network.as_ref().and_then(|n| {
                n.get_wifi_quick_setting_button(
                    id,
                    self.sub_menu,
                    self.config.wifi_more_cmd.is_some(),
                    theme,
                )
            });
            let quick_settings = quick_settings_section(
                vec![
                    wifi_setting_button,
                    self.bluetooth
                        .quick_setting_button(id, theme, self.sub_menu)
                        .map(|(button, submenu)| {
                            (
                                button.map(Message::Bluetooth),
                                submenu.map(|e| e.map(Message::Bluetooth)),
                            )
                        }),
                    self.network.as_ref().and_then(|n| {
                        n.get_vpn_quick_setting_button(
                            id,
                            self.sub_menu,
                            self.config.vpn_more_cmd.is_some(),
                            theme,
                        )
                    }),
                    self.network.as_ref().and_then(|n| {
                        if self.config.remove_airplane_btn {
                            None
                        } else {
                            Some(n.get_airplane_mode_quick_setting_button(theme))
                        }
                    }),
                    self.idle_inhibitor.as_ref().map(|idle_inhibitor| {
                        (
                            quick_setting_button(
                                if idle_inhibitor.is_inhibited() {
                                    Icons::EyeOpened
                                } else {
                                    Icons::EyeClosed
                                },
                                "Idle Inhibitor".to_string(),
                                None,
                                idle_inhibitor.is_inhibited(),
                                Message::ToggleInhibitIdle,
                                None,
                                theme,
                            ),
                            None,
                        )
                    }),
                    self.upower
                        .as_ref()
                        .and_then(|u| u.power_profile.get_quick_setting_button(theme)),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
                theme,
            );

            let (top_sink_slider, bottom_sink_slider) = match position {
                Position::Top => (sink_slider.map(|e| e.map(Message::Audio)), None),
                Position::Bottom => (None, sink_slider.map(|e| e.map(Message::Audio))),
            };
            let (top_source_slider, bottom_source_slider) = match position {
                Position::Top => (source_slider.map(|e| e.map(Message::Audio)), None),
                Position::Bottom => (None, source_slider.map(|e| e.map(Message::Audio))),
            };

            Column::new()
                .push(header)
                .push_maybe(
                    self.sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Power)
                        .map(|_| {
                            sub_menu_wrapper(
                                power_menu(theme, &self.config).map(Message::Power),
                                theme,
                            )
                        }),
                )
                .push_maybe(top_sink_slider)
                .push_maybe(
                    self.sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Sinks)
                        .and_then(|_| {
                            self.audio
                                .sinks_submenu(id, theme)
                                .map(|submenu| sub_menu_wrapper(submenu.map(Message::Audio), theme))
                        }),
                )
                .push_maybe(bottom_sink_slider)
                .push_maybe(top_source_slider)
                .push_maybe(
                    self.sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Sources)
                        .and_then(|_| {
                            self.audio
                                .sources_submenu(id, theme)
                                .map(|submenu| sub_menu_wrapper(submenu.map(Message::Audio), theme))
                        }),
                )
                .push_maybe(bottom_source_slider)
                .push_maybe(self.brightness.as_ref().map(|b| b.brightness_slider()))
                .push(quick_settings)
                .spacing(theme.space.md)
                .into()
        }
    }

    pub fn view(&self, theme: &AshellTheme) -> Element<Message> {
        Row::new()
            .push_maybe(
                self.idle_inhibitor
                    .as_ref()
                    .filter(|i| i.is_inhibited())
                    .map(|_| {
                        container(icon(Icons::EyeOpened)).style(|theme: &Theme| container::Style {
                            text_color: Some(theme.palette().danger),
                            ..Default::default()
                        })
                    }),
            )
            .push_maybe(
                self.upower
                    .as_ref()
                    .and_then(|p| p.power_profile.indicator()),
            )
            .push_maybe(self.audio.sink_indicator().map(|e| e.map(Message::Audio)))
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
            .spacing(theme.space.xs)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            UPowerService::subscribe().map(|event| Message::UPower(UPowerMessage::Event(event))),
            self.audio.subscription().map(Message::Audio),
            BrightnessService::subscribe()
                .map(|event| Message::Brightness(BrightnessMessage::Event(event))),
            NetworkService::subscribe().map(|event| Message::Network(NetworkMessage::Event(event))),
            self.bluetooth.subscription().map(Message::Bluetooth),
        ])
    }
}

fn quick_settings_section<'a>(
    buttons: Vec<(Element<'a, Message>, Option<Element<'a, Message>>)>,
    theme: &'a AshellTheme,
) -> Element<'a, Message> {
    let mut section = column!().spacing(theme.space.xs);

    let mut before: Option<(Element<'a, Message>, Option<Element<'a, Message>>)> = None;

    for (button, menu) in buttons.into_iter() {
        match before.take() {
            Some((before_button, before_menu)) => {
                section = section.push(
                    row![before_button, button]
                        .width(Length::Fill)
                        .spacing(theme.space.xs),
                );

                if let Some(menu) = before_menu {
                    section = section.push(sub_menu_wrapper(menu, theme));
                }

                if let Some(menu) = menu {
                    section = section.push(sub_menu_wrapper(menu, theme));
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
                .spacing(theme.space.xs),
        );

        if let Some(menu) = before_menu {
            section = section.push(sub_menu_wrapper(menu, theme));
        }
    }

    section.into()
}

fn sub_menu_wrapper<'a, Msg: 'static>(
    content: Element<'a, Msg>,
    ashell_theme: &'a AshellTheme,
) -> Element<'a, Msg> {
    container(content)
        .style(move |theme: &Theme| container::Style {
            background: Background::Color(
                theme
                    .extended_palette()
                    .secondary
                    .strong
                    .color
                    .scale_alpha(ashell_theme.opacity),
            )
            .into(),
            border: Border::default().rounded(ashell_theme.radius.lg),
            ..container::Style::default()
        })
        .padding(ashell_theme.space.md)
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
    theme: &'a AshellTheme,
) -> Element<'a, Msg> {
    let main_content = row!(
        icon(icon_type).size(theme.font_size.lg),
        Column::new()
            .push(text(title).size(theme.font_size.sm))
            .push_maybe(subtitle.map(|s| text(s).size(theme.font_size.xs)))
            .spacing(theme.space.xxs)
    )
    .spacing(theme.space.xs)
    .padding(Padding::ZERO.left(theme.space.xxs))
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
                .padding([
                    theme.space.xxs,
                    if Some(menu_type) == submenu {
                        theme.space.xs + 1
                    } else {
                        theme.space.sm
                    },
                ])
                .style(theme.quick_settings_submenu_button_style(active))
                .width(Length::Shrink)
                .height(Length::Shrink)
                .on_press(msg)
            }))
            .spacing(theme.space.xxs)
            .align_y(Alignment::Center)
            .height(Length::Fill),
    )
    .padding([theme.space.xxs, theme.space.xs])
    .on_press(on_press)
    .height(Length::Fill)
    .width(Length::Fill)
    .style(theme.quick_settings_button_style(active))
    .width(Length::Fill)
    .height(Length::Fixed(50.))
    .into()
}
