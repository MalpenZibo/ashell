use crate::{
    HEIGHT,
    components::menu::MenuType,
    components::{ButtonUIRef, Centerbox},
    config::{self, AppearanceStyle, Config, Modules, Position},
    get_log_spec,
    modules::{
        self,
        custom_module::{self, Custom},
        keyboard_layout::KeyboardLayout,
        keyboard_submap::KeyboardSubmap,
        media_player::MediaPlayer,
        notifications::Notifications,
        privacy::Privacy,
        settings::Settings,
        system_info::SystemInfo,
        tempo::Tempo,
        tray::TrayModule,
        updates::Updates,
        window_title::WindowTitle,
        workspaces::Workspaces,
    },
    outputs::{HasOutput, Outputs},
    services::ReadOnlyService,
    theme::{AshellTheme, backdrop_color, darken_color},
};
use flexi_logger::LoggerHandle;
use iced::{
    Alignment, Color, Element, Gradient, Length, OutputEvent, Radians, Subscription, SurfaceId,
    Task, Theme,
    event::listen_with,
    gradient::Linear,
    keyboard, set_exclusive_zone,
    widget::{Row, container, mouse_area},
};
use log::{debug, info, warn};
use std::{collections::HashMap, f32::consts::PI, path::PathBuf};

pub struct GeneralConfig {
    outputs: config::Outputs,
    pub modules: Modules,
    pub layer: config::Layer,
    enable_esc_key: bool,
}

pub struct App {
    config_path: PathBuf,
    pub theme: AshellTheme,
    logger: LoggerHandle,
    pub general_config: GeneralConfig,
    pub outputs: Outputs,
    pub custom: HashMap<String, Custom>,
    pub updates: Option<Updates>,
    pub workspaces: Workspaces,
    pub window_title: WindowTitle,
    pub system_info: SystemInfo,
    pub keyboard_layout: KeyboardLayout,
    pub keyboard_submap: KeyboardSubmap,
    pub tray: TrayModule,
    pub tempo: Tempo,
    pub privacy: Privacy,
    pub settings: Settings,
    pub media_player: MediaPlayer,
    pub notifications: Notifications,
    pub visible: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    ConfigChanged(Box<Config>),
    ToggleMenu(MenuType, SurfaceId, ButtonUIRef),
    CloseMenu(SurfaceId),
    Custom(String, custom_module::Message),
    Updates(modules::updates::Message),
    Workspaces(modules::workspaces::Message),
    WindowTitle(modules::window_title::Message),
    SystemInfo(modules::system_info::Message),
    KeyboardLayout(modules::keyboard_layout::Message),
    KeyboardSubmap(modules::keyboard_submap::Message),
    Tray(modules::tray::Message),
    Tempo(modules::tempo::Message),
    Privacy(modules::privacy::Message),
    Settings(modules::settings::Message),
    MediaPlayer(modules::media_player::Message),
    Notifications(modules::notifications::Message),
    OutputEvent(OutputEvent),
    CloseAllMenus,
    ResumeFromSleep,
    None,
    ToggleVisibility,
}

impl App {
    pub fn new(
        (logger, config, config_path): (LoggerHandle, Config, PathBuf),
    ) -> impl FnOnce() -> (Self, Task<Message>) {
        move || {
            let (outputs, task) = Outputs::new(
                config.appearance.style,
                config.position,
                config.layer,
                config.appearance.scale_factor,
            );

            let custom = config
                .custom_modules
                .clone()
                .into_iter()
                .map(|o| (o.name.clone(), Custom::new(o)))
                .collect();

            let notifications = Notifications::new(config.notifications);

            (
                App {
                    config_path,
                    theme: AshellTheme::new(config.position, &config.appearance),
                    logger,
                    general_config: GeneralConfig {
                        outputs: config.outputs,
                        modules: config.modules,
                        layer: config.layer,
                        enable_esc_key: config.enable_esc_key,
                    },
                    outputs,
                    custom,
                    updates: config.updates.map(Updates::new),
                    workspaces: Workspaces::new(config.workspaces),
                    window_title: WindowTitle::new(config.window_title),
                    system_info: SystemInfo::new(config.system_info),
                    keyboard_layout: KeyboardLayout::new(config.keyboard_layout),
                    keyboard_submap: KeyboardSubmap::default(),
                    tray: TrayModule::default(),
                    tempo: Tempo::new(config.tempo),
                    privacy: Privacy::default(),
                    settings: Settings::new(config.settings),
                    notifications,
                    media_player: MediaPlayer::new(config.media_player),
                    visible: true,
                },
                task,
            )
        }
    }

