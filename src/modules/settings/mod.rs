use crate::{
    components::icons::{Icons, icon},
    config::{Position, SettingsModuleConfig},
    modules::settings::{
        audio::{AudioSettings, AudioSettingsConfig},
        bluetooth::{BluetoothSettings, BluetoothSettingsConfig},
        brightness::BrightnessSettings,
        network::{NetworkSettings, NetworkSettingsConfig},
        power::{PowerSettings, PowerSettingsConfig},
    },
    password_dialog,
    services::idle_inhibitor::IdleInhibitorManager,
    theme::AshellTheme,
};
use iced::{
    Alignment, Background, Border, Element, Length, Padding, Subscription, Task, Theme,
    alignment::{Horizontal, Vertical},
    widget::{Column, Row, Space, button, column, container, horizontal_space, row, text},
    window::Id,
};

mod audio;
mod bluetooth;
mod brightness;
mod network;
mod power;

pub struct Settings {
    lock_cmd: Option<String>,
    power: PowerSettings,
    audio: AudioSettings,
    brightness: BrightnessSettings,
    network: NetworkSettings,
    bluetooth: BluetoothSettings,
    idle_inhibitor: Option<IdleInhibitorManager>,
    sub_menu: Option<SubMenu>,
    password_dialog: Option<(String, String)>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Network(network::Message),
    Bluetooth(bluetooth::Message),
    Audio(audio::Message),
    Brightness(brightness::Message),
    ToggleInhibitIdle,
    Lock,
    Power(power::Message),
    ToggleSubMenu(SubMenu),
    PasswordDialog(password_dialog::Message),
    MenuOpened,
    ConfigReloaded(SettingsModuleConfig),
}

