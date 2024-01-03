use self::{
    audio::{sink_indicator, source_indicator},
    battery::{battery_indicator, settings_battery_indicator},
    net::{vpn_indicator, wifi_indicator},
};
use crate::{
    app::MenuRequest,
    components::icons::{icon, Icons},
    menu::{MenuOutput, SettingsInputMessage},
    style::{HeaderButtonStyle, SettingsButtonStyle, CRUST, LAVENDER, RED, SURFACE_0, YELLOW},
    utils::{
        audio::{Sink, Source},
        battery::{BatteryData, BatteryStatus},
        net::Wifi,
    },
};
use iced::{
    theme::Button,
    widget::{button, column, container, row, text, Space},
    Element, Length, Subscription, Theme,
};
use tokio::sync::mpsc::UnboundedSender;

mod audio;
mod battery;
mod net;

pub struct Settings {
    pub battery_data: Option<BatteryData>,
    wifi: Option<Wifi>,
    vpn_active: bool,
    sinks: Vec<Sink>,
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
}

impl Settings {
    pub fn new() -> Self {
        Settings {
            battery_data: None,
            wifi: None,
            vpn_active: false,
            sinks: vec![],
            sources: vec![],
        }
    }

    pub fn update(&mut self, message: Message) -> Option<MenuRequest> {
        match message {
            Message::ToggleMenu => Some(MenuRequest::Settings),
            Message::Battery(msg) => match msg {
                BatteryMessage::PercentageChanged(percentage) => {
                    if let Some(battery_data) = &mut self.battery_data {
                        battery_data.capacity = percentage;
                    } else {
                        self.battery_data = Some(BatteryData {
                            capacity: percentage,
                            status: BatteryStatus::Full,
                        });
                    }

                    None
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

                    None
                }
            },
            Message::Net(msg) => match msg {
                NetMessage::Wifi(wifi) => {
                    println!("wifi: {:?}", wifi);
                    self.wifi = wifi;

                    None
                }
                NetMessage::VpnActive(active) => {
                    println!("vpn: {:?}", active);
                    self.vpn_active = active;

                    None
                }
            },
            Message::Audio(msg) => match msg {
                AudioMessage::SinkChanges(sinks) => {
                    println!("sinks: {:?}", sinks);
                    self.sinks = sinks;

                    None
                }
                AudioMessage::SourceChanges(sources) => {
                    println!("sources: {:?}", sources);
                    self.sources = sources;

                    None
                }
            },
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

    pub fn subscription(&self) -> Subscription<Message> {
        iced::Subscription::batch(vec![
            crate::utils::battery::subscription().map(Message::Battery),
            crate::utils::net::subscription().map(Message::Net),
            crate::utils::audio::subscription().map(Message::Audio),
        ])
    }
}

#[derive(Debug, Clone)]
pub enum SettingsMenuMessage {
    MainMessage(SettingsInputMessage),
    Lock,
    OpenPowerMenu,
}

pub enum SubMenu {
    Power,
}

pub struct SettingsMenu {
    output_tx: UnboundedSender<MenuOutput>,
    sub_menu: Option<SubMenu>,
    battery_data: Option<BatteryData>,
}

impl SettingsMenu {
    pub fn new(output_tx: UnboundedSender<MenuOutput>, battery_data: Option<BatteryData>) -> Self {
        Self {
            output_tx,
            sub_menu: None,
            battery_data,
        }
    }

    pub fn update(&mut self, message: SettingsMenuMessage) -> iced::Command<SettingsMenuMessage> {
        match message {
            SettingsMenuMessage::Lock => crate::utils::launcher::lock(),
            SettingsMenuMessage::OpenPowerMenu => {
                self.sub_menu.replace(SubMenu::Power);
            }
            SettingsMenuMessage::MainMessage(message) => match message {
                SettingsInputMessage::Battery(battery) => match battery {
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
                },
            },
        };

        iced::Command::none()
    }

    pub fn view(&self) -> Element<SettingsMenuMessage> {
        let battery_data = self.battery_data.map(settings_battery_indicator);
        let right_buttons = row!(
            button(icon(Icons::Lock))
                .padding([8, 9])
                .on_press(SettingsMenuMessage::Lock)
                .style(Button::custom(SettingsButtonStyle)),
            button(icon(Icons::Power))
                .padding([8, 9])
                .on_press(SettingsMenuMessage::OpenPowerMenu)
                .style(Button::custom(SettingsButtonStyle))
        );

        column!(if let Some(battery_data) = battery_data {
            row!(battery_data, Space::with_width(Length::Fill), right_buttons).width(Length::Fill)
        } else {
            row!(Space::with_width(Length::Fill), right_buttons)
        },)
        .max_width(350.)
        .into()
    }
}
