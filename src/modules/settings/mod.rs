use self::{
    audio::{audio_submenu, get_audio_sliders, sink_indicator, source_indicator, AudioMessage},
    battery::{battery_indicator, settings_battery_indicator},
    net::{active_connection_indicator, vpn_indicator, vpn_menu, wifi_menu, NetMessage},
    power::PowerMessage,
};
use crate::{
    app::OpenMenu,
    components::icons::{icon, Icons},
    menu::{close_menu, open_menu},
    modules::settings::{audio::SubmenuEntry, power::power_menu},
    style::{
        HeaderButtonStyle, QuickSettingsSubMenuButtonStyle, SettingsButtonStyle, MANTLE, PEACH,
        SURFACE_0, TEXT,
    },
    utils::{
        audio::{AudioCommand, Sink, Source},
        battery::{BatteryData, BatteryStatus},
        net::{ActiveConnection, NetCommand, Vpn, WifiConnection, WifiDeviceState},
        Commander,
    },
};
use iced::{
    advanced::Widget,
    theme::Button,
    widget::{
        button, column, container, horizontal_space, mouse_area, row, slider, text, Column, Row,
        Space,
    },
    window::Id,
    Alignment, Background, Element, Length, Subscription, Theme,
};

pub mod audio;
mod battery;
pub mod net;
mod power;

pub struct Settings {
    audio_commander: Commander<AudioCommand>,
    brightness_commander: Commander<f64>,
    net_commander: Commander<NetCommand>,
    sub_menu: Option<SubMenu>,
    battery_data: Option<BatteryData>,
    wifi_device_state: WifiDeviceState,
    scanning_nearby_wifi: bool,
    active_connection: Option<ActiveConnection>,
    vpn_active: bool,
    vpn_connections: Vec<Vpn>,
    nearby_wifi: Vec<WifiConnection>,
    default_sink: String,
    default_source: String,
    sinks: Vec<Sink>,
    sources: Vec<Source>,
    cur_sink_volume: i32,
    cur_source_volume: i32,
    cur_brightness: i32,
}

