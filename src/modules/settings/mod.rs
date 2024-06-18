use self::{
    audio::{Audio, AudioMessage},
    battery::{battery_indicator, settings_battery_indicator},
    bluetooth::BluetoothMessage,
    net::NetMessage,
    power::PowerMessage,
    powerprofiles::{PowerProfiles, PowerProfilesMessage},
};
use crate::{
    components::icons::{icon, Icons},
    config::SettingsModuleConfig,
    menu::{Menu, MenuType},
    modules::settings::power::power_menu,
    password_dialog,
    style::{
        HeaderButtonStyle, QuickSettingsButtonStyle, QuickSettingsSubMenuButtonStyle,
        SettingsButtonStyle, MANTLE, RED,
    },
    utils::{
        battery::{BatteryData, BatteryStatus},
        idle_inhibitor::WaylandIdleInhibitor,
    },
};
use bluetooth::Bluetooth;
use brightness::{Brightness, BrightnessMessage};
use iced::{
    theme::Button,
    widget::{
        button, column, container, horizontal_space, row, text, vertical_rule, Column, Row, Space,
    },
    Alignment, Border, Element, Length, Subscription, Theme,
};
use net::Net;

pub mod audio;
mod battery;
pub mod bluetooth;
pub mod brightness;
pub mod net;
mod power;
pub mod powerprofiles;

pub struct Settings {
    audio: Audio,
    brightness: Brightness,
    net: Net,
    bluetooth: Bluetooth,
    powerprofiles: PowerProfiles,
    idle_inhibitor: Option<WaylandIdleInhibitor>,
    sub_menu: Option<SubMenu>,
    battery_data: Option<BatteryData>,
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
    Brightness(BrightnessMessage),
    ToggleInhibitIdle,
    Lock,
    Power(PowerMessage),
    ToggleSubMenu(SubMenu),
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
            audio: Audio::new(),
            brightness: Brightness::new(),
            net: Net::new(),
            bluetooth: Bluetooth::new(),
            powerprofiles: PowerProfiles::new(),
            idle_inhibitor: WaylandIdleInhibitor::new().ok(),
            sub_menu: None,
            battery_data: None,
            password_dialog: None,
        }
    }

    pub fn update(
        &mut self,
        message: Message,
        config: &SettingsModuleConfig,
        menu: &mut Menu,
    ) -> iced::Command<Message> {
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
            Message::Net(msg) => self
                .net
                .update(msg, menu, &mut self.password_dialog, config),
            Message::Bluetooth(msg) => {
                self.bluetooth
                    .update(msg, menu, &mut self.sub_menu, config)
            }
            Message::PowerProfiles(msg) => self.powerprofiles.update(msg),
            Message::Audio(msg) => self.audio.update(msg, menu, config),
            Message::Brightness(msg) => self.brightness.update(msg),
            Message::ToggleSubMenu(menu_type) => {
                if self.sub_menu == Some(menu_type) {
                    self.sub_menu.take();
                } else {
                    match menu_type {
                        SubMenu::Vpn => {
                            self.net.get_vpn_connections();
                        }
                        SubMenu::Wifi => {
                            self.net.get_nearby_wifi();
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
                if let Some(lock_cmd) = &config.lock_cmd {
                    crate::utils::launcher::execute_command(lock_cmd.to_string());
                }
                iced::Command::none()
            }
            Message::Power(msg) => {
                msg.update();
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
                        self.net.activate_wifi(ssid, password);
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

        if let Some(powerprofiles_indicator) = self.powerprofiles.indicator() {
            elements = elements.push(powerprofiles_indicator);
        }

        if let Some(sink_indicator) = self.audio.sink_indicator() {
            elements = elements.push(sink_indicator);
        }

        let mut net_elements = row!().spacing(4);
        if let Some(indicator) = self.net.active_connection_indicator() {
            net_elements = net_elements.push(indicator);
        }

        if let Some(indicator) = self.net.vpn_indicator() {
            net_elements = net_elements.push(indicator);
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

    pub fn menu_view(&self, config: &SettingsModuleConfig) -> Element<Message> {
        Column::with_children(if let Some((_, current_password)) = &self.password_dialog {
            vec![password_dialog::view("ssid", current_password).map(Message::PasswordDialog)]
        } else {
            let battery_data = self.battery_data.map(settings_battery_indicator);
            let right_buttons = Row::with_children(
                vec![
                    config.lock_cmd.as_ref().map(|_| {
                        button(icon(Icons::Lock))
                            .padding([8, 13])
                            .on_press(Message::Lock)
                            .style(Button::custom(SettingsButtonStyle))
                            .into()
                    }),
                    Some(
                        button(icon(if self.sub_menu == Some(SubMenu::Power) {
                            Icons::Close
                        } else {
                            Icons::Power
                        }))
                        .padding([8, 13])
                        .on_press(Message::ToggleSubMenu(SubMenu::Power))
                        .style(Button::custom(SettingsButtonStyle))
                        .into(),
                    ),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
            )
            .spacing(8);

            let header = if let Some(battery_data) = battery_data {
                row!(battery_data, Space::with_width(Length::Fill), right_buttons)
                    .spacing(8)
                    .width(Length::Fill)
            } else {
                row!(Space::with_width(Length::Fill), right_buttons).width(Length::Fill)
            };

            let (sink_slider, source_slider) = self.audio.audio_sliders(self.sub_menu);

            let wifi_setting_button = self
                .net
                .get_wifi_quick_setting_button(self.sub_menu, config.wifi_more_cmd.is_some());
            let quick_settings = quick_settings_section(
                vec![
                    wifi_setting_button,
                    self.net
                        .get_vpn_quick_setting_button(self.sub_menu, config.vpn_more_cmd.is_some()),
                    self.bluetooth.get_quick_setting_button(
                        self.sub_menu,
                        config.bluetooth_more_cmd.is_some(),
                    ),
                    self.powerprofiles.get_quick_setting_button(),
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
                        sub_menu_wrapper(
                            self.audio
                                .sinks_submenu(config.audio_sinks_more_cmd.is_some()),
                        )
                    }),
                source_slider,
                self.sub_menu
                    .filter(|menu_type| *menu_type == SubMenu::Sources)
                    .map(|_| {
                        sub_menu_wrapper(
                            self.audio
                                .sources_submenu(config.audio_sources_more_cmd.is_some()),
                        )
                    }),
                Some(self.brightness.brightness_slider()),
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
            self.audio.subscription().map(Message::Audio),
            self.brightness.subscription().map(Message::Brightness),
            self.net.subscription().map(Message::Net),
            self.bluetooth.subscription().map(Message::Bluetooth),
            self.powerprofiles
                .subscription()
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
