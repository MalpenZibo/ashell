use self::{battery::battery_indicator, net::{wifi_indicator, vpn_indicator}};
use crate::{
    style::HeaderButtonStyle,
    utils::{
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

mod battery;
mod net;

pub struct Settings {
    battery_data: Option<BatteryData>,
    wifi: Option<Wifi>,
    vpn_active: bool,
}

#[derive(Debug, Clone)]
pub enum NetMessage {
    Wifi(Option<Wifi>),
    VpnActive(bool),
}

#[derive(Debug, Clone)]
pub enum Message {
    BatteryUpdate,
    NetUpdate(NetMessage),
}

impl Settings {
    pub fn new() -> Self {
        Settings {
            battery_data: get_battery_capacity(),
            wifi: None,
            vpn_active: false,
        }
    }

    pub fn update(&mut self, message: Message) {
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
        }
    }

    pub fn view(&self) -> Element<Message> {
        let mut elements = row!().spacing(8);

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
            // iced::subscription::channel("wifi-signal-monitor", 10, |mut output| async move {
            //     let get_net = || async move { (get_active_connection().await, get_vpn().await) };
            //
            //     let _ = output.send(Message::NetUpdateActive(get_net().await)).await;
            //
            //     loop {
            //         let _ = output.send(Message::NetUpdateActive(get_net().await)).await;
            //         sleep(Duration::from_secs(60)).await
            //     }
            // }),
            // iced::subscription::channel("nmcli-monitor", 10, |mut output| async move {
            //     let _ = output
            //         .send(Message::NetUpdateActive((
            //             get_active_connection().await,
            //             get_vpn().await,
            //         )))
            //         .await;
            //
            //     let handle = Command::new("nmcli")
            //         .arg("monitor")
            //         .stdout(Stdio::piped())
            //         .stdin(Stdio::null())
            //         .spawn()
            //         .expect("Failed to execute command");
            //
            //     let stdout = handle.stdout.expect("no nmcli stdout");
            //     let reader = BufReader::new(stdout);
            //     let mut lines = reader.lines();
            //
            //     let mut last_time = Instant::now();
            //     loop {
            //         let _line = lines
            //             .next_line()
            //             .await
            //             .ok()
            //             .flatten()
            //             .unwrap_or("".to_string());
            //
            //         let delta = last_time.elapsed();
            //
            //         if delta.as_millis() > 50 {
            //             sleep(Duration::from_millis(500)).await;
            //
            //             let _ = output
            //                 .send(Message::NetUpdateActive((
            //                     get_active_connection().await,
            //                     get_vpn().await,
            //                 )))
            //                 .await;
            //
            //             last_time = Instant::now();
            //         }
            //     }
            // }),
        ])
    }
}