#[derive(Debug, Clone, Copy)]
pub enum BatteryMessage {
    PercentageChanged(i64),
    StatusChanged(BatteryStatus),
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleMenu,
    Battery(BatteryMessage),
    Net(NetMessage),
    Audio(AudioMessage),
    Lock,
    Power(PowerMessage),
    ToggleSubMenu(SubMenu),
    BrightnessChanged(f64),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SubMenu {
    Power,
    Sinks,
    Sources,
    Wifi,
    Vpn,
}

impl Settings {
    pub fn new() -> Self {
        Settings {
            audio_commander: Commander::new(),
            brightness_commander: Commander::new(),
            net_commander: Commander::new(),
            sub_menu: None,
            battery_data: None,
            wifi_device_state: WifiDeviceState::Unavailable,
            scanning_nearby_wifi: false,
            active_connection: None,
            vpn_active: false,
            vpn_connections: vec![],
            nearby_wifi: vec![],
            default_sink: String::new(),
            default_source: String::new(),
            sinks: vec![],
            sources: vec![],
            cur_sink_volume: 0,
            cur_source_volume: 0,
            cur_brightness: 0,
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        menu_id: Id,
        menu_type: &mut Option<OpenMenu>,
    ) -> iced::Command<Message> {
        match message {
            Message::ToggleMenu => match *menu_type {
                Some(OpenMenu::Settings) => {
                    menu_type.take();

                    close_menu(menu_id)
                }
                Some(_) => {
                    menu_type.replace(OpenMenu::Settings);
                    iced::Command::none()
                }
                None => {
                    menu_type.replace(OpenMenu::Settings);

                    open_menu(menu_id)
                }
            },
            Message::Battery(msg) => {
                match msg {
                    BatteryMessage::PercentageChanged(percentage) => {
                        if let Some(battery_data) = &mut self.battery_data {
                            battery_data.capacity = percentage;
                        } else {
                            self.battery_data = Some(BatteryData {
                                capacity: percentage,
                                status: BatteryStatus::Full,
                            });
                        }
                    }
                    BatteryMessage::StatusChanged(status) => {
                        if let Some(battery_data) = &mut self.battery_data {
                            battery_data.status = status;
                        } else {
                            self.battery_data = Some(BatteryData {
                                capacity: 100,
                                status,
                            });
                        }
                    }
                };
                iced::Command::none()
            }
            Message::Net(msg) => {
                msg.update(self);
                iced::Command::none()
            }
            Message::Audio(msg) => {
                msg.update(self);
                iced::Command::none()
            }
            Message::ToggleSubMenu(menu_type) => {
                if self.sub_menu == Some(menu_type) {
                    self.sub_menu.take();
                } else {
                    match menu_type {
                        SubMenu::Vpn => {
                            self.net_commander
                                .send(NetCommand::GetVpnConnections)
                                .unwrap();
                        }
                        SubMenu::Wifi => {
                            self.scanning_nearby_wifi = true;
                            self.net_commander.send(NetCommand::ScanNearByWifi).unwrap();
                        }
                        _ => {}
                    };
                    self.sub_menu.replace(menu_type);
                }
                iced::Command::none()
            }
            Message::Lock => {
                crate::utils::launcher::lock();
                iced::Command::none()
            }
            Message::Power(msg) => {
                msg.update();
                iced::Command::none()
            }
            Message::BrightnessChanged(value) => {
                self.cur_brightness = (value * 100.).round() as i32;
                self.brightness_commander.send(value).unwrap();
                iced::Command::none()
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let mut elements = row!().spacing(8);

        elements = elements.push(
            Row::with_children(
                vec![source_indicator(&self.sources), sink_indicator(&self.sinks)]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>(),
            )
            .spacing(4),
        );

        let mut net_elements = row!().spacing(4);
        if let Some(active_connection) = &self.active_connection {
            net_elements = net_elements.push(active_connection_indicator(active_connection));
        }

        if self.vpn_active {
            net_elements = net_elements.push(vpn_indicator());
        }

        elements = elements.push(net_elements);

        if let Some(battery_data) = self.battery_data {
            elements = elements.push(battery_indicator(battery_data));
        }

        button(elements)
            .style(Button::custom(HeaderButtonStyle::Right))
            .on_press(Message::ToggleMenu)
            .into()
    }

    pub fn menu_view(&self) -> Element<Message> {
        let battery_data = self.battery_data.map(settings_battery_indicator);
        let right_buttons = row!(
            button(icon(Icons::Lock))
                .padding([8, 10])
                .on_press(Message::Lock)
                .style(Button::custom(SettingsButtonStyle)),
            button(icon(if self.sub_menu == Some(SubMenu::Power) {
                Icons::Close
            } else {
                Icons::Power
            }))
            .padding([8, 10])
            .on_press(Message::ToggleSubMenu(SubMenu::Power))
            .style(Button::custom(SettingsButtonStyle))
        )
        .spacing(8);

        let header = if let Some(battery_data) = battery_data {
            row!(battery_data, Space::with_width(Length::Fill), right_buttons).width(Length::Fill)
        } else {
            row!(Space::with_width(Length::Fill), right_buttons)
        };

        let (sink_slider, source_slider) = get_audio_sliders(
            &self.sinks,
            self.cur_sink_volume,
            &self.sources,
            self.cur_source_volume,
            self.sub_menu,
        );

        let brightness_slider = row!(
            container(icon(Icons::Brightness))
                .padding([8, 10])
                .style(|_: &Theme| iced::widget::container::Appearance {
                    background: Background::Color(SURFACE_0).into(),
                    border_radius: 32.0.into(),
                    ..Default::default()
                }),
            slider(
                0..=100,
                self.cur_brightness,
                |v| Message::BrightnessChanged(v as f64 / 100.)
            )
            .step(1)
            .width(Length::Fill),
        )
        .align_items(Alignment::Center)
        .spacing(8);

        let wifi_setting_button = self.active_connection.as_ref().map_or_else(
            || {
                if self.wifi_device_state != WifiDeviceState::Unavailable {
                    Some((
                        quick_setting_button(
                            Icons::Wifi4,
                            "Wi-Fi".to_string(),
                            None,
                            self.wifi_device_state == WifiDeviceState::Active,
                            Message::Net(NetMessage::ToggleWifi),
                            Some((
                                SubMenu::Wifi,
                                self.sub_menu,
                                Message::ToggleSubMenu(SubMenu::Wifi),
                            )),
                        ),
                        self.sub_menu
                            .filter(|menu_type| *menu_type == SubMenu::Wifi)
                            .map(|_| {
                                sub_menu_wrapper(wifi_menu(
                                    self.scanning_nearby_wifi,
                                    None,
                                    &self.nearby_wifi,
                                ))
                                .map(Message::Net)
                            }),
                    ))
                } else {
                    None
                }
            },
            |a| match a {
                ActiveConnection::Wifi(wifi) => Some((
                    quick_setting_button(
                        a.get_icon(),
                        "Wi-Fi".to_string(),
                        Some(wifi.ssid.clone()),
                        true,
                        Message::Net(NetMessage::ToggleWifi),
                        Some((
                            SubMenu::Wifi,
                            self.sub_menu,
                            Message::ToggleSubMenu(SubMenu::Wifi),
                        )),
                    ),
                    self.sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Wifi)
                        .map(|_| {
                            sub_menu_wrapper(wifi_menu(
                                self.scanning_nearby_wifi,
                                Some(&wifi),
                                &self.nearby_wifi,
                            ))
                            .map(Message::Net)
                        }),
                )),
                _ => None,
            },
        );

        let quick_settings = quick_settings_section(
            vec![
                wifi_setting_button,
                Some((
                    quick_setting_button(
                        Icons::Vpn,
                        "Vpn".to_string(),
                        None,
                        self.vpn_active,
                        Message::Net(NetMessage::DeactivateVpns),
                        Some((
                            SubMenu::Vpn,
                            self.sub_menu,
                            Message::ToggleSubMenu(SubMenu::Vpn),
                        )),
                    ),
                    self.sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Vpn)
                        .map(|_| {
                            sub_menu_wrapper(vpn_menu(&self.vpn_connections)).map(Message::Net)
                        }),
                )),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
        );

        Column::with_children(
            vec![
                Some(header.into()),
                self.sub_menu
                    .filter(|menu_type| *menu_type == SubMenu::Power)
                    .map(|_| sub_menu_wrapper(power_menu().map(Message::Power))),
                sink_slider,
                self.sub_menu
                    .filter(|menu_type| *menu_type == SubMenu::Sinks)
                    .map(|_| {
                        sub_menu_wrapper(audio_submenu(
                            self.sinks
                                .iter()
                                .flat_map(|s| {
                                    s.ports.iter().map(|p| SubmenuEntry {
                                        name: format!("{}: {}", p.description, s.description),
                                        device: p.device_type,
                                        active: p.active && s.name == self.default_sink,
                                        msg: Message::Audio(AudioMessage::DefaultSinkChanged(
                                            s.name.clone(),
                                            p.name.clone(),
                                        )),
                                    })
                                })
                                .collect(),
                        ))
                    }),
                source_slider,
                self.sub_menu
                    .filter(|menu_type| *menu_type == SubMenu::Sources)
                    .map(|_| {
                        sub_menu_wrapper(audio_submenu(
                            self.sources
                                .iter()
                                .flat_map(|s| {
                                    s.ports.iter().map(|p| SubmenuEntry {
                                        name: format!("{}: {}", p.description, s.description),
                                        device: p.device_type,
                                        active: p.active && s.name == self.default_source,
                                        msg: Message::Audio(AudioMessage::DefaultSourceChanged(
                                            s.name.clone(),
                                            p.name.clone(),
                                        )),
                                    })
                                })
                                .collect(),
                        ))
                    }),
                Some(brightness_slider.into()),
                Some(quick_settings),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
        )
        .spacing(16)
        .padding(16)
        .max_width(350.)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        iced::Subscription::batch(vec![
            crate::utils::battery::subscription().map(Message::Battery),
            crate::utils::net::subscription(self.net_commander.give_receiver()).map(Message::Net),
            crate::utils::audio::subscription(self.audio_commander.give_receiver())
                .map(Message::Audio),
            crate::utils::brightness::subscription(self.brightness_commander.give_receiver()),
        ])
    }
}

fn quick_settings_section<'a>(
    buttons: Vec<(Element<'a, Message>, Option<Element<'a, Message>>)>,
) -> Element<'a, Message> {
    let mut section = column!().spacing(8);

    let mut before: Option<(Element<'a, Message>, Option<Element<'a, Message>>)> = None;

    for (button, menu) in buttons.into_iter() {
        if let Some((before_button, before_menu)) = before.take() {
            section = section.push(row![before_button, button].width(Length::Fill).spacing(8));

            if let Some(menu) = before_menu {
                section = section.push(sub_menu_wrapper(menu));
            }

            if let Some(menu) = menu {
                section = section.push(sub_menu_wrapper(menu));
            }
        } else {
            before = Some((button, menu));
        }
    }

    if let Some((before_button, before_menu)) = before.take() {
        section = section.push(
            row![before_button, horizontal_space(Length::Fill)]
                .width(Length::Fill)
                .spacing(8),
        );

        if let Some(menu) = before_menu {
            section = section.push(sub_menu_wrapper(menu));
        }
    }

    section.into()
}

fn sub_menu_wrapper<'a, Msg: 'static>(content: impl Into<Element<'a, Msg>>) -> Element<'a, Msg> {
    container(content.into())
        .style(|_: &Theme| iced::widget::container::Appearance {
            background: iced::Background::Color(MANTLE).into(),
            border_radius: 16.0.into(),
            ..iced::widget::container::Appearance::default()
        })
        .padding(8)
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
) -> Element<'a, Msg> {
    mouse_area(
        container(
            Row::with_children(
                vec![
                    Some(icon(icon_type).into()),
                    Some(
                        Column::with_children(
                            vec![
                                Some(text(title).into()),
                                subtitle.map(|s| text(s).size(12).into()),
                            ]
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>(),
                        )
                        .spacing(2)
                        .width(Length::Shrink)
                        .into(),
                    ),
                    with_submenu
                        .as_ref()
                        .map(|_| horizontal_space(Length::Fill).into()),
                    with_submenu.map(|(menu_type, submenu, msg)| {
                        button(icon(if Some(menu_type) == submenu {
                            Icons::Close
                        } else {
                            Icons::VerticalDots
                        }))
                        .on_press(msg)
                        .style(Button::custom(QuickSettingsSubMenuButtonStyle(active)))
                        .into()
                    }),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
            )
            .align_items(Alignment::Center)
            .spacing(8),
        )
        .padding([4, 8])
        .align_y(iced::alignment::Vertical::Center)
        .style(move |_: &Theme| iced::widget::container::Appearance {
            background: Some(Background::Color(if active { PEACH } else { SURFACE_0 })),
            text_color: Some(if active { SURFACE_0 } else { TEXT }),
            border_radius: 32.0.into(),
            ..Default::default()
        })
        .width(Length::Fill)
        .height(Length::Fixed(50.)),
    )
    .on_press(on_press)
    .into()
}
