use self::{
    audio::sink_indicator,
    battery::battery_indicator,
    net::{vpn_indicator, wifi_indicator},
};
use crate::{
    style::HeaderButtonStyle,
    utils::{
        audio::Sink,
        battery::{get_battery_capacity, BatteryData},
        net::Wifi,
    },
};
use iced::{
    theme::Button,
    widget::{button, row},
    Element, Subscription,
};
use std::time::Duration;

mod audio;
mod battery;
mod net;

pub struct Settings {
    battery_data: Option<BatteryData>,
    wifi: Option<Wifi>,
    vpn_active: bool,
    sinks: Vec<Sink>,
}

#[derive(Debug, Clone)]
pub enum NetMessage {
    Wifi(Option<Wifi>),
    VpnActive(bool),
}

#[derive(Debug, Clone)]
pub enum AudioMessage {
    SinkChanges(Vec<Sink>),
}

#[derive(Debug, Clone)]
pub enum Message {
    BatteryUpdate,
    NetUpdate(NetMessage),
    AudioUpdate(AudioMessage),
}

impl Settings {
    pub fn new() -> Self {
        Settings {
            battery_data: get_battery_capacity(),
            wifi: None,
            vpn_active: false,
            sinks: vec![],
        }
    }

    pub fn update(&mut self, message: Message) {
        println!("settings: {:?}", message);
        match message {
            Message::BatteryUpdate => {
                get_battery_capacity();
            }
            Message::NetUpdate(msg) => match msg {
                NetMessage::Wifi(wifi) => {
                    println!("wifi: {:?}", wifi);
                    self.wifi = wifi
                }
                NetMessage::VpnActive(active) => {
                    println!("vpn: {:?}", active);
                    self.vpn_active = active
                }
            },
            Message::AudioUpdate(msg) => match msg {
                AudioMessage::SinkChanges(sinks) => {
                    println!("sinks: {:?}", sinks);
                    self.sinks = sinks
                }
            },
        }
    }

    pub fn view(&self) -> Element<Message> {
        let mut elements = row!().spacing(8);

        let audio_elements = row!(sink_indicator(&self.sinks)).spacing(4);
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
            .on_press(Message::BatteryUpdate)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        iced::Subscription::batch(vec![
            iced::time::every(Duration::from_secs(60)).map(|_| Message::BatteryUpdate),
            crate::utils::net::subscription().map(Message::NetUpdate),
            crate::utils::audio::subscription().map(Message::AudioUpdate),
        ])
    }
}
