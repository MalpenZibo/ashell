use std::collections::HashMap;
use std::time::Duration;

use iced::futures::future::join_all;
use log::{debug, error};
use tokio::process::Command;
use tokio::time::timeout;

use crate::{
    components::{
        ButtonUIRef, MenuSize,
        icons::{DynamicIcon, Icon, StaticIcon, icon, icon_button},
        menu::MenuType,
        password_dialog, position_button, quick_setting_button, sub_menu_wrapper,
    },
    config::{Position, SettingsCustomButton, SettingsIndicator, SettingsModuleConfig},
    modules::settings::{
        audio::{AudioSettings, AudioSettingsConfig},
        bluetooth::{BluetoothSettings, BluetoothSettingsConfig},
        brightness::BrightnessSettings,
        network::{NetworkSettings, NetworkSettingsConfig},
        power::{PowerSettings, PowerSettingsConfig},
    },
    services::idle_inhibitor::IdleInhibitorManager,
    t,
    theme::use_theme,
};
use iced::{
    Border, Color, Element, Length, Shadow, Subscription, SurfaceId, Task, Theme, Vector,
    widget::{Column, Row, Space, container, row, space},
};

pub(crate) mod audio;
mod bluetooth;
pub(crate) mod brightness;
pub(crate) mod network;
mod power;

type TooltipConfig = (fn(ButtonUIRef, SurfaceId) -> Message, MenuType);

pub struct Settings {
    lock_cmd: Option<String>,
    power: PowerSettings,
    audio: AudioSettings,
    brightness: BrightnessSettings,
    network: NetworkSettings,
    bluetooth: BluetoothSettings,
    idle_inhibitor: Option<IdleInhibitorManager>,
    sub_menu: Option<SubMenu>,
    network_dialog: Option<NetworkDialogState>,
    network_dialog_show_password: bool,
    indicators: Vec<SettingsIndicator>,
    custom_buttons: Vec<SettingsCustomButton>,
    custom_buttons_status: HashMap<String, Option<bool>>,
    enable_tooltips: bool,
}

#[derive(Debug, Clone)]
enum NetworkDialogKind {
    Password,
    OpenNetworkWarning,
}

#[derive(Debug, Clone)]
struct NetworkDialogState {
    ssid: String,
    password: Option<String>,
    kind: NetworkDialogKind,
}

impl NetworkDialogState {
    fn new_password_dialog(ssid: String) -> Self {
        Self {
            ssid,
            password: Some(String::new()),
            kind: NetworkDialogKind::Password,
        }
    }

    fn new_warning_dialog(ssid: String) -> Self {
        Self {
            ssid,
            password: None,
            kind: NetworkDialogKind::OpenNetworkWarning,
        }
    }
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
    CustomButton(String),
    CustomButtonsStatus(Vec<(String, Option<bool>)>),
    MenuOpened,
    ConfigReloaded(SettingsModuleConfig),
    AudioTooltipHover(ButtonUIRef, SurfaceId),
    BluetoothTooltipHover(ButtonUIRef, SurfaceId),
    WifiTooltipHover(ButtonUIRef, SurfaceId),
    VpnTooltipHover(ButtonUIRef, SurfaceId),
    BatteryTooltipHover(ButtonUIRef, SurfaceId),
    PeripheralBatteryTooltipHover(ButtonUIRef, SurfaceId, usize),
    TooltipUnhover(SurfaceId, MenuType),
}

pub enum Action {
    None,
    Command(Task<Message>),
    CloseMenu(SurfaceId),
    RequestKeyboard(SurfaceId),
    ReleaseKeyboard(SurfaceId),
    ReleaseKeyboardWithCommand(SurfaceId, Task<Message>),
    OpenTooltipMenu(SurfaceId, MenuType, ButtonUIRef),
    CloseTooltipMenu(SurfaceId, MenuType),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SubMenu {
    PeripheralMenu,
    Power,
    Sinks,
    Sources,
    Wifi,
    Vpn,
    Bluetooth,
}

impl Settings {
    pub fn audio(&self) -> &AudioSettings {
        &self.audio
    }