    fn refresh_config(&mut self, config: Box<Config>) {
        self.general_config = GeneralConfig {
            outputs: config.outputs,
            modules: config.modules,
            layer: config.layer,
            enable_esc_key: config.enable_esc_key,
        };
        self.theme = AshellTheme::new(config.position, &config.appearance);
        let custom = config
            .custom_modules
            .into_iter()
            .map(|o| (o.name.clone(), Custom::new(o)))
            .collect();

        self.custom = custom;
        self.updates = config.updates.map(Updates::new);

        // ignore task, since config change should not generate any
        let _ = self
            .workspaces
            .update(modules::workspaces::Message::ConfigReloaded(
                config.workspaces,
            ))
            .map(Message::Workspaces);

        self.window_title
            .update(modules::window_title::Message::ConfigReloaded(
                config.window_title,
            ));

        self.system_info = SystemInfo::new(config.system_info);

        let _ = self
            .keyboard_layout
            .update(modules::keyboard_layout::Message::ConfigReloaded(
                config.keyboard_layout,
            ))
            .map(Message::KeyboardLayout);

        self.keyboard_submap = KeyboardSubmap::default();
        self.tempo
            .update(modules::tempo::Message::ConfigReloaded(config.tempo));
        self.settings
            .update(modules::settings::Message::ConfigReloaded(config.settings));
        self.media_player
            .update(modules::media_player::Message::ConfigReloaded(
                config.media_player,
            ));
        let _ = self
            .notifications
            .update(modules::notifications::Message::ConfigReloaded(
                config.notifications,
            ));
    }

    pub fn theme(&self) -> Theme {
        self.theme.get_theme().clone()
    }