pub enum Action {
    None,
    Command(Task<Message>),
    CloseMenu(Id),
    RequestKeyboard(Id),
    ReleaseKeyboard(Id),
    ReleaseKeyboardWithCommand(Id, Task<Message>),
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
    pub fn new(config: SettingsModuleConfig) -> Self {
        Settings {
            lock_cmd: config.lock_cmd,
            power: PowerSettings::new(PowerSettingsConfig::new(
                config.suspend_cmd,
                config.reboot_cmd,
                config.shutdown_cmd,
                config.logout_cmd,
            )),
            audio: AudioSettings::new(AudioSettingsConfig::new(
                config.audio_sinks_more_cmd,
                config.audio_sources_more_cmd,
            )),
            brightness: BrightnessSettings::new(),
            network: NetworkSettings::new(NetworkSettingsConfig::new(
                config.wifi_more_cmd,
                config.vpn_more_cmd,
                config.remove_airplane_btn,
            )),
            bluetooth: BluetoothSettings::new(BluetoothSettingsConfig::new(
                config.bluetooth_more_cmd,
            )),
            idle_inhibitor: if config.remove_idle_btn {
                None
            } else {
                IdleInhibitorManager::new()
            },
            sub_menu: None,
            password_dialog: None,
        }
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::Power(msg) => match self.power.update(msg) {
                power::Action::None => Action::None,
                power::Action::Command(task) => Action::Command(task.map(Message::Power)),
            },
            Message::Audio(msg) => match self.audio.update(msg) {
                audio::Action::None => Action::None,
                audio::Action::ToggleSinksMenu => {
                    if self.sub_menu == Some(SubMenu::Sinks) {
                        self.sub_menu.take();
                    } else {
                        self.sub_menu.replace(SubMenu::Sinks);
                    }
                    Action::None
                }
                audio::Action::ToggleSourcesMenu => {
                    if self.sub_menu == Some(SubMenu::Sources) {
                        self.sub_menu.take();
                    } else {
                        self.sub_menu.replace(SubMenu::Sources);
                    }
                    Action::None
                }
                audio::Action::CloseSubMenu => {
                    if self.sub_menu == Some(SubMenu::Sinks)
                        || self.sub_menu == Some(SubMenu::Sources)
                    {
                        self.sub_menu.take();
                    }
                    Action::None
                }
                audio::Action::CloseMenu(id) => Action::CloseMenu(id),
            },
            Message::Network(msg) => match self.network.update(msg) {
                network::Action::None => Action::None,
                network::Action::RequestPasswordForSSID(ssid) => {
                    self.password_dialog = Some((ssid, "".to_string()));
                    Action::None
                }
                network::Action::RequestPassword(id, ssid) => {
                    self.password_dialog = Some((ssid, "".to_string()));
                    Action::RequestKeyboard(id)
                }
                network::Action::Command(task) => Action::Command(task.map(Message::Network)),
                network::Action::ToggleWifiMenu => {
                    if self.sub_menu == Some(SubMenu::Wifi) {
                        self.sub_menu.take();
                    } else {
                        self.sub_menu.replace(SubMenu::Wifi);
                    }
                    Action::None
                }
                network::Action::ToggleVpnMenu => {
                    if self.sub_menu == Some(SubMenu::Vpn) {
                        self.sub_menu.take();
                    } else {
                        self.sub_menu.replace(SubMenu::Vpn);
                    }
                    Action::None
                }
                network::Action::CloseSubMenu(task) => {
                    if self.sub_menu == Some(SubMenu::Wifi) || self.sub_menu == Some(SubMenu::Vpn) {
                        self.sub_menu.take();
                    }

                    Action::Command(task.map(Message::Network))
                }
                network::Action::CloseMenu(id) => Action::CloseMenu(id),
            },
            Message::Bluetooth(msg) => match self.bluetooth.update(msg) {
                bluetooth::Action::None => Action::None,
                bluetooth::Action::ToggleBluetoothMenu => {
                    if self.sub_menu == Some(SubMenu::Bluetooth) {
                        self.sub_menu.take();
                    } else {
                        self.sub_menu.replace(SubMenu::Bluetooth);
                    }
                    Action::None
                }
                bluetooth::Action::CloseSubMenu(task) => {
                    if self.sub_menu == Some(SubMenu::Bluetooth) {
                        self.sub_menu.take();
                    }

                    Action::Command(task.map(Message::Bluetooth))
                }
                bluetooth::Action::CloseMenu(id) => Action::CloseMenu(id),
            },
            Message::Brightness(msg) => match self.brightness.update(msg) {
                brightness::Action::None => Action::None,
                brightness::Action::Command(task) => Action::Command(task.map(Message::Brightness)),
            },
            Message::ToggleSubMenu(menu_type) => {
                if self.sub_menu == Some(menu_type) {
                    self.sub_menu.take();

                    Action::None
                } else {
                    self.sub_menu.replace(menu_type);

                    if menu_type == SubMenu::Wifi {
                        match self.network.update(network::Message::WifiMenuOpened) {
                            network::Action::Command(task) => {
                                Action::Command(task.map(Message::Network))
                            }
                            _ => Action::None,
                        }
                    } else {
                        Action::None
                    }
                }
            }
            Message::ToggleInhibitIdle => {
                if let Some(idle_inhibitor) = &mut self.idle_inhibitor {
                    idle_inhibitor.toggle();
                }
                Action::None
            }
            Message::Lock => {
                if let Some(lock_cmd) = &self.lock_cmd {
                    crate::utils::launcher::execute_command(lock_cmd.to_string());
                }
                Action::None
            }
            Message::PasswordDialog(msg) => match msg {
                password_dialog::Message::PasswordChanged(password) => {
                    if let Some((_, current_password)) = &mut self.password_dialog {
                        *current_password = password;
                    }

                    Action::None
                }
                password_dialog::Message::DialogConfirmed(id) => {
                    if let Some((ssid, password)) = self.password_dialog.take() {
                        match self
                            .network
                            .update(network::Message::PasswordDialogConfirmed(
                                ssid.clone(),
                                password.clone(),
                            )) {
                            network::Action::Command(task) => {
                                Action::ReleaseKeyboardWithCommand(id, task.map(Message::Network))
                            }
                            _ => Action::ReleaseKeyboard(id),
                        }
                    } else {
                        Action::ReleaseKeyboard(id)
                    }
                }
                password_dialog::Message::DialogCancelled(id) => {
                    self.password_dialog = None;

                    Action::ReleaseKeyboard(id)
                }
            },
            Message::MenuOpened => {
                self.sub_menu = None;

                match self.brightness.update(brightness::Message::MenuOpened) {
                    brightness::Action::None => Action::None,
                    brightness::Action::Command(task) => {
                        Action::Command(task.map(Message::Brightness))
                    }
                }
            }
            Message::ConfigReloaded(config) => {
                self.lock_cmd = config.lock_cmd;
                self.power
                    .update(power::Message::ConfigReloaded(PowerSettingsConfig::new(
                        config.suspend_cmd,
                        config.reboot_cmd,
                        config.shutdown_cmd,
                        config.logout_cmd,
                    )));
                self.audio
                    .update(audio::Message::ConfigReloaded(AudioSettingsConfig::new(
                        config.audio_sinks_more_cmd,
                        config.audio_sources_more_cmd,
                    )));
                self.network.update(network::Message::ConfigReloaded(
                    NetworkSettingsConfig::new(
                        config.wifi_more_cmd,
                        config.vpn_more_cmd,
                        config.remove_airplane_btn,
                    ),
                ));
                self.bluetooth.update(bluetooth::Message::ConfigReloaded(
                    BluetoothSettingsConfig::new(config.bluetooth_more_cmd),
                ));
                if config.remove_idle_btn {
                    self.idle_inhibitor = None;
                } else if self.idle_inhibitor.is_none() {
                    self.idle_inhibitor = IdleInhibitorManager::new();
                }
                Action::None
            }
        }
    }

    pub fn menu_view<'a>(
        &'a self,
        id: Id,
        theme: &'a AshellTheme,
        position: Position,
    ) -> Element<'a, Message> {
        if let Some((ssid, current_password)) = &self.password_dialog {
            password_dialog::view(id, theme, ssid, current_password).map(Message::PasswordDialog)
        } else {
            let battery_data = self
                .power
                .battery_menu_indicator(theme)
                .map(|e| e.map(Message::Power));
            let right_buttons = Row::new()
                .push_maybe(self.lock_cmd.as_ref().map(|_| {
                    button(icon(Icons::Lock))
                        .padding([theme.space.xs, theme.space.sm + 1])
                        .on_press(Message::Lock)
                        .style(theme.settings_button_style())
                }))
                .push(
                    button(icon(if self.sub_menu == Some(SubMenu::Power) {
                        Icons::Close
                    } else {
                        Icons::Power
                    }))
                    .padding([theme.space.xs, theme.space.sm + 1])
                    .on_press(Message::ToggleSubMenu(SubMenu::Power))
                    .style(theme.settings_button_style()),
                )
                .spacing(theme.space.xs);

            let header = Row::new()
                .push_maybe(battery_data)
                .push(Space::with_width(Length::Fill))
                .push(right_buttons)
                .spacing(theme.space.xs)
                .width(Length::Fill);

            let (sink_slider, source_slider) = self.audio.sliders(theme, self.sub_menu);

            let wifi_setting_button = self
                .network
                .wifi_quick_setting_button(id, theme, self.sub_menu)
                .map(|(button, submenu)| {
                    (
                        button.map(Message::Network),
                        submenu.map(|e| e.map(Message::Network)),
                    )
                });
            let quick_settings = quick_settings_section(
                theme,
                vec![
                    wifi_setting_button,
                    self.bluetooth
                        .quick_setting_button(id, theme, self.sub_menu)
                        .map(|(button, submenu)| {
                            (
                                button.map(Message::Bluetooth),
                                submenu.map(|e| e.map(Message::Bluetooth)),
                            )
                        }),
                    self.network
                        .vpn_quick_setting_button(id, theme, self.sub_menu)
                        .map(|(button, submenu)| {
                            (
                                button.map(Message::Network),
                                submenu.map(|e| e.map(Message::Network)),
                            )
                        }),
                    self.network
                        .airplane_mode_quick_setting_button(theme)
                        .map(|(button, _)| (button.map(Message::Network), None)),
                    self.idle_inhibitor.as_ref().map(|idle_inhibitor| {
                        (
                            quick_setting_button(
                                theme,
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
                    self.power
                        .quick_setting_button(theme)
                        .map(|(button, submenu)| {
                            (
                                button.map(Message::Power),
                                submenu.map(|e| e.map(Message::Power)),
                            )
                        }),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
            );

            let (top_sink_slider, bottom_sink_slider) = match position {
                Position::Top => (sink_slider.map(|e| e.map(Message::Audio)), None),
                Position::Bottom => (None, sink_slider.map(|e| e.map(Message::Audio))),
            };
            let (top_source_slider, bottom_source_slider) = match position {
                Position::Top => (source_slider.map(|e| e.map(Message::Audio)), None),
                Position::Bottom => (None, source_slider.map(|e| e.map(Message::Audio))),
            };

            Column::new()
                .push(header)
                .push_maybe(
                    self.sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Power)
                        .map(|_| {
                            sub_menu_wrapper(theme, self.power.menu(theme).map(Message::Power))
                        }),
                )
                .push_maybe(top_sink_slider)
                .push_maybe(
                    self.sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Sinks)
                        .and_then(|_| {
                            self.audio
                                .sinks_submenu(id, theme)
                                .map(|submenu| sub_menu_wrapper(theme, submenu.map(Message::Audio)))
                        }),
                )
                .push_maybe(bottom_sink_slider)
                .push_maybe(top_source_slider)
                .push_maybe(
                    self.sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Sources)
                        .and_then(|_| {
                            self.audio
                                .sources_submenu(id, theme)
                                .map(|submenu| sub_menu_wrapper(theme, submenu.map(Message::Audio)))
                        }),
                )
                .push_maybe(bottom_source_slider)
                .push_maybe(
                    self.brightness
                        .slider(theme)
                        .map(|e| e.map(Message::Brightness)),
                )
                .push(quick_settings)
                .spacing(theme.space.md)
                .into()
        }
    }

    pub fn view<'a>(&'a self, theme: &'a AshellTheme) -> Element<'a, Message> {
        Row::new()
            .push_maybe(
                self.idle_inhibitor
                    .as_ref()
                    .filter(|i| i.is_inhibited())
                    .map(|_| {
                        container(icon(Icons::EyeOpened)).style(|theme: &Theme| container::Style {
                            text_color: Some(theme.palette().danger),
                            ..Default::default()
                        })
                    }),
            )
            .push_maybe(
                self.power
                    .power_profile_indicator()
                    .map(|e| e.map(Message::Power)),
            )
            .push_maybe(self.audio.sink_indicator().map(|e| e.map(Message::Audio)))
            .push(
                Row::new()
                    .push_maybe(
                        self.network
                            .connection_indicator(theme)
                            .map(|e| e.map(Message::Network)),
                    )
                    .push_maybe(
                        self.network
                            .vpn_indicator(theme)
                            .map(|e| e.map(Message::Network)),
                    )
                    .spacing(theme.space.xxs),
            )
            .push_maybe(
                self.power
                    .battery_indicator(theme)
                    .map(|e| e.map(Message::Power)),
            )
            .spacing(theme.space.xs)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            self.power.subscription().map(Message::Power),
            self.audio.subscription().map(Message::Audio),
            self.brightness.subscription().map(Message::Brightness),
            self.network.subscription().map(Message::Network),
            self.bluetooth.subscription().map(Message::Bluetooth),
        ])
    }
}

