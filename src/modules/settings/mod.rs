use self::{
    audio::{sink_indicator, source_indicator},
    battery::{battery_indicator, settings_battery_indicator},
    net::{vpn_indicator, wifi_indicator},
};
use crate::{
    app::OpenMenu,
    components::icons::{icon, Icons},
    menu::{close_menu, open_menu},
    style::{GhostButtonStyle, HeaderButtonStyle, SettingsButtonStyle, CRUST, MANTLE},
    utils::{
        audio::{Sink, Source},
        battery::{BatteryData, BatteryStatus},
        net::Wifi,
    },
};
use iced::{
    theme::Button,
    widget::{button, column, container, horizontal_rule, mouse_area, row, slider, text, Space},
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
    OpenSubMenu(SubMenu),
    CloseSubMenu,
    None,
}

#[derive(Debug, Clone, Copy)]
pub enum SubMenu {
    Power,
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
                        println!("sinks: {:?}", sinks);
                        self.sinks = sinks;
                    }
                    AudioMessage::SourceChanges(sources) => {
                        println!("sources: {:?}", sources);
                        self.sources = sources;
                    }
                };
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
            Message::OpenSubMenu(menu_type) => {
                self.sub_menu.replace(menu_type);

                iced::Command::none()
            }
            Message::CloseSubMenu => {
                self.sub_menu.take();

                iced::Command::none()
            }
            Message::None => iced::Command::none(),
        }
    }

    pub fn view(&self) -> Element<Message> {
        let mut elements = row!().spacing(8);

        let sink = sink_indicator(&self.sinks);
        let audio_elements = if let Some(source) = source_indicator(&self.sources) {
            row!(source, sink)
        } else {
            row!(sink)
        }
        .spacing(4);
        elements = elements.push(audio_elements);

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
        let sub_menu_open = self.sub_menu.is_some();

        let battery_data = self
            .battery_data
            .map(|battery_data| settings_battery_indicator(battery_data, sub_menu_open));
        let right_buttons = row!(
            button(icon(Icons::Lock))
                .padding([8, 9])
                .on_press_maybe(if !sub_menu_open {
                    Some(Message::Lock)
                } else {
                    None
                })
                .style(Button::custom(SettingsButtonStyle)),
            button(icon(Icons::Power))
                .padding([8, 9])
                .on_press_maybe(if !sub_menu_open {
                    Some(Message::OpenSubMenu(SubMenu::Power))
                } else {
                    None
                })
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

        let sink_slider = active_sink
            .map(|s| {
                row!(
                    button(if s.is_mute {
                        icon(Icons::Speaker0)
                    } else {
                        icon(Icons::Speaker3)
                    })
                    .padding([8, 9])
                    .on_press(Message::None)
                    .style(Button::custom(SettingsButtonStyle)),
                    slider(0..=100, 3, |v| Message::None)
                        .step(1)
                        // .style(|_: &Theme| {
                        //     iced::widget::slider::Appearance {
                        //         rail: iced::widget::slider::Rail {
                        //             colors: RailBackground(iced::Color::TRANSPARENT, iced::Color::TRANSPARENT),
                        //             width: 2.,
                        //             border_radius: 16.0.into(),
                        //         },
                        //         handle: iced::widget::slider::Handle {
                        //             shape: iced::widget::slider::HandleShape::Circle { radius: 8. },
                        //             color: LAVENDER,
                        //             border_width: 0.,
                        //             border_color: iced::Color::TRANSPARENT,
                        //         },
                        //     }
                        // })
                        .width(Length::Fill),
                )
                .align_items(Alignment::Center)
                .spacing(8)
            })
            .unwrap_or(row!());

        match self.sub_menu {
            None => column!(header, sink_slider)
                .spacing(16)
                .padding(16)
                .max_width(350.)
                .into(),
            Some(SubMenu::Power) => {
                let power_menu = column!(
                    button(text("Suspend"))
                        .padding([8, 9])
                        .on_press(Message::Suspend)
                        .width(Length::Fill)
                        .style(Button::custom(GhostButtonStyle)),
                    button(text("Reboot"))
                        .padding([8, 9])
                        .on_press(Message::Reboot)
                        .width(Length::Fill)
                        .style(Button::custom(GhostButtonStyle)),
                    button(text("Shutdown"))
                        .padding([8, 9])
                        .on_press(Message::Shutdown)
                        .width(Length::Fill)
                        .style(Button::custom(GhostButtonStyle)),
                    horizontal_rule(1),
                    button(text("Logout"))
                        .padding([8, 9])
                        .on_press(Message::Logout)
                        .width(Length::Fill)
                        .style(Button::custom(GhostButtonStyle)),
                )
                .padding(8)
                .width(Length::Fill)
                .spacing(8);

                mouse_area(
                    container(
                        column!(
                            header,
                            container(mouse_area(power_menu).on_release(Message::None)).style(
                                |theme: &Theme| iced::widget::container::Appearance {
                                    background: Some(theme.palette().background.into()),
                                    border_radius: 16.0.into(),
                                    ..Default::default()
                                }
                            ),
                            sink_slider
                        )
                        .spacing(16),
                    )
                    .style(|_: &Theme| iced::widget::container::Appearance {
                        background: Some(iced::Background::Color(MANTLE)),
                        border_radius: 16.0.into(),
                        border_width: 1.,
                        border_color: CRUST,
                        ..Default::default()
                    })
                    .max_width(350.)
                    .padding(16),
                )
                .on_release(Message::CloseSubMenu)
                .into()
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        iced::Subscription::batch(vec![
            crate::utils::battery::subscription().map(Message::Battery),
            crate::utils::net::subscription().map(Message::Net),
            crate::utils::audio::subscription().map(Message::Audio),
        ])
    }
}
