use self::{
    audio::{audio_submenu, get_audio_sliders, sink_indicator, source_indicator, AudioMessage},
    battery::{battery_indicator, settings_battery_indicator},
    net::{vpn_indicator, wifi_indicator, NetMessage},
    power::PowerMessage,
};
use crate::{
    app::OpenMenu,
    components::icons::{icon, Icons},
    menu::{close_menu, open_menu},
    modules::settings::{audio::SubmenuEntry, power::power_menu},
    style::{HeaderButtonStyle, SettingsButtonStyle, MANTLE, SURFACE_0},
    utils::{
        audio::{AudioCommand, Sink, Source},
        battery::{BatteryData, BatteryStatus},
        net::Wifi,
        Commander,
    },
};
use iced::{
    theme::Button,
    widget::{button, container, row, slider, Column, Row, Space},
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
    sub_menu: Option<SubMenu>,
    battery_data: Option<BatteryData>,
    wifi: Option<Wifi>,
    vpn_active: bool,
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
}

impl Settings {
    pub fn new() -> Self {
        Settings {
            audio_commander: Commander::new(),
            brightness_commander: Commander::new(),
            sub_menu: None,
            battery_data: None,
            wifi: None,
            vpn_active: false,
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
        if let Some(wifi) = &self.wifi {
            net_elements = net_elements.push(wifi_indicator(wifi));
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

        let sub_menu_wrapper = |content| {
            container(content)
                .style(|_: &Theme| iced::widget::container::Appearance {
                    background: iced::Background::Color(MANTLE).into(),
                    border_radius: 16.0.into(),
                    ..iced::widget::container::Appearance::default()
                })
                .padding(8)
                .width(Length::Fill)
        };

        Column::with_children(
            match self.sub_menu {
                None => vec![
                    Some(header.into()),
                    sink_slider,
                    source_slider,
                    Some(brightness_slider.into()),
                ],
                Some(SubMenu::Power) => {
                    let power_menu = sub_menu_wrapper(power_menu().map(Message::Power));

                    vec![
                        Some(header.into()),
                        Some(power_menu.into()),
                        sink_slider,
                        source_slider,
                        Some(brightness_slider.into()),
                    ]
                }
                Some(SubMenu::Sinks) => {
                    let sink_menu = sub_menu_wrapper(audio_submenu(
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
                    ));
                    vec![
                        Some(header.into()),
                        sink_slider,
                        Some(sink_menu.into()),
                        source_slider,
                        Some(brightness_slider.into()),
                    ]
                }
                Some(SubMenu::Sources) => {
                    let source_menu = sub_menu_wrapper(audio_submenu(
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
                    ));
                    vec![
                        Some(header.into()),
                        sink_slider,
                        source_slider,
                        Some(source_menu.into()),
                        Some(brightness_slider.into()),
                    ]
                }
            }
            .into_iter()
            .flatten()
            .collect(),
        )
        .spacing(16)
        .padding(16)
        .max_width(350.)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        iced::Subscription::batch(vec![
            crate::utils::battery::subscription().map(Message::Battery),
            crate::utils::net::subscription().map(Message::Net),
            crate::utils::audio::subscription(self.audio_commander.give_receiver())
                .map(Message::Audio),
            crate::utils::brightness::subscription(self.brightness_commander.give_receiver()),
        ])
    }
}