fn quick_settings_section<'a>(
    theme: &'a AshellTheme,
    buttons: Vec<(Element<'a, Message>, Option<Element<'a, Message>>)>,
) -> Element<'a, Message> {
    let mut section = column!().spacing(theme.space.xs);

    let mut before: Option<(Element<'a, Message>, Option<Element<'a, Message>>)> = None;

    for (button, menu) in buttons.into_iter() {
        match before.take() {
            Some((before_button, before_menu)) => {
                section = section.push(
                    row![before_button, button]
                        .width(Length::Fill)
                        .spacing(theme.space.xs),
                );

                if let Some(menu) = before_menu {
                    section = section.push(sub_menu_wrapper(theme, menu));
                }

                if let Some(menu) = menu {
                    section = section.push(sub_menu_wrapper(theme, menu));
                }
            }
            _ => {
                before = Some((button, menu));
            }
        }
    }

    if let Some((before_button, before_menu)) = before.take() {
        section = section.push(
            row![before_button, horizontal_space()]
                .width(Length::Fill)
                .spacing(theme.space.xs),
        );

        if let Some(menu) = before_menu {
            section = section.push(sub_menu_wrapper(theme, menu));
        }
    }

    section.into()
}

fn sub_menu_wrapper<'a, Msg: 'static>(
    ashell_theme: &'a AshellTheme,
    content: Element<'a, Msg>,
) -> Element<'a, Msg> {
    container(content)
        .style(move |theme: &Theme| container::Style {
            background: Background::Color(
                theme
                    .extended_palette()
                    .secondary
                    .strong
                    .color
                    .scale_alpha(ashell_theme.opacity),
            )
            .into(),
            border: Border::default().rounded(ashell_theme.radius.lg),
            ..container::Style::default()
        })
        .padding(ashell_theme.space.md)
        .width(Length::Fill)
        .into()
}

