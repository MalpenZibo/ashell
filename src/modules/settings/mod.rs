use self::{
    audio::{audio_slider, audio_submenu, sink_indicator, source_indicator, SliderType},
    battery::{battery_indicator, settings_battery_indicator},
    net::{vpn_indicator, wifi_indicator},
};
use crate::{
    app::OpenMenu,
    components::icons::{icon, Icons},
    menu::{close_menu, open_menu},
    modules::settings::audio::SubmenuEntry,
    style::{GhostButtonStyle, HeaderButtonStyle, SettingsButtonStyle, CRUST, LAVENDER, MANTLE},
    utils::{
        audio::{Sink, Source},
        battery::{BatteryData, BatteryStatus},
        net::Wifi,
    },
};
use iced::{
    theme::Button,
    widget::{
        button, column, container, horizontal_rule, mouse_area, row, slider, text, Column, Row,
        Space,
    },
    window::Id,
    Alignment, Element, Length, Subscription, Theme,
};

mod audio;
mod battery;
mod net;

pub struct Settings {
    sub_menu: Option<SubMenu>,
    pub battery_data: Option<BatteryData>,
    wifi: Option<Wifi>,
    vpn_active: bool,
    pub sinks: Vec<Sink>,
    sources: Vec<Source>,
    cur_sink_volume: i32,
    cur_source_volume: i32,
}

#[derive(Debug, Clone, Copy)]
pub enum BatteryMessage {
    PercentageChanged(i64),
    StatusChanged(BatteryStatus),
}

#[derive(Debug, Clone)]
pub enum NetMessage {
    Wifi(Option<Wifi>),
    VpnActive(bool),
}

