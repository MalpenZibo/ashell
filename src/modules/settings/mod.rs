use self::{
    audio::{audio_submenu, get_audio_sliders, sink_indicator, AudioMessage},
    battery::{battery_indicator, settings_battery_indicator},
    bluetooth::{get_bluetooth_quick_setting_button, BluetoothMessage, BluetoothState, Device},
    net::{
        active_connection_indicator, get_wifi_quick_setting_button, vpn_indicator, vpn_menu,
        NetMessage,
    },
    power::PowerMessage,
    powerprofiles::{
        get_powerprofiles_quick_setting_button, powerprofiles_indicator, PowerProfilesMessage,
        Profiles,
    },
};
use crate::{
    components::icons::{icon, Icons},
    menu::{Menu, MenuType},
    modules::settings::{audio::SubmenuEntry, power::power_menu},
    password_dialog,
    style::{
        HeaderButtonStyle, QuickSettingsButtonStyle, QuickSettingsSubMenuButtonStyle,
        SettingsButtonStyle, MANTLE, RED,
    },
    utils::{
        audio::{AudioCommand, Sink, Source},
        battery::{BatteryData, BatteryStatus},
        bluetooth::BluetoothCommand,
        idle_inhibitor::WaylandIdleInhibitor,
        net::{ActiveConnection, NetCommand, Vpn, WifiConnection, WifiDeviceState},
        powerprofiles::PowerProfilesCommand,
        Commander,
    },
};
use iced::{
    theme::Button,
    widget::{
        button, column, container, horizontal_space, row, slider, text, vertical_rule, Column, Row,
        Space,
    },
    Alignment, Border, Element, Length, Subscription, Theme,
};

pub mod audio;
mod battery;
pub mod bluetooth;
pub mod net;
mod power;
pub mod powerprofiles;

pub struct Settings {
    audio_commander: Commander<AudioCommand>,
    brightness_commander: Commander<f64>,
    net_commander: Commander<NetCommand>,
    bluetooth_commander: Commander<BluetoothCommand>,
    powerprofiles_commander: Commander<PowerProfilesCommand>,
    powerprofiles: Option<Profiles>,
    idle_inhibitor: Option<WaylandIdleInhibitor>,
    sub_menu: Option<SubMenu>,
    battery_data: Option<BatteryData>,
    wifi_device_state: WifiDeviceState,
    scanning_nearby_wifi: bool,
    active_connection: Option<ActiveConnection>,
    vpn_active: bool,
    vpn_connections: Vec<Vpn>,
    nearby_wifi: Vec<WifiConnection>,
    bluetooth_state: BluetoothState,
    bluetooth_devices: Vec<Device>,
    default_sink: String,
    default_source: String,
    sinks: Vec<Sink>,
    sources: Vec<Source>,
    cur_sink_volume: i32,
    cur_source_volume: i32,
    cur_brightness: i32,
    pub password_dialog: Option<(String, String)>,
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
    Bluetooth(BluetoothMessage),
    PowerProfiles(PowerProfilesMessage),
    Audio(AudioMessage),
    ToggleInhibitIdle,
    Lock,
    Power(PowerMessage),
    ToggleSubMenu(SubMenu),
    BrightnessChanged(f64),
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
    pub fn new() -> Self {
        Settings {
            audio_commander: Commander::new(),
            brightness_commander: Commander::new(),
            net_commander: Commander::new(),
            bluetooth_commander: Commander::new(),
            powerprofiles_commander: Commander::new(),
            powerprofiles: None,
            idle_inhibitor: WaylandIdleInhibitor::new().ok(),
            sub_menu: None,
            battery_data: None,
            wifi_device_state: WifiDeviceState::Unavailable,
            scanning_nearby_wifi: false,
            active_connection: None,
            vpn_active: false,
            vpn_connections: vec![],
            bluetooth_state: BluetoothState::Unavailable,
            bluetooth_devices: vec![],
            nearby_wifi: vec![],
            default_sink: String::new(),
            default_source: String::new(),
            sinks: vec![],
            sources: vec![],
            cur_sink_volume: 0,
            cur_source_volume: 0,
            cur_brightness: 0,
            password_dialog: None,
        }
    }

