use std::collections::HashMap;
use std::time::Duration;

use iced::futures::future::join_all;
use log::{debug, error, warn};
use tokio::process::Command;
use tokio::time::timeout;

use crate::{
    components::{
        ButtonUIRef, MenuSize, collapsible,
        icons::{DynamicIcon, Icon, StaticIcon, icon, icon_button},
        menu::MenuType,
        password_dialog, position_button, quick_setting_button, share_wifi_dialog,
        sub_menu_wrapper,
    },
    config::{ModuleName, Position, SettingsCustomButton, SettingsIndicator, SettingsModuleConfig},
    modules::settings::{
        audio::{AudioSettings, AudioSettingsConfig},
        bluetooth::{BluetoothSettings, BluetoothSettingsConfig},
        brightness::BrightnessSettings,
        network::{NetworkSettings, NetworkSettingsConfig},
        power::{PowerSettings, PowerSettingsConfig},
    },
    services::{
        ReadOnlyService, Service, ServiceEvent,
        airplane::{self, AirplaneCommand, AirplaneService},
        idle_inhibitor::{self, IdleInhibitor},
        network::{PasswordReply, PasswordRequestId, Redacted},
    },
    theme::{transparent_button_style, use_theme},
};
use ashell_i18n::t;
use iced::{
    Element, Length, Subscription, SurfaceId, Task, Theme,
    widget::{Column, Row, Space, container, image, row, space},
};

pub(crate) mod audio;
mod bluetooth;
pub(crate) mod brightness;
pub(crate) mod network;
mod power;

