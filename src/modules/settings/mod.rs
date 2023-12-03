use self::{battery::battery_indicator, net::net_indicator};
use crate::{
    style::HeaderButtonStyle,
    utils::{
        battery::{get_battery_capacity, BatteryData},
        net::{get_active_connection, get_vpn, ActiveConnection, Vpn},
    },
};
use iced::{
    futures::SinkExt,
    theme::Button,
    widget::{button, row},
    Element, Subscription,
};
use std::{
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    time::sleep,
};

mod battery;
mod net;

pub struct Settings {
    battery_data: Option<BatteryData>,
    active_connection: Option<ActiveConnection>,
    active_vpn: Vec<Vpn>,
}

#[derive(Debug, Clone)]
pub enum Message {
    BatteryUpdate,
    NetUpdateActive((Option<ActiveConnection>, Vec<Vpn>)),
}

impl Settings {
    pub fn new() -> Self {
        Settings {
            battery_data: get_battery_capacity(),
            active_connection: None,
            active_vpn: vec![],
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::BatteryUpdate => {
                get_battery_capacity();
            }
            Message::NetUpdateActive((active_connection, active_vpn)) => {
                self.active_connection = active_connection;
                self.active_vpn = active_vpn;
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let mut elements = row!().spacing(8);

        if self.active_connection.is_some() || !self.active_vpn.is_empty() {
            elements = elements.push(net_indicator(&self.active_connection, &self.active_vpn));
        }

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
            iced::subscription::channel("wifi-signal-monitor", 10, |mut output| async move {
                let get_net = || async move { (get_active_connection().await, get_vpn().await) };

                let _ = output.send(Message::NetUpdateActive(get_net().await)).await;

                loop {
                    let _ = output.send(Message::NetUpdateActive(get_net().await)).await;
                    sleep(Duration::from_secs(60)).await
                }
            }),
            iced::subscription::channel("nmcli-monitor", 10, |mut output| async move {
                let _ = output
                    .send(Message::NetUpdateActive((
                        get_active_connection().await,
                        get_vpn().await,
                    )))
                    .await;

                let handle = Command::new("nmcli")
                    .arg("monitor")
                    .stdout(Stdio::piped())
                    .stdin(Stdio::null())
                    .spawn()
                    .expect("Failed to execute command");

                let stdout = handle.stdout.expect("no nmcli stdout");
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();

                let mut last_time = Instant::now();
                loop {
                    let _line = lines
                        .next_line()
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or("".to_string());

                    let delta = last_time.elapsed();

                    if delta.as_millis() > 50 {
                        sleep(Duration::from_millis(500)).await;

                        let _ = output
                            .send(Message::NetUpdateActive((
                                get_active_connection().await,
                                get_vpn().await,
                            )))
                            .await;

                        last_time = Instant::now();
                    }
                }
            }),
        ])
    }
}