    pub fn brightness(&self) -> &BrightnessSettings {
        &self.brightness
    }

    pub fn network(&self) -> &NetworkSettings {
        &self.network
    }

    pub fn idle_inhibitor(&self) -> &Option<IdleInhibitorManager> {
        &self.idle_inhibitor
    }

    pub fn volume_adjust(&mut self, up: bool) -> Action {
        match self.audio.volume_adjust(up) {
            audio::Action::Task(task) => Action::Command(task.map(Message::Audio)),
            _ => Action::None,
        }
    }

    pub fn toggle_mute(&mut self) -> Action {
        self.audio.toggle_mute();
        Action::None
    }

    pub fn microphone_adjust(&mut self, up: bool) -> Action {
        match self.audio.microphone_adjust(up) {
            audio::Action::Task(task) => Action::Command(task.map(Message::Audio)),
            _ => Action::None,
        }
    }

    pub fn microphone_toggle_mute(&mut self) -> Action {
        self.audio.microphone_toggle_mute();
        Action::None
    }

    pub fn brightness_adjust(&mut self, up: bool) -> Action {
        match self.brightness.brightness_adjust(up) {
            brightness::Action::Command(task) => Action::Command(task.map(Message::Brightness)),
            brightness::Action::None => Action::None,
        }
    }

    pub fn toggle_airplane(&mut self) -> Action {
        match self.network.update(network::Message::ToggleAirplaneMode) {
            network::Action::CloseSubMenu(task) => Action::Command(task.map(Message::Network)),
            network::Action::Command(task) => Action::Command(task.map(Message::Network)),
            _ => Action::None,
        }
    }

    pub fn toggle_idle_inhibitor(&mut self) -> Action {
        if let Some(idle_inhibitor) = &mut self.idle_inhibitor {
            idle_inhibitor.toggle();
        }
        Action::None
    }

    pub fn new(config: SettingsModuleConfig) -> Self {
        Settings {
            lock_cmd: config.lock_cmd,
            power: PowerSettings::new(PowerSettingsConfig::new(
                config.suspend_cmd,
                config.hibernate_cmd,
                config.reboot_cmd,
                config.shutdown_cmd,
                config.logout_cmd,
                config.battery_format,
                config.battery_hide_when_full,
                config.peripheral_indicators,
                config.peripheral_battery_format,
                config.peripheral_expanded_by_default,
            )),
            audio: AudioSettings::new(AudioSettingsConfig::new(
                config.audio_sinks_more_cmd,
                config.audio_sources_more_cmd,
                config.volume_step,
                config.max_volume,
                config.audio_indicator_format,
                config.microphone_indicator_format,
            )),
            brightness: BrightnessSettings::new(config.brightness_indicator_format),
            network: NetworkSettings::new(NetworkSettingsConfig::new(
                config.wifi_more_cmd,
                config.vpn_more_cmd,
                config.remove_airplane_btn,
                config.network_indicator_format,
            )),
            bluetooth: BluetoothSettings::new(BluetoothSettingsConfig::new(
                config.bluetooth_more_cmd,
                config.bluetooth_indicator_format,
            )),
            idle_inhibitor: if config.remove_idle_btn {
                None
            } else {
                IdleInhibitorManager::new()
            },
            sub_menu: None,
            network_dialog: None,
            indicators: config.indicators,
            custom_buttons: config.custom_buttons,
            custom_buttons_status: HashMap::new(),
            network_dialog_show_password: false,
            enable_tooltips: config.enable_tooltips,
        }
    }