type TooltipConfig = (fn(ButtonUIRef, SurfaceId) -> Message, MenuType);
type QuickSettingEntry<'a> = (Element<'a, Message>, Option<(bool, Element<'a, Message>)>);

pub struct Settings {
    lock_cmd: Option<String>,
    power: PowerSettings,
    audio: AudioSettings,
    brightness: BrightnessSettings,
    network: NetworkSettings,
    bluetooth: BluetoothSettings,
    airplane: Option<AirplaneService>,
    remove_airplane_btn: bool,
    idle_inhibitor: Option<IdleInhibitor>,
    remove_dnd_btn: bool,
    sub_menu: Option<SubMenu>,
    network_dialog: Option<NetworkDialogState>,
    network_dialog_show_password: bool,
    indicators: Vec<SettingsIndicator>,
    custom_buttons: Vec<SettingsCustomButton>,
    custom_buttons_status: HashMap<String, Option<bool>>,
    enable_tooltips: bool,
}

#[derive(Debug, Clone)]
enum NetworkDialogState {
    Password {
        ssid: String,
        password: String,
        /// The request of NetworkManager or iwd it answers, if any.
        request: Option<PasswordRequestId>,
    },
    OpenNetworkWarning {
        ssid: String,
        connect_once: bool,
    },
    ShareWifi {
        ssid: String,
        password: Option<String>,
        error: Option<String>,
        qr: Option<(image::Handle, u32)>,
    },
}

fn focus_password_input() -> Task<Message> {
    iced::widget::focus::<Message>(password_dialog::PASSWORD_INPUT_ID).into()
}

#[derive(Debug, Clone)]
pub enum Message {
    Network(network::Message),
    Bluetooth(bluetooth::Message),
    Audio(audio::Message),
    Brightness(brightness::Message),
    Airplane(ServiceEvent<AirplaneService>),
    ToggleAirplaneMode,
    ToggleInhibitIdle,
    ToggleDnd,
    Lock(SurfaceId),
    Power(power::Message),
    ToggleSubMenu(SubMenu),
    PasswordDialog(password_dialog::Message),
    FocusPasswordInput,
    ShareDialog(share_wifi_dialog::Message),
    CustomButton(String),
    CustomButtonsStatus(Vec<(String, Option<bool>)>),
    MenuOpened,
    ConfigReloaded(Box<SettingsModuleConfig>),
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
    RequestKeyboardWithCommand(SurfaceId, Task<Message>),
    /// Run `sync_password_request`.
    SyncPasswordRequest,
    /// The password dialog closed, with its task.
    PasswordDialogClosed(Task<Message>),
    OpenTooltipMenu(SurfaceId, MenuType, ButtonUIRef),
    CloseTooltipMenu(SurfaceId, MenuType),
    ToggleDnd,
}

/// What `Settings::sync_password_request` did to the dialog.
pub enum PasswordDialog {
    /// It shows a password request now.
    Shown,
    /// The request it showed is no longer pending, and nothing took its place.
    Closed,
    Unchanged,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum SubMenu {
    BatteryMenu,
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

    pub fn idle_inhibitor(&self) -> &Option<IdleInhibitor> {
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

    pub fn set_mute(&mut self, muted: bool) -> Action {
        self.audio.set_sink_mute(muted);
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

    pub fn set_microphone_mute(&mut self, muted: bool) -> Action {
        self.audio.set_source_mute(muted);
        Action::None
    }

    pub fn brightness_adjust(&mut self, up: bool) -> Action {
        match self.brightness.brightness_adjust(up) {
            brightness::Action::Command(task) => Action::Command(task.map(Message::Brightness)),
            brightness::Action::None => Action::None,
        }
    }

    pub fn is_airplane_mode(&self) -> bool {
        self.airplane
            .as_ref()
            .is_some_and(|service| service.is_on())
    }

    /// Whether airplane mode is on after `command`, `None` when it can't
    /// change anything.
    pub fn airplane_after(&self, command: AirplaneCommand) -> Option<bool> {
        self.airplane
            .as_ref()
            .and_then(|service| service.after(command))
    }

    pub fn toggle_airplane(&mut self) -> Action {
        self.airplane_command(AirplaneCommand::Toggle)
    }

    pub fn set_airplane(&mut self, enabled: bool) -> Action {
        self.airplane_command(AirplaneCommand::Set(enabled))
    }

    /// Resolved against the state the OSD reads, so a quick repeat does what
    /// it shows.
    fn airplane_command(&mut self, command: AirplaneCommand) -> Action {
        let Some(service) = self.airplane.as_mut() else {
            return Action::None;
        };
        match service.target(command) {
            Some(enable) => Action::Command(
                service
                    .command(AirplaneCommand::Set(enable))
                    .map(Message::Airplane),
            ),
            None => Action::None,
        }
    }

    pub fn toggle_idle_inhibitor(&mut self) -> Action {
        if let Some(idle_inhibitor) = &mut self.idle_inhibitor
            && let Err(err) = idle_inhibitor.toggle()
        {
            warn!("Failed to toggle idle inhibitor: {err}");
        }
        Action::None
    }

    pub fn set_idle_inhibitor(&mut self, enabled: bool) -> Action {
        if let Some(idle_inhibitor) = &mut self.idle_inhibitor
            && let Err(err) = idle_inhibitor.set(enabled)
        {
            warn!("Failed to set idle inhibitor: {err}");
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
                config.battery_health,
                config.keyboard_backlight_slider,
            )),
            audio: AudioSettings::new(AudioSettingsConfig::new(
                config.audio_sinks_more_cmd,
                config.audio_sources_more_cmd,
                config.audio_sink_post_switch_cmd,
                config.audio_source_post_switch_cmd,
                config.volume_step,
                config.max_volume,
                config.audio_indicator_format,
                config.microphone_indicator_format,
            )),
            brightness: BrightnessSettings::new(config.brightness_indicator_format),
            network: NetworkSettings::new(NetworkSettingsConfig::new(
                config.wifi_more_cmd,
                config.vpn_more_cmd,
                config.network_indicator_format,
                config.network_backend,
            )),
            bluetooth: BluetoothSettings::new(BluetoothSettingsConfig::new(
                config.bluetooth_more_cmd,
                config.bluetooth_indicator_format,
            )),
            airplane: None,
            remove_airplane_btn: config.remove_airplane_btn,
            idle_inhibitor: if config.remove_idle_btn {
                None
            } else {
                idle_inhibitor::connect()
            },
            remove_dnd_btn: config.remove_dnd_btn,
            sub_menu: None,
            network_dialog: None,
            indicators: config.indicators,
            custom_buttons: config.custom_buttons,
            custom_buttons_status: HashMap::new(),
            network_dialog_show_password: false,
            enable_tooltips: config.enable_tooltips,
        }
    }

    /// The Collapsible owns the close animation and the surrounding spacing,
    /// so this is just a state flip.
    fn close_submenu(&mut self) {
        self.sub_menu = None;
    }

    fn close_network_submenu(&mut self) {
        if matches!(self.sub_menu, Some(SubMenu::Wifi | SubMenu::Vpn)) {
            self.close_submenu();
        }
    }

    /// Closes the dialog of a password request no longer pending, and shows the
    /// oldest pending one when no other dialog is open. Without a settings menu
    /// open, a dialog left from the last one doesn't count.
    pub fn sync_password_request(&mut self, menu_open: bool) -> PasswordDialog {
        let request_closed = matches!(
            &self.network_dialog,
            Some(NetworkDialogState::Password { request: Some(id), .. })
                if !self.network.is_password_pending(*id)
        );
        let left_over =
            !menu_open && self.network_dialog.is_some() && !self.shows_password_request();
        if request_closed || left_over {
            self.network_dialog = None;
            self.network_dialog_show_password = false;
        }

        if self.network_dialog.is_none()
            && let Some(request) = self.network.password_request()
        {
            self.network_dialog = Some(NetworkDialogState::Password {
                ssid: request.ssid.clone(),
                password: String::new(),
                request: Some(request.id),
            });
            self.network_dialog_show_password = false;
            PasswordDialog::Shown
        } else if request_closed {
            PasswordDialog::Closed
        } else {
            PasswordDialog::Unchanged
        }
    }

    /// Leaves the pending password requests to other agents: nothing can show
    /// them.
    pub fn decline_password_requests(&mut self) {
        self.network.decline_passwords();
        if self.shows_password_request() {
            self.network_dialog = None;
        }
    }

    /// Whether the dialog shown answers a password request.
    pub fn shows_password_request(&self) -> bool {
        matches!(
            self.network_dialog,
            Some(NetworkDialogState::Password {
                request: Some(_),
                ..
            })
        )
    }

    fn toggle_submenu_to(&mut self, target: SubMenu) {
        if self.sub_menu == Some(target) {
            self.sub_menu = None;
        } else {
            self.sub_menu = Some(target);
        }
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::Power(msg) => match self.power.update(msg) {
                power::Action::None => Action::None,
                power::Action::ToggleBatteryMenu => {
                    self.toggle_submenu_to(SubMenu::BatteryMenu);
                    Action::None
                }
                power::Action::Command(task) => Action::Command(task.map(Message::Power)),
                power::Action::CloseMenu(id) => Action::CloseMenu(id),
            },
            Message::Audio(msg) => match self.audio.update(msg) {
                audio::Action::None => Action::None,
                audio::Action::ToggleSinksMenu => {
                    self.toggle_submenu_to(SubMenu::Sinks);
                    Action::None
                }
                audio::Action::ToggleSourcesMenu => {
                    self.toggle_submenu_to(SubMenu::Sources);
                    Action::None
                }
                audio::Action::CloseSubMenu => {
                    if self.sub_menu == Some(SubMenu::Sinks)
                        || self.sub_menu == Some(SubMenu::Sources)
                    {
                        self.close_submenu();
                    }
                    Action::None
                }
                audio::Action::CloseMenu(id) => Action::CloseMenu(id),
                audio::Action::Task(task) => Action::Command(task.map(Message::Audio)),
            },
            Message::Network(msg) => match self.network.update(msg) {
                network::Action::None => Action::None,
                network::Action::RequestPassword(id, ssid) => {
                    self.network_dialog = Some(NetworkDialogState::Password {
                        ssid,
                        password: String::new(),
                        request: None,
                    });
                    self.network_dialog_show_password = false;
                    Action::RequestKeyboardWithCommand(id, focus_password_input())
                }
                network::Action::PasswordRequestsChanged => Action::SyncPasswordRequest,
                network::Action::ConfirmOpenNetwork(ssid) => {
                    self.network_dialog = Some(NetworkDialogState::OpenNetworkWarning {
                        ssid,
                        connect_once: false,
                    });
                    self.network_dialog_show_password = false;
                    Action::None
                }
                network::Action::OpenShareWifiDialog { ssid, outcome } => {
                    let (password, error, qr_payload) =
                        share_wifi_dialog::materialize(&ssid, outcome);
                    self.network_dialog = Some(NetworkDialogState::ShareWifi {
                        qr: qr_payload
                            .as_deref()
                            .and_then(share_wifi_dialog::qr_image_handle),
                        ssid,
                        password,
                        error,
                    });
                    Action::None
                }
                network::Action::Command(task) => Action::Command(task.map(Message::Network)),
                network::Action::ToggleWifiMenu => {
                    self.toggle_submenu_to(SubMenu::Wifi);
                    Action::None
                }
                network::Action::ToggleVpnMenu => {
                    self.toggle_submenu_to(SubMenu::Vpn);
                    Action::None
                }
                network::Action::CloseSubMenu(task) => {
                    self.close_network_submenu();
                    Action::Command(task.map(Message::Network))
                }
                network::Action::CloseMenu(id) => Action::CloseMenu(id),
            },
            Message::Bluetooth(msg) => match self.bluetooth.update(msg) {
                bluetooth::Action::None => Action::None,
                bluetooth::Action::ToggleBluetoothMenu => {
                    self.toggle_submenu_to(SubMenu::Bluetooth);
                    Action::None
                }
                bluetooth::Action::CloseSubMenu(task) => {
                    if self.sub_menu == Some(SubMenu::Bluetooth) {
                        self.close_submenu();
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
                self.toggle_submenu_to(menu_type);
                if self.sub_menu == Some(menu_type) && menu_type == SubMenu::Wifi {
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
            Message::Airplane(event) => {
                match event {
                    ServiceEvent::Init(service) => self.airplane = Some(service),
                    ServiceEvent::Update(data) => {
                        if let Some(service) = self.airplane.as_mut() {
                            service.update(data);
                        }
                    }
                    ServiceEvent::Error(_) => {}
                }
                Action::None
            }
            Message::ToggleAirplaneMode => {
                self.close_network_submenu();
                self.toggle_airplane()
            }
            Message::ToggleInhibitIdle => self.toggle_idle_inhibitor(),
            Message::ToggleDnd => Action::ToggleDnd,
            Message::Lock(id) => {
                if let Some(lock_cmd) = &self.lock_cmd {
                    crate::utils::launcher::execute_command(lock_cmd);
                }
                Action::CloseMenu(id)
            }
            Message::PasswordDialog(msg) => match msg {
                password_dialog::Message::PasswordChanged(password) => {
                    if let Some(NetworkDialogState::Password { password: pw, .. }) =
                        self.network_dialog.as_mut()
                    {
                        *pw = password;
                    }

                    Action::None
                }
                password_dialog::Message::TogglePasswordVisibility => {
                    self.network_dialog_show_password = !self.network_dialog_show_password;

                    Action::None
                }
                password_dialog::Message::ConnectOnceToggled(connect_once) => {
                    if let Some(NetworkDialogState::OpenNetworkWarning {
                        connect_once: value,
                        ..
                    }) = self.network_dialog.as_mut()
                    {
                        *value = connect_once;
                    }

                    Action::None
                }
                password_dialog::Message::DialogConfirmed => {
                    let message = match self.network_dialog.take() {
                        Some(NetworkDialogState::Password {
                            password,
                            request: Some(request),
                            ..
                        }) => {
                            self.network.reply_password(
                                request,
                                PasswordReply::Password(Redacted::new(password)),
                            );
                            None
                        }
                        Some(NetworkDialogState::Password {
                            ssid,
                            password,
                            request: None,
                        }) => Some(network::Message::PasswordDialogConfirmed(ssid, password)),
                        Some(NetworkDialogState::OpenNetworkWarning { ssid, connect_once }) => {
                            let connect_once = connect_once && self.network.can_connect_once(&ssid);
                            Some(network::Message::OpenNetworkDialogConfirmed(
                                ssid,
                                connect_once,
                            ))
                        }
                        Some(NetworkDialogState::ShareWifi { .. }) => {
                            warn!("Ignoring password confirmation from the share Wi-Fi dialog");
                            None
                        }
                        None => None,
                    };

                    let task = match message.map(|message| self.network.update(message)) {
                        Some(network::Action::Command(task)) => task.map(Message::Network),
                        _ => Task::none(),
                    };
                    self.network_dialog_show_password = false;
                    Action::PasswordDialogClosed(task)
                }
                password_dialog::Message::DialogCancelled => {
                    if let Some(NetworkDialogState::Password {
                        request: Some(request),
                        ..
                    }) = self.network_dialog.take()
                    {
                        self.network.reply_password(request, PasswordReply::Cancel);
                    }
                    self.network_dialog_show_password = false;

                    Action::PasswordDialogClosed(Task::none())
                }
            },
            Message::FocusPasswordInput => Action::Command(focus_password_input()),
            Message::ShareDialog(msg) => match msg {
                share_wifi_dialog::Message::Close => {
                    self.network_dialog = None;
                    Action::SyncPasswordRequest
                }
            },
            Message::CustomButton(name) => {
                if let Some(button) = self.custom_buttons.iter().find(|b| b.name == name) {
                    crate::utils::launcher::execute_command(&button.command);

                    let current_status = self.custom_buttons_status.get(&name).and_then(|v| *v);
                    self.custom_buttons_status
                        .insert(name, current_status.map(|s| !s));
                }
                Action::None
            }
            Message::CustomButtonsStatus(statuses) => {
                for (name, status) in statuses {
                    self.custom_buttons_status.insert(name, status);
                }
                Action::None
            }
            Message::MenuOpened => {
                self.network_dialog = None;
                self.network_dialog_show_password = false;
                self.sync_password_request(true);

                self.sub_menu = if self.power.config.peripheral_expanded_by_default {
                    Some(SubMenu::BatteryMenu)
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
                                    (button.name, None)
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
                        config.battery_health,
                        config.keyboard_backlight_slider,
                    )));
                self.audio
                    .update(audio::Message::ConfigReloaded(AudioSettingsConfig::new(
                        config.audio_sinks_more_cmd,
                        config.audio_sources_more_cmd,
                        config.audio_sink_post_switch_cmd,
                        config.audio_source_post_switch_cmd,
                        config.volume_step,
                        config.max_volume,
                        config.audio_indicator_format,
                        config.microphone_indicator_format,
                    )));
                self.network.update(network::Message::ConfigReloaded(
                    NetworkSettingsConfig::new(
                        config.wifi_more_cmd,
                        config.vpn_more_cmd,
                        config.network_indicator_format,
                        config.network_backend,
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
                    self.idle_inhibitor = idle_inhibitor::connect();
                }
                self.remove_airplane_btn = config.remove_airplane_btn;
                self.remove_dnd_btn = config.remove_dnd_btn;
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

    pub fn menu_view<'a>(
        &'a self,
        id: SurfaceId,
        position: Position,
        dnd: Option<bool>,
    ) -> Element<'a, Message> {
        let space = use_theme(|t| t.space);
        container(if let Some(dialog) = &self.network_dialog {
            match dialog {
                NetworkDialogState::Password { ssid, password, .. } => password_dialog::view(
                    ssid,
                    password,
                    self.network_dialog_show_password,
                    false,
                    false,
                    false,
                )
                .map(Message::PasswordDialog),
                NetworkDialogState::OpenNetworkWarning { ssid, connect_once } => {
                    password_dialog::view(
                        ssid,
                        "",
                        self.network_dialog_show_password,
                        true,
                        self.network.can_connect_once(ssid),
                        *connect_once,
                    )
                    .map(Message::PasswordDialog)
                }
                NetworkDialogState::ShareWifi {
                    ssid,
                    password,
                    error,
                    qr,
                } => share_wifi_dialog::view(ssid, password.clone(), error.clone(), qr.clone())
                    .map(Message::ShareDialog),
            }
        } else {
            let battery_data = self
                .power
                .battery_menu_indicator()
                .map(|e| e.map(Message::Power));
            let right_buttons = Row::with_capacity(2)
                .push(
                    self.lock_cmd
                        .as_ref()
                        .map(|_| icon_button(StaticIcon::Lock).on_press(Message::Lock(id))),
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
                        submenu.map(|(expanded, e)| (expanded, e.map(Message::Network))),
                    )
                });
            let quick_settings = quick_settings_section(
                vec![
                    wifi_setting_button,
                    self.bluetooth.quick_setting_button(id, self.sub_menu).map(
                        |(button, submenu)| {
                            (
                                button.map(Message::Bluetooth),
                                submenu.map(|(expanded, e)| (expanded, e.map(Message::Bluetooth))),
                            )
                        },
                    ),
                    self.network
                        .vpn_quick_setting_button(id, self.sub_menu)
                        .map(|(button, submenu)| {
                            (
                                button.map(Message::Network),
                                submenu.map(|(expanded, e)| (expanded, e.map(Message::Network))),
                            )
                        }),
                    self.airplane
                        .as_ref()
                        .filter(|service| !self.remove_airplane_btn && service.can_change())
                        .map(|service| {
                            (
                                quick_setting_button(
                                    airplane::icon(service.is_on()),
                                    t!("settings-network-airplane-mode"),
                                    None,
                                    service.is_on(),
                                    Message::ToggleAirplaneMode,
                                    Some(Message::Network(network::Message::OpenMore)),
                                    None,
                                    None,
                                ),
                                None,
                            )
                        }),
                    self.idle_inhibitor.as_ref().map(|idle_inhibitor| {
                        (
                            quick_setting_button(
                                idle_inhibitor::icon(idle_inhibitor.is_inhibited()),
                                t!("settings-idle-inhibitor"),
                                None,
                                idle_inhibitor.is_inhibited(),
                                Message::ToggleInhibitIdle,
                                None,
                                None,
                                None,
                            ),
                            None,
                        )
                    }),
                    dnd.filter(|_| !self.remove_dnd_btn).map(|active| {
                        (
                            quick_setting_button(
                                if active {
                                    StaticIcon::BellOff
                                } else {
                                    StaticIcon::Bell
                                },
                                t!("settings-dnd"),
                                None,
                                active,
                                Message::ToggleDnd,
                                None,
                                None,
                                None,
                            ),
                            None,
                        )
                    }),
                    self.power.quick_setting_button().map(|(button, submenu)| {
                        (
                            button.map(Message::Power),
                            submenu.map(|(expanded, e)| (expanded, e.map(Message::Power))),
                        )
                    }),
                    self.power
                        .charge_limit_quick_setting_button()
                        .map(|(button, _)| (button.map(Message::Power), None)),
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
                            None,
                        ),
                        None,
                    )
                }))
                .collect::<Vec<_>>(),
            );

            // The Collapsible bakes the inter-row gap into its animated height, so
            // spacing applies between groups only — no jump when one collapses.
            let animated = use_theme(|t| t.animations_enabled);
            let header_group = {
                let mut col = Column::<'a, Message>::with_capacity(3).spacing(0);
                col = col.push(header);
                if let Some(e) = self.power.battery_menu() {
                    col = col.push(
                        collapsible(
                            self.sub_menu == Some(SubMenu::BatteryMenu),
                            sub_menu_wrapper(e.map(Message::Power)),
                        )
                        .animated(animated)
                        .open_padding_top(space.md),
                    );
                }
                col = col.push(
                    collapsible(
                        self.sub_menu == Some(SubMenu::Power),
                        sub_menu_wrapper(self.power.menu(id).map(Message::Power)),
                    )
                    .animated(animated)
                    .open_padding_top(space.md),
                );
                col
            };

            let audio_sinks_group = sink_slider.map(|slider| {
                audio_group(
                    position,
                    slider.map(Message::Audio),
                    self.audio.sinks_submenu(id).map(|submenu| {
                        (
                            self.sub_menu == Some(SubMenu::Sinks),
                            sub_menu_wrapper(submenu.map(Message::Audio)),
                        )
                    }),
                )
            });

            let audio_sources_group = source_slider.map(|slider| {
                audio_group(
                    position,
                    slider.map(Message::Audio),
                    self.audio.sources_submenu(id).map(|submenu| {
                        (
                            self.sub_menu == Some(SubMenu::Sources),
                            sub_menu_wrapper(submenu.map(Message::Audio)),
                        )
                    }),
                )
            });

            Column::with_capacity(5)
                .push(header_group)
                .push(audio_sinks_group)
                .push(audio_sources_group)
                .push(self.brightness.slider().map(|e| e.map(Message::Brightness)))
                .push(
                    self.power
                        .kbd_backlight_slider()
                        .map(|e| e.map(Message::Power)),
                )
                .push(quick_settings)
                .spacing(space.md)
                .into()
        })
        .width(MenuSize::Medium)
        .into()
    }

    /// Size of the icons and texts of the bar indicators.
    pub fn indicator_size() -> f32 {
        use_theme(|t| t.module_font_size(&ModuleName::Settings, t.font_size.md))
    }

    pub fn view<'a>(&'a self, id: SurfaceId) -> Element<'a, Message> {
        let spacing = use_theme(|t| t.module_spacing(&ModuleName::Settings, t.space.xs));
        let mut row = Row::with_capacity(self.indicators.len());

        for indicator in &self.indicators {
            let element: Option<Element<'a, Message>> = match indicator {
                SettingsIndicator::IdleInhibitor => self
                    .idle_inhibitor
                    .as_ref()
                    .filter(|i| i.is_inhibited())
                    .map(|_| {
                        container(icon(idle_inhibitor::icon(true)).size(Self::indicator_size()))
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
                    .connection_indicator(self.is_airplane_mode())
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

        row.spacing(spacing).into()
    }

    /// `in_bar` when the module is in the bar, so its menu can open.
    pub fn subscription(&self, in_bar: bool) -> Subscription<Message> {
        Subscription::batch([
            self.power.subscription().map(Message::Power),
            self.audio.subscription().map(Message::Audio),
            self.brightness.subscription().map(Message::Brightness),
            self.network.subscription(in_bar).map(Message::Network),
            self.bluetooth.subscription().map(Message::Bluetooth),
            AirplaneService::subscribe().map(Message::Airplane),
        ])
    }

    pub fn tooltip_view<'a>(&'a self, menu_type: &MenuType) -> Element<'a, Message> {
        let (space, font_size) = use_theme(|t| (t.space, t.font_size));
        let fallback = |label: String| -> Element<'a, Message> { iced::widget::text(label).into() };
        match menu_type {
            MenuType::AudioTooltip => {
                let sink = self.audio.active_sink_label();
                let source = self.audio.active_source_label();
                if sink.is_none() && source.is_none() {
                    return fallback(t!("settings-tooltip-empty-audio"));
                }
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
                                    iced::widget::text(format!("{bat}%"))
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
                    fallback(t!("settings-tooltip-empty-bluetooth"))
                }
            }
            MenuType::WifiTooltip => {
                if let Some(label) = self.network.connected_wifi_label() {
                    let band = self.network.connected_wifi_band();
                    let mut r = row![StaticIcon::Wifi4.to_text(), iced::widget::text(label)]
                        .spacing(space.xs);
                    if let Some(b) = band {
                        r = r.push(iced::widget::text(b).size(font_size.xs).style(
                            |theme: &Theme| iced::widget::text::Style {
                                color: Some(theme.palette().primary),
                            },
                        ));
                    }
                    r.into()
                } else {
                    fallback(t!("settings-tooltip-empty-wifi"))
                }
            }
            MenuType::VpnTooltip => {
                if let Some(label) = self.network.vpn_tooltip_label() {
                    row![StaticIcon::Vpn.to_text(), iced::widget::text(label)]
                        .spacing(space.xs)
                        .into()
                } else {
                    fallback(t!("settings-tooltip-empty-vpn"))
                }
            }
            MenuType::BatteryTooltip => {
                if let Some(power::BatteryTooltipInfo {
                    capacity,
                    status_label,
                    details,
                    health_lines,
                }) = self.power.battery_tooltip_info()
                {
                    let mut r = Row::new()
                        .push(StaticIcon::Battery4.to_text())
                        .push(iced::widget::text(format!("{capacity}%")))
                        .push(iced::widget::text(status_label))
                        .spacing(space.xs);
                    if !details.is_empty() {
                        r = r.push(iced::widget::text(details));
                    }
                    Column::new()
                        .push(r)
                        .extend(health_lines.into_iter().map(|(icon_kind, line)| {
                            row![icon_kind.to_text(), iced::widget::text(line)]
                                .spacing(space.xs)
                                .into()
                        }))
                        .spacing(space.xs)
                        .into()
                } else {
                    fallback(t!("settings-tooltip-empty-battery"))
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
                    fallback(t!("settings-tooltip-empty-battery"))
                }
            }
            _ => fallback(t!("settings-tooltip-empty")),
        }
    }
}