#[derive(Debug, Clone)]
pub enum AudioMessage {
    SinkChanges(Vec<Sink>),
    SourceChanges(Vec<Source>),
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleMenu,
    Battery(BatteryMessage),
    Net(NetMessage),
    Audio(AudioMessage),
    Lock,
    Suspend,
    Reboot,
    Shutdown,
    Logout,
    ToggleSubMenu(SubMenu),
    SinkToggleMute,
    SinkVolumeChanged(i32),
    SinkVolumeChangedEnd,
    SourceToggleMute,
    SourceVolumeChanged(i32),
    SourceVolumeChangedEnd,
    None,
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
            sub_menu: None,
            battery_data: None,
            wifi: None,
            vpn_active: false,
            sinks: vec![],
            sources: vec![],
            cur_sink_volume: 0,
            cur_source_volume: 0,
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
                        println!("battery: {:?}", status);
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
                match msg {
                    NetMessage::Wifi(wifi) => {
                        println!("wifi: {:?}", wifi);
                        self.wifi = wifi;
                    }
                    NetMessage::VpnActive(active) => {
                        println!("vpn: {:?}", active);
                        self.vpn_active = active;
                    }
                };
                iced::Command::none()
            }
            Message::Audio(msg) => {
                match msg {
                    AudioMessage::SinkChanges(sinks) => {
                        self.sinks = sinks;
                        self.cur_sink_volume = self
                            .sinks
                            .iter()
                            .find_map(|sink| {
                                if sink.ports.iter().any(|p| p.active) {
                                    Some(if sink.is_mute {
                                        0
                                    } else {
                                        (sink.volume * 100.) as i32
                                    })
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_default();
                    }
                    AudioMessage::SourceChanges(sources) => {
                        self.sources = sources;
                        self.cur_source_volume = self
                            .sources
                            .iter()
                            .find_map(|source| {
                                if source.ports.iter().any(|p| p.active) {
                                    Some(if source.is_mute {
                                        0
                                    } else {
                                        (source.volume * 100.) as i32
                                    })
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_default();
                    }
                };
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
            Message::Suspend => {
                crate::utils::launcher::suspend();
                iced::Command::none()
            }
            Message::Reboot => {
                crate::utils::launcher::reboot();
                iced::Command::none()
            }
            Message::Shutdown => {
                crate::utils::launcher::shutdown();
                iced::Command::none()
            }
            Message::Logout => {
                crate::utils::launcher::logout();
                iced::Command::none()
            }
            Message::SinkToggleMute => iced::Command::none(),
            Message::SinkVolumeChanged(volume) => {
                self.cur_sink_volume = volume;
                iced::Command::none()
            }
            Message::SinkVolumeChangedEnd => iced::Command::none(),
            Message::SourceToggleMute => iced::Command::none(),
            Message::SourceVolumeChanged(volume) => {
                self.cur_source_volume = volume;
                iced::Command::none()
            }
            Message::SourceVolumeChangedEnd => iced::Command::none(),
            Message::None => iced::Command::none(),
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

        let active_sink = self
            .sinks
            .iter()
            .find(|sink| sink.ports.iter().any(|p| p.active));

        let sink_slider = active_sink.map(|s| {
            audio_slider(
                SliderType::Sink,
                s.is_mute,
                Message::None,
                self.cur_sink_volume,
                Message::SinkVolumeChanged,
                Message::SinkVolumeChangedEnd,
                if self.sinks.len() > 1 {
                    Some((self.sub_menu, Message::ToggleSubMenu(SubMenu::Sinks)))
                } else {
                    None
                },
            )
        });

        let active_source = self
            .sources
            .iter()
            .find(|source| source.ports.iter().any(|p| p.active));

        let source_slider = active_source.map(|s| {
            audio_slider(
                SliderType::Source,
                s.is_mute,
                Message::None,
                self.cur_source_volume,
                Message::SourceVolumeChanged,
                Message::SourceVolumeChangedEnd,
                if self.sources.len() > 1 {
                    Some((self.sub_menu, Message::ToggleSubMenu(SubMenu::Sources)))
                } else {
                    None
                },
            )
        });

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
                None => vec![Some(header.into()), sink_slider, source_slider],
                Some(SubMenu::Power) => {
                    let power_menu = sub_menu_wrapper(
                        column!(
                            button(text("Suspend"))
                                .padding([4, 12])
                                .on_press(Message::Suspend)
                                .width(Length::Fill)
                                .style(Button::custom(GhostButtonStyle)),
                            button(text("Reboot"))
                                .padding([4, 12])
                                .on_press(Message::Reboot)
                                .width(Length::Fill)
                                .style(Button::custom(GhostButtonStyle)),
                            button(text("Shutdown"))
                                .padding([4, 12])
                                .on_press(Message::Shutdown)
                                .width(Length::Fill)
                                .style(Button::custom(GhostButtonStyle)),
                            horizontal_rule(1),
                            button(text("Logout"))
                                .padding([4, 12])
                                .on_press(Message::Logout)
                                .width(Length::Fill)
                                .style(Button::custom(GhostButtonStyle)),
                        )
                        .padding(8)
                        .width(Length::Fill)
                        .spacing(8)
                        .into(),
                    );

                    vec![
                        Some(header.into()),
                        Some(power_menu.into()),
                        sink_slider,
                        source_slider,
                    ]
                }
                Some(SubMenu::Sinks) => {
                    let sink_menu = sub_menu_wrapper(audio_submenu(
                        self.sinks
                            .iter()
                            .flat_map(|s| {
                                s.ports.iter().map(|p| SubmenuEntry {
                                    name: format!("{}: {}", p.description, s.description),
                                    active: p.active,
                                    msg: Message::None,
                                })
                            })
                            .collect(),
                    ));
                    vec![
                        Some(header.into()),
                        sink_slider,
                        Some(sink_menu.into()),
                        source_slider,
                    ]
                }
                Some(SubMenu::Sources) => {
                    let source_menu = sub_menu_wrapper(audio_submenu(
                        self.sources
                            .iter()
                            .flat_map(|s| {
                                s.ports.iter().map(|p| SubmenuEntry {
                                    name: format!("{}: {}", p.description, s.description),
                                    active: p.active,
                                    msg: Message::None,
                                })
                            })
                            .collect(),
                    ));
                    vec![
                        Some(header.into()),
                        sink_slider,
                        source_slider,
                        Some(source_menu.into()),
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
            crate::utils::audio::subscription().map(Message::Audio),
        ])
    }
}