    pub fn scale_factor(&self) -> f64 {
        self.theme.scale_factor
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ConfigChanged(config) => {
                info!("New config: {config:?}");
                let mut tasks = Vec::new();
                info!(
                    "Current outputs: {:?}, new outputs: {:?}",
                    self.general_config.outputs, config.outputs
                );
                if self.general_config.outputs != config.outputs
                    || self.theme.bar_position != config.position
                    || self.theme.bar_style != config.appearance.style
                    || self.theme.scale_factor != config.appearance.scale_factor
                    || self.general_config.layer != config.layer
                {
                    warn!("Outputs changed, syncing");
                    tasks.push(self.outputs.sync(
                        config.appearance.style,
                        &config.outputs,
                        config.position,
                        config.layer,
                        config.appearance.scale_factor,
                    ));
                }

                self.logger.set_new_spec(get_log_spec(&config.log_level));
                self.refresh_config(config);

                Task::batch(tasks)
            }
            Message::ToggleMenu(menu_type, id, button_ui_ref) => {
                let mut cmd = vec![];
                match &menu_type {
                    MenuType::Updates => {
                        if let Some(updates) = self.updates.as_mut() {
                            updates.update(modules::updates::Message::MenuOpened);
                        }
                    }
                    MenuType::Tray(name) => {
                        self.tray
                            .update(modules::tray::Message::MenuOpened(name.clone()));
                    }
                    MenuType::Settings => {
                        cmd.push(
                            match self.settings.update(modules::settings::Message::MenuOpened) {
                                modules::settings::Action::Command(task) => {
                                    task.map(Message::Settings)
                                }
                                _ => Task::none(),
                            },
                        );
                    }
                    _ => {}
                };
                cmd.push(self.outputs.toggle_menu(
                    id,
                    menu_type,
                    button_ui_ref,
                    self.general_config.enable_esc_key,
                ));

                Task::batch(cmd)
            }
            Message::CloseMenu(id) => self
                .outputs
                .close_menu(id, self.general_config.enable_esc_key),
            Message::Custom(name, msg) => {
                if let Some(custom) = self.custom.get_mut(&name) {
                    custom.update(msg);
                }

                Task::none()
            }
            Message::Updates(msg) => {
                if let Some(updates) = self.updates.as_mut() {
                    match updates.update(msg) {
                        modules::updates::Action::None => Task::none(),
                        modules::updates::Action::CheckForUpdates(task) => {
                            task.map(Message::Updates)
                        }
                        modules::updates::Action::CloseMenu(id, task) => Task::batch(vec![
                            task.map(Message::Updates),
                            self.outputs.close_menu_if(
                                id,
                                MenuType::Updates,
                                self.general_config.enable_esc_key,
                            ),
                        ]),
                    }
                } else {
                    Task::none()
                }
            }
            Message::Workspaces(msg) => self.workspaces.update(msg).map(Message::Workspaces),
            Message::WindowTitle(msg) => {
                self.window_title.update(msg);
                Task::none()
            }
            Message::SystemInfo(msg) => {
                self.system_info.update(msg);
                Task::none()
            }
            Message::KeyboardLayout(message) => self
                .keyboard_layout
                .update(message)
                .map(Message::KeyboardLayout),
            Message::KeyboardSubmap(message) => {
                self.keyboard_submap.update(message);
                Task::none()
            }
            Message::Tray(msg) => match self.tray.update(msg) {
                modules::tray::Action::None => Task::none(),
                modules::tray::Action::ToggleMenu(name, id, button_ui_ref) => {
                    self.outputs.toggle_menu(
                        id,
                        MenuType::Tray(name),
                        button_ui_ref,
                        self.general_config.enable_esc_key,
                    )
                }
                modules::tray::Action::TrayMenuCommand(task) => Task::batch(vec![
                    self.outputs
                        .close_all_menus(self.general_config.enable_esc_key),
                    task.map(Message::Tray),
                ]),
                modules::tray::Action::CloseTrayMenu(name) => self
                    .outputs
                    .close_all_menu_if(MenuType::Tray(name), self.general_config.enable_esc_key),
            },
            Message::Tempo(message) => match self.tempo.update(message) {
                modules::tempo::Action::None => Task::none(),
            },
            Message::Privacy(msg) => {
                self.privacy.update(msg);
                Task::none()
            }
            Message::Settings(message) => match self.settings.update(message) {
                modules::settings::Action::None => Task::none(),
                modules::settings::Action::Command(task) => task.map(Message::Settings),
                modules::settings::Action::CloseMenu(id) => self
                    .outputs
                    .close_menu(id, self.general_config.enable_esc_key),
                modules::settings::Action::RequestKeyboard(id) => self.outputs.request_keyboard(id),
                modules::settings::Action::ReleaseKeyboard(id) => self.outputs.release_keyboard(id),
                modules::settings::Action::ReleaseKeyboardWithCommand(id, task) => {
                    Task::batch(vec![
                        task.map(Message::Settings),
                        self.outputs.release_keyboard(id),
                    ])
                }
            },
            Message::OutputEvent(event) => match event {
                OutputEvent::Added(info) => {
                    info!("Output created: {info:?}");
                    let name = &info.name;

                    if let Some((_, h)) = info.logical_size {
                        self.outputs.set_output_logical_height(info.id, h as u32);
                    }

                    self.outputs.add(
                        self.theme.bar_style,
                        &self.general_config.outputs,
                        self.theme.bar_position,
                        self.general_config.layer,
                        name,
                        info.id,
                        self.theme.scale_factor,
                    )
                }
                OutputEvent::Removed(output_id) => {
                    info!("Output destroyed");
                    self.outputs.remove(
                        self.theme.bar_style,
                        self.theme.bar_position,
                        self.general_config.layer,
                        output_id,
                        self.theme.scale_factor,
                    )
                }
                OutputEvent::InfoChanged(_) => Task::none(),
            },
            Message::MediaPlayer(msg) => match self.media_player.update(msg) {
                modules::media_player::Action::None => Task::none(),
                modules::media_player::Action::Command(task) => task.map(Message::MediaPlayer),
            },
            Message::CloseAllMenus => {
                if self.outputs.menu_is_open() {
                    self.outputs
                        .close_all_menus(self.general_config.enable_esc_key)
                } else {
                    Task::none()
                }
            }
            Message::ResumeFromSleep => self.outputs.sync(
                self.theme.bar_style,
                &self.general_config.outputs,
                self.theme.bar_position,
                self.general_config.layer,
                self.theme.scale_factor,
            ),
            Message::Notifications(message) => match self.notifications.update(message) {
                modules::notifications::Action::None => Task::none(),
                modules::notifications::Action::Task(task) => task.map(Message::Notifications),
                modules::notifications::Action::Show(task) => {
                    let position = self.notifications.toast_position();
                    let width = crate::components::MenuSize::Medium.size() as u32;
                    Task::batch(vec![
                        task.map(Message::Notifications),
                        self.outputs.show_toast_layer(width, position),
                    ])
                }
                modules::notifications::Action::Hide(task) => Task::batch(vec![
                    task.map(Message::Notifications),
                    self.outputs.hide_toast_layer(),
                ]),
                modules::notifications::Action::UpdateToastInputRegion(content_size) => {
                    let position = self.notifications.toast_position();
                    self.outputs
                        .update_toast_input_region(content_size, position)
                }
            },
            Message::None => Task::none(),
            Message::ToggleVisibility => {
                self.visible = !self.visible;
                let height = if self.visible {
                    (crate::HEIGHT
                        - match self.theme.bar_style {
                            AppearanceStyle::Solid | AppearanceStyle::Gradient => 8.,
                            AppearanceStyle::Islands => 0.,
                        })
                        * self.theme.scale_factor
                } else {
                    0.0
                };

                Task::batch(
                    self.outputs
                        .iter()
                        .filter_map(|(_, shell_info, _)| {
                            shell_info
                                .as_ref()
                                .map(|info| set_exclusive_zone(info.id, height as i32))
                        })
                        .collect::<Vec<_>>(),
                )
            }
        }
    }

    pub fn view(&'_ self, id: SurfaceId) -> Element<'_, Message> {
        match self.outputs.has(id) {
            Some(HasOutput::Main) => {
                if !self.visible {
                    return Row::new().into();
                }

                let [left, center, right] = self.modules_section(id, &self.theme);

                let centerbox = Centerbox::new([left, center, right])
                    .spacing(self.theme.space.xxs)
                    .width(Length::Fill)
                    .align_items(Alignment::Center)
                    .height(if self.theme.bar_style == AppearanceStyle::Islands {
                        HEIGHT
                    } else {
                        HEIGHT - self.theme.space.xs as f64
                    } as f32)
                    .padding(if self.theme.bar_style == AppearanceStyle::Islands {
                        [self.theme.space.xxs, self.theme.space.xxs]
                    } else {
                        [0.0, 0.0]
                    });

                let status_bar = container(centerbox).style(|t: &Theme| container::Style {
                    background: match self.theme.bar_style {
                        AppearanceStyle::Gradient => Some({
                            let start_color =
                                t.palette().background.scale_alpha(self.theme.opacity);

                            let start_color = if self.outputs.menu_is_open() {
                                darken_color(start_color, self.theme.menu.backdrop)
                            } else {
                                start_color
                            };

                            let end_color = if self.outputs.menu_is_open() {
                                backdrop_color(self.theme.menu.backdrop)
                            } else {
                                Color::TRANSPARENT
                            };

                            Gradient::Linear(
                                Linear::new(Radians(PI))
                                    .add_stop(
                                        0.0,
                                        match self.theme.bar_position {
                                            Position::Top => start_color,
                                            Position::Bottom => end_color,
                                        },
                                    )
                                    .add_stop(
                                        1.0,
                                        match self.theme.bar_position {
                                            Position::Top => end_color,
                                            Position::Bottom => start_color,
                                        },
                                    ),
                            )
                            .into()
                        }),
                        AppearanceStyle::Solid => Some({
                            let bg = t.palette().background.scale_alpha(self.theme.opacity);
                            if self.outputs.menu_is_open() {
                                darken_color(bg, self.theme.menu.backdrop)
                            } else {
                                bg
                            }
                            .into()
                        }),
                        AppearanceStyle::Islands => {
                            if self.outputs.menu_is_open() {
                                Some(backdrop_color(self.theme.menu.backdrop).into())
                            } else {
                                None
                            }
                        }
                    },
                    ..Default::default()
                });

                if self.outputs.menu_is_open() {
                    mouse_area(status_bar)
                        .on_release(Message::CloseMenu(id))
                        .into()
                } else {
                    status_bar.into()
                }
            }
            Some(HasOutput::Menu(menu_info)) => match menu_info {
                Some((MenuType::Updates, button_ui_ref)) => {
                    if let Some(updates) = self.updates.as_ref() {
                        self.menu_wrapper(
                            id,
                            updates.menu_view(id, &self.theme).map(Message::Updates),
                            *button_ui_ref,
                        )
                    } else {
                        Row::new().into()
                    }
                }
                Some((MenuType::Tray(name), button_ui_ref)) => self.menu_wrapper(
                    id,
                    self.tray.menu_view(&self.theme, name).map(Message::Tray),
                    *button_ui_ref,
                ),
                Some((MenuType::Notifications, button_ui_ref)) => self.menu_wrapper(
                    id,
                    self.notifications
                        .menu_view(&self.theme)
                        .map(Message::Notifications),
                    *button_ui_ref,
                ),
                Some((MenuType::Settings, button_ui_ref)) => self.menu_wrapper(
                    id,
                    self.settings
                        .menu_view(id, &self.theme, self.theme.bar_position)
                        .map(Message::Settings),
                    *button_ui_ref,
                ),
                Some((MenuType::MediaPlayer, button_ui_ref)) => self.menu_wrapper(
                    id,
                    self.media_player
                        .menu_view(&self.theme)
                        .map(Message::MediaPlayer),
                    *button_ui_ref,
                ),
                Some((MenuType::SystemInfo, button_ui_ref)) => self.menu_wrapper(
                    id,
                    self.system_info
                        .menu_view(&self.theme)
                        .map(Message::SystemInfo),
                    *button_ui_ref,
                ),

                Some((MenuType::Tempo, button_ui_ref)) => self.menu_wrapper(
                    id,
                    self.tempo.menu_view(&self.theme).map(Message::Tempo),
                    *button_ui_ref,
                ),
                None => Row::new().into(),
            },
            Some(HasOutput::Toast) => self
                .notifications
                .toast_view(&self.theme)
                .map(Message::Notifications),
            None => Row::new().into(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            Subscription::batch(self.modules_subscriptions(&self.general_config.modules.left)),
            Subscription::batch(self.modules_subscriptions(&self.general_config.modules.center)),
            Subscription::batch(self.modules_subscriptions(&self.general_config.modules.right)),
            config::subscription(&self.config_path),
            crate::services::logind::LogindService::subscribe().map(|event| match event {
                crate::services::ServiceEvent::Update(_) => Message::ResumeFromSleep,
                _ => Message::None,
            }),
            iced::output_events().map(Message::OutputEvent),
            listen_with(move |evt, _, _| match evt {
                iced::event::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                    debug!("Keyboard event received: {key:?}");
                    if matches!(key, keyboard::Key::Named(keyboard::key::Named::Escape)) {
                        debug!("ESC key pressed, closing all menus");
                        Some(Message::CloseAllMenus)
                    } else {
                        None
                    }
                }
                _ => None,
            }),
            Subscription::run(|| {
                use iced::futures::StreamExt;
                signal_hook_tokio::Signals::new([libc::SIGUSR1])
                    .expect("Failed to create signal stream")
                    .filter_map(|sig| {
                        if sig == libc::SIGUSR1 {
                            iced::futures::future::ready(Some(Message::ToggleVisibility))
                        } else {
                            iced::futures::future::ready(None)
                        }
                    })
            }),
            Subscription::run(|| {
                iced::futures::stream::unfold(None::<tokio::net::UnixListener>, |listener| async {
                    let listener = match listener {
                        Some(l) => l,
                        None => {
                            let std_listener = match crate::ipc::create_listener() {
                                Ok(l) => l,
                                Err(e) => {
                                    log::error!("Failed to create IPC listener: {e:#}");
                                    return None;
                                }
                            };
                            match tokio::net::UnixListener::from_std(std_listener) {
                                Ok(l) => l,
                                Err(e) => {
                                    log::error!("Failed to convert IPC listener to tokio: {e}");
                                    return None;
                                }
                            }
                        }
                    };
                    match listener.accept().await {
                        Ok((stream, _)) => {
                            let msg = match stream.into_std() {
                                Ok(std_stream) => match crate::ipc::read_request(&std_stream) {
                                    Ok(request) => {
                                        let mut writer = std_stream;
                                        crate::ipc::write_response(&mut writer, "ok");
                                        match request {
                                            crate::ipc::IpcRequest::ToggleVisibility => {
                                                Message::ToggleVisibility
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let mut writer = std_stream;
                                        crate::ipc::write_error(&mut writer, &format!("{e:#}"));
                                        Message::None
                                    }
                                },
                                Err(e) => {
                                    log::error!("IPC stream conversion error: {e}");
                                    Message::None
                                }
                            };
                            Some((msg, Some(listener)))
                        }
                        Err(e) => {
                            log::error!("IPC accept error: {e}");
                            Some((Message::None, Some(listener)))
                        }
                    }
                })
            }),
        ])
    }
}