fn quick_setting_button<'a, Msg: Clone + 'static>(
    theme: &'a AshellTheme,
    icon_type: Icons,
    title: String,
    subtitle: Option<String>,
    active: bool,
    on_press: Msg,
    with_submenu: Option<(SubMenu, Option<SubMenu>, Msg)>,
) -> Element<'a, Msg> {
    let main_content = row!(
        icon(icon_type).size(theme.font_size.lg),
        Column::new()
            .push(text(title).size(theme.font_size.sm))
            .push_maybe(subtitle.map(|s| text(s).size(theme.font_size.xs)))
            .spacing(theme.space.xxs)
    )
    .spacing(theme.space.xs)
    .padding(Padding::ZERO.left(theme.space.xxs))
    .width(Length::Fill)
    .align_y(Alignment::Center);

    button(
        Row::new()
            .push(main_content)
            .push_maybe(with_submenu.map(|(menu_type, submenu, msg)| {
                button(
                    container(icon(if Some(menu_type) == submenu {
                        Icons::Close
                    } else {
                        Icons::RightChevron
                    }))
                    .align_y(Vertical::Center)
                    .align_x(Horizontal::Center),
                )
                .padding([
                    theme.space.xxs,
                    if Some(menu_type) == submenu {
                        theme.space.xs + 1
                    } else {
                        theme.space.sm
                    },
                ])
                .style(theme.quick_settings_submenu_button_style(active))
                .width(Length::Shrink)
                .height(Length::Shrink)
                .on_press(msg)
            }))
            .spacing(theme.space.xxs)
            .align_y(Alignment::Center)
            .height(Length::Fill),
    )
    .padding([theme.space.xxs, theme.space.xs])
    .on_press(on_press)
    .height(Length::Fill)
    .width(Length::Fill)
    .style(theme.quick_settings_button_style(active))
    .width(Length::Fill)
    .height(Length::Fixed(50.))
    .into()
}