    fn toggle_sub_menu(&mut self, target: SubMenu) {
        if self.sub_menu == Some(target) {
            self.sub_menu.take();
        } else {
            self.sub_menu.replace(target);
        }
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::Power(msg) => match self.power.update(msg) {
                power::Action::None => Action::None,
                power::Action::TogglePeripheralMenu => {
                    self.toggle_sub_menu(SubMenu::PeripheralMenu);
                    Action::None
                }
                power::Action::Command(task) => Action::Command(task.map(Message::Power)),
            },
            Message::Audio(msg) => match self.audio.update(msg) {
                audio::Action::None => Action::None,
                audio::Action::ToggleSinksMenu => {
                    self.toggle_sub_menu(SubMenu::Sinks);
                    Action::None
                }
                audio::Action::ToggleSourcesMenu => {
                    self.toggle_sub_menu(SubMenu::Sources);
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
                audio::Action::Task(task) => Action::Command(task.map(Message::Audio)),
            },
            Message::Network(msg) => match self.network.update(msg) {
                network::Action::None => Action::None,
                network::Action::RequestPasswordForSSID(ssid) => {
                    self.network_dialog = Some(NetworkDialogState::new_password_dialog(ssid));
                    self.network_dialog_show_password = false;
                    Action::None
                }
                network::Action::RequestPassword(id, ssid) => {
                    self.network_dialog = Some(NetworkDialogState::new_password_dialog(ssid));
                    self.network_dialog_show_password = false;
                    Action::RequestKeyboard(id)
                }
                network::Action::ConfirmOpenNetwork(ssid) => {
                    self.network_dialog = Some(NetworkDialogState::new_warning_dialog(ssid));
                    self.network_dialog_show_password = false;
                    Action::None
                }
                network::Action::Command(task) => Action::Command(task.map(Message::Network)),
                network::Action::ToggleWifiMenu => {
                    self.toggle_sub_menu(SubMenu::Wifi);
                    Action::None
                }
                network::Action::ToggleVpnMenu => {
                    self.toggle_sub_menu(SubMenu::Vpn);
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
                    self.toggle_sub_menu(SubMenu::Bluetooth);
                    Action::None
                }
                bluetooth::Action::CloseSubMenu(task) => {
                    if self.sub_menu == Some(SubMenu::Bluetooth) {
                        self.sub_menu.take();
                    }

                    Action::Command(task.map(Message::Bluetooth))
                }
                bluetooth::Action::Command(task) => Action::Command(task.map(Message::Bluetooth)),
                bluetooth::Action::CloseMenu(id) => Action::CloseMenu(id),
            },
            Message::Brightness(msg) => match self.brightness.update(msg) {
                brightness::Action::None => Action::None,
                brightness::Action::Command(task) => Action::Command(task.map(Message::Brightness)),
            },
            Message::ToggleSubMenu(menu_type) => {
                let was_open = self.sub_menu == Some(menu_type);
                self.toggle_sub_menu(menu_type);

                if !was_open && menu_type == SubMenu::Wifi {
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
            Message::ToggleInhibitIdle => {
                if let Some(idle_inhibitor) = &mut self.idle_inhibitor {
                    idle_inhibitor.toggle();
                }
                Action::None
            }
            Message::Lock => {
                if let Some(lock_cmd) = &self.lock_cmd {
                    crate::utils::launcher::execute_command(lock_cmd);
                }
                Action::None
            }
            Message::PasswordDialog(msg) => match msg {
                password_dialog::Message::PasswordChanged(password) => {
                    if let Some(dialog) = &mut self.network_dialog {
                        dialog.password = Some(password);
                    }

                    Action::None
                }
                password_dialog::Message::TogglePasswordVisibility => {
                    self.network_dialog_show_password = !self.network_dialog_show_password;

                    Action::None
                }
                password_dialog::Message::DialogConfirmed(id) => {
                    let action = if let Some(dialog) = self.network_dialog.take() {
                        let message = match dialog.kind {
                            NetworkDialogKind::Password => {
                                network::Message::PasswordDialogConfirmed(
                                    dialog.ssid,
                                    dialog.password.unwrap_or_default(),
                                )
                            }
                            NetworkDialogKind::OpenNetworkWarning => {
                                network::Message::OpenNetworkDialogConfirmed(dialog.ssid)
                            }
                        };

                        match self.network.update(message) {
                            network::Action::Command(task) => {
                                Action::ReleaseKeyboardWithCommand(id, task.map(Message::Network))
                            }
                            _ => Action::ReleaseKeyboard(id),
                        }
                    } else {
                        Action::ReleaseKeyboard(id)
                    };
                    self.network_dialog_show_password = false;
                    action
                }
                password_dialog::Message::DialogCancelled(id) => {
                    self.network_dialog = None;
                    self.network_dialog_show_password = false;

                    Action::ReleaseKeyboard(id)
                }
            },
            Message::CustomButton(name) => {
                if let Some(button) = self.custom_buttons.iter().find(|b| b.name == name) {
                    crate::utils::launcher::execute_command(&button.command);

                    // Toggle button state immediately
                    let current_status = self.custom_buttons_status.get(&name).and_then(|v| *v);
                    self.custom_buttons_status
                        .insert(name, current_status.map(|s| !s));
                }
                Action::None
            }
            Message::CustomButtonsStatus(statuses) => {
                for (name, status) in statuses.into_iter() {
                    self.custom_buttons_status.insert(name, status);
                }
                Action::None
            }
            Message::MenuOpened => {
                self.sub_menu = if self.power.config.peripheral_expanded_by_default {
                    Some(SubMenu::PeripheralMenu)
                } else {
                    None
                };

                let buttons = self.custom_buttons.clone();

                let custom_buttons_task = if buttons.is_empty() {
                    Task::none()
                } else {
                    Task::perform(
                        async move {
                            let futures = buttons.into_iter().map(|button| async move {
                                if let Some(cmd) = button.status_command {
                                    let result = timeout(Duration::from_secs(1), async {
                                        let output = Command::new("bash")
                                            .arg("-c")
                                            .arg(cmd)
                                            .status()
                                            .await?;
                                        Ok::<_, std::io::Error>(output.success())
                                    })
                                    .await;
                                    match result {
                                        Ok(Ok(output)) => {
                                            debug!(
                                                "Custom button '{}' status_command executed with result: {}",
                                                button.name, output
                                            );
                                            (button.name, Some(output))
                                        }
                                        Ok(Err(e)) => {
                                            error!(
                                                "Failed to spawn status_command for custom button '{}': {}",
                                                button.name, e
                                            );
                                            (button.name, None)
                                        }
                                        Err(_) => {
                                            error!(
                                                "Custom button '{}' status_command timed out after 1000ms",
                                                button.name
                                            );
                                            (button.name, None)
                                        }
                                    }
                                } else {
                                    (button.name, Some(false))
                                }
                            });
                            join_all(futures).await
                        },
                        Message::CustomButtonsStatus,
                    )
                };

                self.brightness.update(brightness::Message::MenuOpened);

                Action::Command(custom_buttons_task)
            }
            Message::ConfigReloaded(config) => {
                self.enable_tooltips = config.enable_tooltips;
                self.lock_cmd = config.lock_cmd;
                self.power
                    .update(power::Message::ConfigReloaded(PowerSettingsConfig::new(
                        config.suspend_cmd,
                        config.hibernate_cmd,
                        config.reboot_cmd,
                        config.shutdown_cmd,
                        config.logout_cmd,
                        config.battery_format,
                        config.battery_hide_when_full,
                        config.peripheral_indicators,
                        config.peripheral_battery_format,
                        config.peripheral_expanded_by_default,
                    )));
                self.audio
                    .update(audio::Message::ConfigReloaded(AudioSettingsConfig::new(
                        config.audio_sinks_more_cmd,
                        config.audio_sources_more_cmd,
                        config.volume_step,
                        config.max_volume,
                        config.audio_indicator_format,
                        config.microphone_indicator_format,
                    )));
                self.network.update(network::Message::ConfigReloaded(
                    NetworkSettingsConfig::new(
                        config.wifi_more_cmd,
                        config.vpn_more_cmd,
                        config.remove_airplane_btn,
                        config.network_indicator_format,
                    ),
                ));
                self.bluetooth.update(bluetooth::Message::ConfigReloaded(
                    BluetoothSettingsConfig::new(
                        config.bluetooth_more_cmd,
                        config.bluetooth_indicator_format,
                    ),
                ));
                self.brightness.update(brightness::Message::ConfigReloaded(
                    config.brightness_indicator_format,
                ));
                if config.remove_idle_btn {
                    self.idle_inhibitor = None;
                } else if self.idle_inhibitor.is_none() {
                    self.idle_inhibitor = IdleInhibitorManager::new();
                }
                self.indicators = config.indicators;
                self.custom_buttons = config.custom_buttons;
                Action::None
            }
            Message::AudioTooltipHover(ui_ref, id) => {
                Action::OpenTooltipMenu(id, MenuType::AudioTooltip, ui_ref)
            }
            Message::BluetoothTooltipHover(ui_ref, id) => {
                Action::OpenTooltipMenu(id, MenuType::BluetoothTooltip, ui_ref)
            }
            Message::WifiTooltipHover(ui_ref, id) => {
                Action::OpenTooltipMenu(id, MenuType::WifiTooltip, ui_ref)
            }
            Message::VpnTooltipHover(ui_ref, id) => {
                Action::OpenTooltipMenu(id, MenuType::VpnTooltip, ui_ref)
            }
            Message::BatteryTooltipHover(ui_ref, id) => {
                Action::OpenTooltipMenu(id, MenuType::BatteryTooltip, ui_ref)
            }
            Message::PeripheralBatteryTooltipHover(ui_ref, id, index) => {
                Action::OpenTooltipMenu(id, MenuType::PeripheralBatteryTooltip(index), ui_ref)
            }
            Message::TooltipUnhover(id, menu_type) => Action::CloseTooltipMenu(id, menu_type),
        }
    }

    pub fn menu_view<'a>(&'a self, id: SurfaceId, position: Position) -> Element<'a, Message> {
        let space = use_theme(|t| t.space);
        container(if let Some(dialog) = &self.network_dialog {
            password_dialog::view(
                id,
                &dialog.ssid,
                dialog.password.as_deref().unwrap_or(""),
                self.network_dialog_show_password,
                matches!(dialog.kind, NetworkDialogKind::OpenNetworkWarning),
            )
            .map(Message::PasswordDialog)
        } else {
            let battery_data = self
                .power
                .battery_menu_indicator()
                .map(|e| e.map(Message::Power));
            let right_buttons = Row::with_capacity(2)
                .push(
                    self.lock_cmd
                        .as_ref()
                        .map(|_| icon_button(StaticIcon::Lock).on_press(Message::Lock)),
                )
                .push(
                    icon_button(if self.sub_menu == Some(SubMenu::Power) {
                        StaticIcon::Close
                    } else {
                        StaticIcon::Power
                    })
                    .on_press(Message::ToggleSubMenu(SubMenu::Power)),
                )
                .spacing(space.xs);

            let header = Row::with_capacity(3)
                .push(battery_data)
                .push(Space::new().width(Length::Fill))
                .push(right_buttons)
                .spacing(space.xs)
                .width(Length::Fill);

            let (sink_slider, source_slider) = self.audio.sliders(self.sub_menu);

            let wifi_setting_button = self
                .network
                .wifi_quick_setting_button(id, self.sub_menu)
                .map(|(button, submenu)| {
                    (
                        button.map(Message::Network),
                        submenu.map(|e| e.map(Message::Network)),
                    )
                });
            let quick_settings = quick_settings_section(
                vec![
                    wifi_setting_button,
                    self.bluetooth.quick_setting_button(id, self.sub_menu).map(
                        |(button, submenu)| {
                            (
                                button.map(Message::Bluetooth),
                                submenu.map(|e| e.map(Message::Bluetooth)),
                            )
                        },
                    ),
                    self.network
                        .vpn_quick_setting_button(id, self.sub_menu)
                        .map(|(button, submenu)| {
                            (
                                button.map(Message::Network),
                                submenu.map(|e| e.map(Message::Network)),
                            )
                        }),
                    self.network
                        .airplane_mode_quick_setting_button()
                        .map(|(button, _)| (button.map(Message::Network), None)),
                    self.idle_inhibitor.as_ref().map(|idle_inhibitor| {
                        (
                            quick_setting_button(
                                IdleInhibitorManager::idle_inhibitor_icon(
                                    idle_inhibitor.is_inhibited(),
                                ),
                                t!("settings-idle-inhibitor"),
                                None,
                                idle_inhibitor.is_inhibited(),
                                Message::ToggleInhibitIdle,
                                None,
                                None,
                            ),
                            None,
                        )
                    }),
                    self.power.quick_setting_button().map(|(button, submenu)| {
                        (
                            button.map(Message::Power),
                            submenu.map(|e| e.map(Message::Power)),
                        )
                    }),
                ]
                .into_iter()
                .flatten()
                .chain(self.custom_buttons.iter().map(|button| {
                    let is_active = self
                        .custom_buttons_status
                        .get(&button.name)
                        .and_then(|v| *v)
                        .unwrap_or(false);
                    (
                        quick_setting_button(
                            DynamicIcon(button.icon.clone()),
                            button.name.clone(),
                            button.tooltip.clone(),
                            is_active,
                            Message::CustomButton(button.name.clone()),
                            None,
                            None,
                        ),
                        None,
                    )
                }))
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

            Column::with_capacity(11)
                .push(header)
                .push(
                    self.sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::PeripheralMenu)
                        .and_then(|_| {
                            self.power
                                .peripheral_menu()
                                .map(|e| sub_menu_wrapper(e.map(Message::Power)))
                        }),
                )
                .push(
                    self.sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Power)
                        .map(|_| sub_menu_wrapper(self.power.menu().map(Message::Power))),
                )
                .push(top_sink_slider)
                .push(
                    self.sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Sinks)
                        .and_then(|_| {
                            self.audio
                                .sinks_submenu(id)
                                .map(|submenu| sub_menu_wrapper(submenu.map(Message::Audio)))
                        }),
                )
                .push(bottom_sink_slider)
                .push(top_source_slider)
                .push(
                    self.sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Sources)
                        .and_then(|_| {
                            self.audio
                                .sources_submenu(id)
                                .map(|submenu| sub_menu_wrapper(submenu.map(Message::Audio)))
                        }),
                )
                .push(bottom_source_slider)
                .push(self.brightness.slider().map(|e| e.map(Message::Brightness)))
                .push(quick_settings)
                .spacing(space.md)
                .into()
        })
        .width(MenuSize::Medium)
        .into()
    }

    pub fn view<'a>(&'a self, id: SurfaceId) -> Element<'a, Message> {
        let space = use_theme(|t| t.space);
        let mut row = Row::with_capacity(self.indicators.len());

        for indicator in &self.indicators {
            let element: Option<Element<'a, Message>> = match indicator {
                SettingsIndicator::IdleInhibitor => self
                    .idle_inhibitor
                    .as_ref()
                    .filter(|i| i.is_inhibited())
                    .map(|_| {
                        container(icon(IdleInhibitorManager::idle_inhibitor_icon(true)))
                            .style(|theme: &Theme| container::Style {
                                text_color: Some(theme.palette().danger),
                                ..Default::default()
                            })
                            .into()
                    }),
                SettingsIndicator::PowerProfile => self
                    .power
                    .power_profile_indicator()
                    .map(|e| e.map(Message::Power)),
                SettingsIndicator::Audio => {
                    self.audio.sink_indicator().map(|e| e.map(Message::Audio))
                }
                SettingsIndicator::Network => self
                    .network
                    .connection_indicator()
                    .map(|e| e.map(Message::Network)),
                SettingsIndicator::Vpn => self
                    .network
                    .vpn_indicator()
                    .map(|e| e.map(Message::Network)),
                SettingsIndicator::Bluetooth => self
                    .bluetooth
                    .bluetooth_indicator()
                    .map(|e| e.map(Message::Bluetooth)),
                SettingsIndicator::Microphone => {
                    self.audio.source_indicator().map(|e| e.map(Message::Audio))
                }
                SettingsIndicator::Battery => self
                    .power
                    .battery_indicator()
                    .map(|e| e.map(Message::Power)),
                SettingsIndicator::PeripheralBattery => {
                    let peripherals = self.power.peripheral_indicators();
                    for (index, element) in peripherals.into_iter().enumerate() {
                        let element = element.map(Message::Power);
                        if self.enable_tooltips {
                            row = row.push(
                                position_button(element)
                                    .width(Length::Shrink)
                                    .height(Length::Shrink)
                                    .padding(0)
                                    .style(transparent_button_style)
                                    .on_hover_with_position(move |ui_ref| {
                                        Message::PeripheralBatteryTooltipHover(ui_ref, id, index)
                                    })
                                    .on_unhover(Message::TooltipUnhover(
                                        id,
                                        MenuType::PeripheralBatteryTooltip(index),
                                    )),
                            );
                        } else {
                            row = row.push(element);
                        }
                    }
                    None
                }
                SettingsIndicator::Brightness => self
                    .brightness
                    .brightness_indicator()
                    .map(|e| e.map(Message::Brightness)),
            };

            if let Some(element) = element {
                let tooltip_config: Option<TooltipConfig> = if self.enable_tooltips {
                    match indicator {
                        SettingsIndicator::Audio | SettingsIndicator::Microphone => {
                            Some((Message::AudioTooltipHover, MenuType::AudioTooltip))
                        }
                        SettingsIndicator::Network => {
                            Some((Message::WifiTooltipHover, MenuType::WifiTooltip))
                        }
                        SettingsIndicator::Vpn => {
                            Some((Message::VpnTooltipHover, MenuType::VpnTooltip))
                        }
                        SettingsIndicator::Bluetooth => {
                            Some((Message::BluetoothTooltipHover, MenuType::BluetoothTooltip))
                        }
                        SettingsIndicator::Battery => {
                            Some((Message::BatteryTooltipHover, MenuType::BatteryTooltip))
                        }
                        _ => None,
                    }
                } else {
                    None
                };

                if let Some((hover_msg, menu_type)) = tooltip_config {
                    row = row.push(
                        position_button(element)
                            .width(Length::Shrink)
                            .height(Length::Shrink)
                            .padding(0)
                            .style(transparent_button_style)
                            .on_hover_with_position(move |ui_ref| hover_msg(ui_ref, id))
                            .on_unhover(Message::TooltipUnhover(id, menu_type)),
                    );
                } else {
                    row = row.push(element);
                }
            }
        }

        row.spacing(space.xs).into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            self.power.subscription().map(Message::Power),
            self.audio.subscription().map(Message::Audio),
            self.brightness.subscription().map(Message::Brightness),
            self.network.subscription().map(Message::Network),
            self.bluetooth.subscription().map(Message::Bluetooth),
        ])
    }

    pub fn tooltip_view<'a>(&'a self, menu_type: &MenuType) -> Element<'a, Message> {
        let space = use_theme(|t| t.space);
        match menu_type {
            MenuType::AudioTooltip => {
                let sink = self.audio.active_sink_label();
                let source = self.audio.active_source_label();
                let mut column = Column::new().spacing(space.xs);
                if let Some(sink) = sink {
                    column = column.push(
                        row![StaticIcon::Speaker3.to_text(), iced::widget::text(sink)]
                            .spacing(space.xs),
                    );
                }
                if let Some(source) = source {
                    column = column.push(
                        row![StaticIcon::Mic1.to_text(), iced::widget::text(source)]
                            .spacing(space.xs),
                    );
                }
                column.into()
            }
            MenuType::BluetoothTooltip => {
                let devices = self.bluetooth.connected_devices_for_tooltip();
                if !devices.is_empty() {
                    Column::with_capacity(devices.len())
                        .extend(devices.into_iter().map(|(name, battery)| {
                            if let Some(bat) = battery {
                                row![
                                    iced::widget::text(name),
                                    iced::widget::text(format!("{}%", bat))
                                ]
                                .spacing(space.xs)
                                .into()
                            } else {
                                row![iced::widget::text(name)].spacing(space.xs).into()
                            }
                        }))
                        .spacing(space.xs)
                        .into()
                } else {
                    Row::new().into()
                }
            }
            MenuType::WifiTooltip => {
                if let Some(label) = self.network.connected_wifi_label() {
                    row![StaticIcon::Wifi4.to_text(), iced::widget::text(label)]
                        .spacing(space.xs)
                        .into()
                } else {
                    Row::new().into()
                }
            }
            MenuType::VpnTooltip => {
                if let Some(label) = self.network.vpn_tooltip_label() {
                    row![StaticIcon::Vpn.to_text(), iced::widget::text(label)]
                        .spacing(space.xs)
                        .into()
                } else {
                    Row::new().into()
                }
            }
            MenuType::BatteryTooltip => {
                if let Some((capacity, status_label, details)) = self.power.battery_tooltip_info() {
                    let mut r = Row::new()
                        .push(StaticIcon::Battery4.to_text())
                        .push(iced::widget::text(format!("{capacity}%")))
                        .push(iced::widget::text(status_label))
                        .spacing(space.xs);
                    if !details.is_empty() {
                        r = r.push(iced::widget::text(details));
                    }
                    r.into()
                } else {
                    Row::new().into()
                }
            }
            MenuType::PeripheralBatteryTooltip(index) => {
                if let Some((name, capacity, device_icon)) =
                    self.power.peripheral_battery_tooltip_info(*index)
                {
                    row![
                        device_icon.to_text(),
                        iced::widget::text(name),
                        iced::widget::text(format!("{capacity}%"))
                    ]
                    .spacing(space.xs)
                    .into()
                } else {
                    Row::new().into()
                }
            }
            _ => Row::new().into(),
        }
    }
}