/// Slider + optional sub-menu as one group. Sub-menu sits below (Top bar) or
/// above (Bottom bar); the Collapsible's animated padding holds the inter-row gap.
fn audio_group<'a>(
    position: Position,
    slider: Element<'a, Message>,
    submenu: Option<(bool, Element<'a, Message>)>,
) -> Element<'a, Message> {
    let pad = use_theme(|t| t.space.md);
    let mut col = Column::<'a, Message>::with_capacity(2).spacing(0);
    match position {
        Position::Top => {
            col = col.push(slider);
            if let Some((expanded, content)) = submenu {
                col = col.push(collapsible(expanded, content).open_padding_top(pad));
            }
        }
        Position::Bottom => {
            if let Some((expanded, content)) = submenu {
                col = col.push(collapsible(expanded, content).open_padding_bottom(pad));
            }
            col = col.push(slider);
        }
    }
    col.into()
}

fn quick_settings_section<'a>(buttons: Vec<QuickSettingEntry<'a>>) -> Element<'a, Message> {
    let (space, animated) = use_theme(|t| (t.space, t.animations_enabled));
    let mut section = Column::with_capacity(buttons.len().div_ceil(2)).spacing(space.xs);

    let mut buttons_iter = buttons.into_iter();
    while let Some((left_button, left_menu)) = buttons_iter.next() {
        let mut group = Column::<'a, Message>::with_capacity(3).spacing(0);

        let (button_row, right_menu): (Element<'a, Message>, _) =
            if let Some((right_button, right_menu)) = buttons_iter.next() {
                (
                    row![left_button, right_button]
                        .width(Length::Fill)
                        .spacing(space.xs)
                        .into(),
                    right_menu,
                )
            } else {
                (
                    row![left_button, space::horizontal()]
                        .width(Length::Fill)
                        .spacing(space.xs)
                        .into(),
                    None,
                )
            };

        group = group.push(button_row);

        if let Some((expanded, content)) = left_menu {
            group = group.push(
                collapsible(expanded, sub_menu_wrapper(content))
                    .animated(animated)
                    .open_padding_top(space.xs),
            );
        }
        if let Some((expanded, content)) = right_menu {
            group = group.push(
                collapsible(expanded, sub_menu_wrapper(content))
                    .animated(animated)
                    .open_padding_top(space.xs),
            );
        }

        section = section.push(group);
    }

    section.into()
}
