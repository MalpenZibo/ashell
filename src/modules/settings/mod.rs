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
    style::{GhostButtonStyle, HeaderButtonStyle, SettingsButtonStyle, MANTLE, SURFACE_0},
    utils::{
        audio::{AudioCommand, Sink, Source, Volume},
        battery::{BatteryData, BatteryStatus},
        net::Wifi,
        Commander,
    },
};
use iced::{
    theme::Button,
    widget::{button, column, container, horizontal_rule, row, slider, text, Column, Row, Space},
    window::Id,
    Alignment, Background, Element, Length, Subscription, Theme,
};

mod audio;
mod battery;
mod net;

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
pub enum NetMessage {
    Wifi(Option<Wifi>),
    VpnActive(bool),
}

#[derive(Debug, Clone)]
pub enum AudioMessage {
    DefaultSinkSourceChanged(String, String),
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
    DefaultSinkChanged(String, String),
    SourceToggleMute,
    SourceVolumeChanged(i32),
    DefaultSourceChanged(String, String),
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
                match msg {
                    NetMessage::Wifi(wifi) => {
                        self.wifi = wifi;
                    }
                    NetMessage::VpnActive(active) => {
                        self.vpn_active = active;
                    }
                };
                iced::Command::none()
            }
            Message::Audio(msg) => {
                match msg {
                    AudioMessage::SinkChanges(sinks) => {
                        self.sinks = sinks;
                        self.cur_sink_volume = (self
                            .sinks
                            .iter()
                            .find_map(|sink| {
                                if sink
                                    .ports
                                    .iter()
                                    .any(|p| p.active && sink.name == self.default_sink)
                                {
                                    Some(if sink.is_mute {
                                        0.
                                    } else {
                                        sink.volume.get_volume()
                                    })
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_default()
                            * 100.) as i32;
                    }
                    AudioMessage::SourceChanges(sources) => {
                        self.sources = sources;
                        self.cur_source_volume = (self
                            .sources
                            .iter()
                            .find_map(|source| {
                                if source
                                    .ports
                                    .iter()
                                    .any(|p| p.active && source.name == self.default_source)
                                {
                                    Some(if source.is_mute {
                                        0.
                                    } else {
                                        source.volume.get_volume()
                                    })
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_default()
                            * 100.) as i32;
                    }
                    AudioMessage::DefaultSinkSourceChanged(sink, source) => {
                        self.default_sink = sink;
                        self.default_source = source;

                        self.cur_sink_volume = (self
                            .sinks
                            .iter()
                            .find_map(|sink| {
                                if sink
                                    .ports
                                    .iter()
                                    .any(|p| p.active && sink.name == self.default_sink)
                                {
                                    Some(if sink.is_mute {
                                        0.
                                    } else {
                                        sink.volume.get_volume()
                                    })
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_default()
                            * 100.) as i32;
                        self.cur_source_volume = (self
                            .sources
                            .iter()
                            .find_map(|source| {
                                if source
                                    .ports
                                    .iter()
                                    .any(|p| p.active && source.name == self.default_source)
                                {
                                    Some(if source.is_mute {
                                        0.
                                    } else {
                                        source.volume.get_volume()
                                    })
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_default()
                            * 100.) as i32;
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
            Message::SinkToggleMute => {
                if let Some(sink) = self
                    .sinks
                    .iter()
                    .find(|sink| sink.name == self.default_sink)
                {
                    let _ = self.audio_commander.send(AudioCommand::SinkMute(
                        self.default_sink.clone(),
                        !sink.is_mute,
                    ));
                }
                iced::Command::none()
            }
            Message::SinkVolumeChanged(volume) => {
                self.cur_sink_volume = volume;
                if let Some(sink) = self
                    .sinks
                    .iter_mut()
                    .find(|sink| sink.name == self.default_sink)
                {
                    if let Some(new_volume) =
                        sink.volume.scale_volume(self.cur_sink_volume as f64 / 100.)
                    {
                        let _ = self
                            .audio_commander
                            .send(AudioCommand::SinkVolume(sink.name.clone(), *new_volume));
                    }
                }
                iced::Command::none()
            }
            Message::DefaultSinkChanged(name, port) => {
                self.default_sink = name.clone();
                for sink in self.sinks.iter_mut() {
                    for cur_port in sink.ports.iter_mut() {
                        cur_port.active = sink.name == name && cur_port.name == port;
                    }
                }

                let _ = self
                    .audio_commander
                    .send(AudioCommand::DefaultSink(name, port));
                iced::Command::none()
            }
            Message::SourceToggleMute => {
                if let Some(source) = self
                    .sources
                    .iter()
                    .find(|source| source.name == self.default_source)
                {
                    let _ = self.audio_commander.send(AudioCommand::SourceMute(
                        self.default_source.clone(),
                        !source.is_mute,
                    ));
                }
                iced::Command::none()
            }
            Message::SourceVolumeChanged(volume) => {
                self.cur_source_volume = volume;
                if let Some(source) = self
                    .sources
                    .iter_mut()
                    .find(|source| source.name == self.default_source)
                {
                    if let Some(new_volume) = source
                        .volume
                        .scale_volume(self.cur_source_volume as f64 / 100.)
                    {
                        let _ = self
                            .audio_commander
                            .send(AudioCommand::SourceVolume(source.name.clone(), *new_volume));
                    }
                }
                iced::Command::none()
            }
            Message::DefaultSourceChanged(name, port) => {
                self.default_source = name.clone();
                for source in self.sources.iter_mut() {
                    for cur_port in source.ports.iter_mut() {
                        cur_port.active = source.name == name && cur_port.name == port;
                    }
                }

                let _ = self
                    .audio_commander
                    .send(AudioCommand::DefaultSource(name, port));
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

        let active_sink = self
            .sinks
            .iter()
            .find(|sink| sink.ports.iter().any(|p| p.active));

        let sink_slider = active_sink.map(|s| {
            audio_slider(
                SliderType::Sink,
                s.is_mute,
                Message::SinkToggleMute,
                self.cur_sink_volume,
                Message::SinkVolumeChanged,
                if self.sinks.iter().map(|s| s.ports.len()).sum::<usize>() > 1 {
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
                Message::SourceToggleMute,
                self.cur_source_volume,
                Message::SourceVolumeChanged,
                if self.sources.iter().map(|s| s.ports.len()).sum::<usize>() > 1 {
                    Some((self.sub_menu, Message::ToggleSubMenu(SubMenu::Sources)))
                } else {
                    None
                },
            )
        });

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
                                    msg: Message::DefaultSinkChanged(
                                        s.name.clone(),
                                        p.name.clone(),
                                    ),
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
                                    msg: Message::DefaultSourceChanged(
                                        s.name.clone(),
                                        p.name.clone(),
                                    ),
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