fn transparent_button_style(
    theme: &Theme,
    _status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: None,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.,
            radius: 0.into(),
        },
        shadow: Shadow {
            color: Color::TRANSPARENT,
            offset: Vector::default(),
            blur_radius: 0.,
        },
        text_color: theme.palette().text,
        snap: true,
    }
}

fn quick_settings_section<'a>(
    buttons: Vec<(Element<'a, Message>, Option<Element<'a, Message>>)>,
) -> Element<'a, Message> {
    let space = use_theme(|t| t.space);
    // TODO trying to read this function gives me a headache; there's surely
    // a better way to do this, maybe with Iterator::chunks or something?
    // I might be way off though, I still don't fully understand how this works.
    let mut section = Column::with_capacity(buttons.len() * 3).spacing(space.xs);

    let mut before: Option<(Element<'a, Message>, Option<Element<'a, Message>>)> = None;

    for (button, menu) in buttons.into_iter() {
        match before.take() {
            Some((before_button, before_menu)) => {
                section = section.push(
                    row![before_button, button]
                        .width(Length::Fill)
                        .spacing(space.xs),
                );

                if let Some(menu) = before_menu {
                    section = section.push(sub_menu_wrapper(menu));
                }

                if let Some(menu) = menu {
                    section = section.push(sub_menu_wrapper(menu));
                }
            }
            _ => {
                before = Some((button, menu));
            }
        }
    }

    if let Some((before_button, before_menu)) = before.take() {
        section = section.push(
            row![before_button, space::horizontal()]
                .width(Length::Fill)
                .spacing(space.xs),
        );

        if let Some(menu) = before_menu {
            section = section.push(sub_menu_wrapper(menu));
        }
    }

    section.into()
}