    pub fn update(&mut self, message: Message, menu: &mut Menu) -> iced::Command<Message> {
        match message {
            Message::ToggleMenu => {
                self.sub_menu = None;
                self.password_dialog = None;
                iced::Command::batch(vec![
                    menu.unset_keyboard_interactivity(),
                    menu.toggle(MenuType::Settings),
                ])
            }
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
            Message::Net(msg) => msg.update(self, menu),
            Message::Bluetooth(msg) => msg.update(self),
            Message::PowerProfiles(msg) => msg.update(self),
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
            Message::ToggleInhibitIdle => {
                if let Some(idle_inhibitor) = &mut self.idle_inhibitor {
                    let _ = idle_inhibitor.toggle();
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
                if (value - (self.cur_brightness as f64 / 100.)).abs() > 0.01 {
                    self.cur_brightness = (value * 100.).round() as i32;
                    self.brightness_commander.send(value).unwrap();
                }
                iced::Command::none()
            }
            Message::PasswordDialog(msg) => match msg {
                password_dialog::Message::PasswordChanged(password) => {
                    if let Some((_, current_password)) = &mut self.password_dialog {
                        *current_password = password;
                    }

                    iced::Command::none()
                }
                password_dialog::Message::DialogConfirmed => {
                    if let Some((ssid, password)) = self.password_dialog.take() {
                        let _ = self
                            .net_commander
                            .send(NetCommand::ActivateWifiConnection(ssid, Some(password)));

                        menu.unset_keyboard_interactivity()
                    } else {
                        iced::Command::none()
                    }
                }
                password_dialog::Message::DialogCancelled => {
                    if let Some((_, _)) = self.password_dialog.take() {
                        menu.unset_keyboard_interactivity()
                    } else {
                        iced::Command::none()
                    }
                }
            },
        }
    }

    pub fn view(&self) -> Element<Message> {
        let mut elements = row!().spacing(8);

        if self
            .idle_inhibitor
            .as_ref()
            .filter(|i| i.is_inhibited())
            .is_some()
        {
            elements = elements.push(icon(Icons::EyeOpened).style(RED));
        }

        if let Some(powerprofiles_indicator) = powerprofiles_indicator(self) {
            elements = elements.push(powerprofiles_indicator);
        }

        if let Some(sink_indicator) = sink_indicator(&self.sinks) {
            elements = elements.push(sink_indicator);
        }

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
            .padding([2, 8])
            .on_press(Message::ToggleMenu)
            .into()
    }

    pub fn menu_view(&self) -> Element<Message> {
        Column::with_children(if let Some((_, current_password)) = &self.password_dialog {
            vec![password_dialog::view("ssid", current_password).map(Message::PasswordDialog)]
        } else {
            let battery_data = self.battery_data.map(settings_battery_indicator);
            let right_buttons = row!(
                button(icon(Icons::Lock))
                    .padding([8, 13])
                    .on_press(Message::Lock)
                    .style(Button::custom(SettingsButtonStyle)),
                button(icon(if self.sub_menu == Some(SubMenu::Power) {
                    Icons::Close
                } else {
                    Icons::Power
                }))
                .padding([8, 13])
                .on_press(Message::ToggleSubMenu(SubMenu::Power))
                .style(Button::custom(SettingsButtonStyle))
            )
            .spacing(8);

            let header = if let Some(battery_data) = battery_data {
                row!(battery_data, Space::with_width(Length::Fill), right_buttons)
                    .spacing(8)
                    .width(Length::Fill)
            } else {
                row!(Space::with_width(Length::Fill), right_buttons).width(Length::Fill)
            };

            let (sink_slider, source_slider) = get_audio_sliders(
                &self.sinks,
                self.cur_sink_volume,
                &self.sources,
                self.cur_source_volume,
                self.sub_menu,
            );

            let brightness_slider = row!(
                container(icon(Icons::Brightness)).padding([8, 11]),
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

            let wifi_setting_button = get_wifi_quick_setting_button(self);
            let quick_settings = quick_settings_section(
                vec![
                    wifi_setting_button,
                    Some((
                        quick_setting_button(
                            Icons::Vpn,
                            "Vpn".to_string(),
                            None,
                            self.vpn_active,
                            Message::ToggleSubMenu(SubMenu::Vpn),
                            None,
                        ),
                        self.sub_menu
                            .filter(|menu_type| *menu_type == SubMenu::Vpn)
                            .map(|_| {
                                sub_menu_wrapper(vpn_menu(&self.vpn_connections)).map(Message::Net)
                            }),
                    )),
                    get_bluetooth_quick_setting_button(self),
                    get_powerprofiles_quick_setting_button(self),
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
                            ),
                            None,
                        )
                    }),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
            );

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
            .collect::<Vec<_>>()
        })
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
            crate::utils::bluetooth::subscription(self.bluetooth_commander.give_receiver())
                .map(Message::Bluetooth),
            crate::utils::powerprofiles::subscription(self.powerprofiles_commander.give_receiver())
                .map(Message::PowerProfiles),
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
            border: Border::with_radius(16),
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
    let main_content = row!(
        icon(icon_type).size(20),
        Column::with_children(
            vec![
                Some(text(title).size(12).into()),
                subtitle.map(|s| text(s).size(10).into()),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
        )
        .spacing(4)
    )
    .spacing(8)
    .padding([0, 0, 0, 4])
    .width(Length::Fill)
    .align_items(Alignment::Center);

    button(
        Row::with_children(
            vec![
                Some(main_content.into()),
                with_submenu.as_ref().map(|_| vertical_rule(1).into()),
                with_submenu.map(|(menu_type, submenu, msg)| {
                    button(
                        container(icon(if Some(menu_type) == submenu {
                            Icons::Close
                        } else {
                            Icons::VerticalDots
                        }))
                        .align_y(iced::alignment::Vertical::Center)
                        .align_x(iced::alignment::Horizontal::Center),
                    )
                    .padding([4, if Some(menu_type) == submenu { 9 } else { 12 }])
                    .style(Button::custom(QuickSettingsSubMenuButtonStyle(active)))
                    .width(Length::Shrink)
                    .height(Length::Shrink)
                    .on_press(msg)
                    .into()
                }),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
        )
        .spacing(4)
        .align_items(Alignment::Center)
        .height(Length::Fill),
    )
    .padding([4, 8])
    .on_press(on_press)
    .height(Length::Fill)
    .width(Length::Fill)
    .style(Button::custom(QuickSettingsButtonStyle(active)))
    .width(Length::Fill)
    .height(Length::Fixed(50.))
    .into()
}
