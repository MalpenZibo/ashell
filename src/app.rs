use crate::{
    HEIGHT,
    components::{ButtonUIRef, Centerbox, menu::MenuType},
    config::{self, AppearanceStyle, Config, Modules, Position},
    get_log_spec,
    i18n::{Localizer, init_localizer},
    ipc::IpcCommand,
    modules::{
        self,
        custom_module::{self, Custom},
        keyboard_layout::KeyboardLayout,
        keyboard_submap::KeyboardSubmap,
        media_player::MediaPlayer,
        notifications::Notifications,
        privacy::Privacy,
        settings::{self, Settings, audio},
        system_info::SystemInfo,
        tempo::Tempo,
        tray::TrayModule,
        updates::Updates,
        window_title::WindowTitle,
        workspaces::Workspaces,
    },
    osd::{self, Osd, OsdKind},
    outputs::{HasOutput, Outputs},
    services::ReadOnlyService,
    theme::{AshellTheme, backdrop_color, darken_color, init_theme, use_theme},
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

const OSD_WIDTH: u32 = 250;
const OSD_HEIGHT: u32 = 64;

fn resolve_localizer(config: &Config) -> Localizer {
    Localizer::resolve(config.language.as_deref(), config.region.as_deref())
}

pub struct GeneralConfig {
    outputs: config::Outputs,
    pub modules: Modules,
    pub layer: config::Layer,
    enable_esc_key: bool,
}

pub struct App {
    config_path: PathBuf,
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
    pub osd: Osd,
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
    Osd(osd::Message),
    IpcOsdCommand(IpcCommand),
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
            let outputs = Outputs::new(
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

            init_theme(AshellTheme::new(
                config.position,
                &config.appearance,
                &config.animations,
            ));
            init_localizer(resolve_localizer(&config));

            let notifications = Notifications::new(config.notifications);

            (
                App {
                    config_path,
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
                    tray: TrayModule::new(config.tray),
                    tempo: Tempo::new(config.tempo),
                    privacy: Privacy::default(),
                    settings: Settings::new(config.settings),
                    notifications,
                    media_player: MediaPlayer::new(config.media_player),
                    osd: Osd::new(config.osd),
                    visible: true,
                },
                Task::none(),
            )
        }
    }

    fn refresh_config(&mut self, config: Box<Config>) {
        init_theme(AshellTheme::new(
            config.position,
            &config.appearance,
            &config.animations,
        ));
        init_localizer(resolve_localizer(&config));
        self.general_config = GeneralConfig {
            outputs: config.outputs,
            modules: config.modules,
            layer: config.layer,
            enable_esc_key: config.enable_esc_key,
        };
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
        self.osd.update(osd::Message::ConfigReloaded(config.osd));
    }

    pub fn theme(&self) -> Theme {
        use_theme(|t| t.iced_theme.clone())
    }

    pub fn scale_factor(&self) -> f64 {
        use_theme(|t| t.scale_factor)
    }

    /// Build OSD display info (kind, normalised value, muted) for the given
    /// IPC command, reading current state from the Settings services.
    fn osd_info_for(&self, cmd: &IpcCommand) -> Option<(OsdKind, f32, bool)> {
        fn normalise(cur: u32, max: u32) -> f32 {
            if max > 0 {
                cur as f32 / max as f32
            } else {
                0.0
            }
        }

        match cmd {
            IpcCommand::VolumeUp { .. } | IpcCommand::VolumeDown { .. } => {
                // Use slider value — it has the optimistic RequestAndTimeout update,
                // which was computed from real_sink_volume in volume_adjust().
                let vol = self.settings.audio().current_sink_volume().unwrap_or(0);
                let muted = self.settings.audio().is_sink_muted().unwrap_or(false);
                Some((
                    OsdKind::Volume,
                    normalise(vol, audio::AudioSettings::vol_max()),
                    muted,
                ))
            }
            IpcCommand::VolumeToggleMute { .. } => {
                let vol = self.settings.audio().real_sink_volume().unwrap_or(0);
                // Invert: the toggle was just sent but PulseAudio hasn't
                // round-tripped yet, so the current state is stale.
                let muted = !self.settings.audio().is_sink_muted().unwrap_or(false);
                Some((
                    OsdKind::Volume,
                    normalise(vol, audio::AudioSettings::vol_max()),
                    muted,
                ))
            }
            IpcCommand::MicrophoneUp { .. } | IpcCommand::MicrophoneDown { .. } => {
                // Use slider value — it has the optimistic RequestAndTimeout update,
                // which was computed from real_source_volume in microphone_adjust().
                let vol = self.settings.audio().current_source_volume().unwrap_or(0);
                let muted = self.settings.audio().is_source_muted().unwrap_or(false);
                Some((
                    OsdKind::Microphone,
                    normalise(vol, audio::AudioSettings::mic_max()),
                    muted,
                ))
            }
            IpcCommand::MicrophoneToggleMute { .. } => {
                let vol = self.settings.audio().real_source_volume().unwrap_or(0);
                // Invert: the toggle was just sent but PulseAudio hasn't
                // round-tripped yet, so the current state is stale.
                let muted = !self.settings.audio().is_source_muted().unwrap_or(false);
                Some((
                    OsdKind::Microphone,
                    normalise(vol, audio::AudioSettings::mic_max()),
                    muted,
                ))
            }
            IpcCommand::BrightnessUp { .. } | IpcCommand::BrightnessDown { .. } => self
                .settings
                .brightness()
                .current_brightness()
                .map(|(cur, max)| (OsdKind::Brightness, normalise(cur, max), false)),
            IpcCommand::ToggleAirplaneMode { .. } => {
                // After toggle: the new state is the opposite of current.
                // For toggles, `muted` carries the active/enabled state; `value` is unused.
                let active = !self.settings.network().is_airplane_mode().unwrap_or(false);
                Some((OsdKind::Airplane, 0.0, active))
            }
            IpcCommand::ToggleIdleInhibitor { .. } => {
                if let Some(idle_inhibitor) = self.settings.idle_inhibitor() {
                    let active = idle_inhibitor.is_inhibited();
                    Some((OsdKind::IdleInhibitor, 0.0, active))
                } else {
                    None
                }
            }
            IpcCommand::ToggleVisibility => None,
        }
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
                let (bar_position, bar_style, scale_factor) =
                    use_theme(|t| (t.bar_position, t.bar_style, t.scale_factor));
                if self.general_config.outputs != config.outputs
                    || bar_position != config.position
                    || bar_style != config.appearance.style
                    || scale_factor != config.appearance.scale_factor
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
            Message::CloseMenu(id) => {
                self.outputs
                    .close_menu(id, None, self.general_config.enable_esc_key)
            }
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
                            self.outputs.close_menu(
                                id,
                                Some(MenuType::Updates),
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
                modules::tray::Action::TrayMenuCommandKeepOpen(task) => task.map(Message::Tray),
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
                modules::settings::Action::CloseMenu(id) => {
                    self.outputs
                        .close_menu(id, None, self.general_config.enable_esc_key)
                }
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
                    let name = &format!("{} {} {}", info.name, info.make, info.model);

                    if let Some((_, h)) = info.logical_size {
                        self.outputs.set_output_logical_height(info.id, h as u32);
                    }

                    let (bar_style, bar_position, scale_factor) =
                        use_theme(|t| (t.bar_style, t.bar_position, t.scale_factor));
                    self.outputs.add(
                        bar_style,
                        &self.general_config.outputs,
                        bar_position,
                        self.general_config.layer,
                        name,
                        info.id,
                        scale_factor,
                    )
                }
                OutputEvent::Removed(output_id) => {
                    info!("Output destroyed");
                    let (bar_style, bar_position, scale_factor) =
                        use_theme(|t| (t.bar_style, t.bar_position, t.scale_factor));
                    self.outputs.remove(
                        bar_style,
                        bar_position,
                        self.general_config.layer,
                        output_id,
                        scale_factor,
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
            Message::ResumeFromSleep => {
                let (bar_style, bar_position, scale_factor) =
                    use_theme(|t| (t.bar_style, t.bar_position, t.scale_factor));
                self.outputs.sync(
                    bar_style,
                    &self.general_config.outputs,
                    bar_position,
                    self.general_config.layer,
                    scale_factor,
                )
            }
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
            Message::IpcOsdCommand(cmd) => {
                let mut tasks = vec![];

                // Execute the action via Settings.
                let action = match &cmd {
                    IpcCommand::VolumeUp { .. } => self.settings.volume_adjust(true),
                    IpcCommand::VolumeDown { .. } => self.settings.volume_adjust(false),
                    IpcCommand::VolumeToggleMute { .. } => self.settings.toggle_mute(),
                    IpcCommand::MicrophoneUp { .. } => self.settings.microphone_adjust(true),
                    IpcCommand::MicrophoneDown { .. } => self.settings.microphone_adjust(false),
                    IpcCommand::MicrophoneToggleMute { .. } => {
                        self.settings.microphone_toggle_mute()
                    }
                    IpcCommand::BrightnessUp { .. } => self.settings.brightness_adjust(true),
                    IpcCommand::BrightnessDown { .. } => self.settings.brightness_adjust(false),
                    IpcCommand::ToggleAirplaneMode { .. } => self.settings.toggle_airplane(),
                    IpcCommand::ToggleIdleInhibitor { .. } => self.settings.toggle_idle_inhibitor(),
                    IpcCommand::ToggleVisibility => unreachable!(),
                };
                if let settings::Action::Command(task) = action {
                    tasks.push(task.map(Message::Settings));
                }

                // Show OSD overlay if enabled.
                if self.osd.config().enabled && !cmd.no_osd() {
                    let osd_info = self.osd_info_for(&cmd);

                    if let Some((kind, value, muted)) = osd_info
                        && let osd::Action::Show(timer) =
                            self.osd.update(osd::Message::Show { kind, value, muted })
                    {
                        tasks.push(timer.map(Message::Osd));
                        tasks.push(self.outputs.show_osd_layer(OSD_WIDTH, OSD_HEIGHT));
                    }
                }

                Task::batch(tasks)
            }
            Message::Osd(msg) => match self.osd.update(msg) {
                osd::Action::Hide => self.outputs.hide_osd_layer(),
                _ => Task::none(),
            },
            Message::None => Task::none(),
            Message::ToggleVisibility => {
                self.visible = !self.visible;
                let (bar_style, scale_factor) = use_theme(|t| (t.bar_style, t.scale_factor));
                let height = if self.visible {
                    (crate::HEIGHT
                        - match bar_style {
                            AppearanceStyle::Solid | AppearanceStyle::Gradient => 8.,
                            AppearanceStyle::Islands => 0.,
                        })
                        * scale_factor
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

                let [left, center, right] = self.modules_section(id);

                let (space, bar_style, bar_position, opacity, menu, animations_enabled) =
                    use_theme(|t| {
                        (
                            t.space,
                            t.bar_style,
                            t.bar_position,
                            t.opacity,
                            t.menu,
                            t.animations_enabled,
                        )
                    });
                let centerbox = Centerbox::new([left, center, right])
                    .animated(animations_enabled)
                    .spacing(space.xxs)
                    .width(Length::Fill)
                    .align_items(Alignment::Center)
                    .height(if bar_style == AppearanceStyle::Islands {
                        HEIGHT
                    } else {
                        HEIGHT - space.xs as f64
                    } as f32)
                    .padding(if bar_style == AppearanceStyle::Islands {
                        [space.xxs, space.xxs]
                    } else {
                        [0.0, 0.0]
                    });

                let menu_is_open = self.outputs.menu_is_open();
                let status_bar = container(centerbox).style(move |t: &Theme| container::Style {
                    background: match bar_style {
                        AppearanceStyle::Gradient => Some({
                            let start_color = t.palette().background.scale_alpha(opacity);

                            let start_color = if menu_is_open {
                                darken_color(start_color, menu.backdrop)
                            } else {
                                start_color
                            };

                            let end_color = if menu_is_open {
                                backdrop_color(menu.backdrop)
                            } else {
                                Color::TRANSPARENT
                            };

                            Gradient::Linear(
                                Linear::new(Radians(PI))
                                    .add_stop(
                                        0.0,
                                        match bar_position {
                                            Position::Top => start_color,
                                            Position::Bottom => end_color,
                                        },
                                    )
                                    .add_stop(
                                        1.0,
                                        match bar_position {
                                            Position::Top => end_color,
                                            Position::Bottom => start_color,
                                        },
                                    ),
                            )
                            .into()
                        }),
                        AppearanceStyle::Solid => Some({
                            let bg = t.palette().background.scale_alpha(opacity);
                            if menu_is_open {
                                darken_color(bg, menu.backdrop)
                            } else {
                                bg
                            }
                            .into()
                        }),
                        AppearanceStyle::Islands => {
                            if menu_is_open {
                                Some(backdrop_color(menu.backdrop).into())
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
            Some(HasOutput::Menu(Some(open_menu))) => {
                let ui_ref = open_menu.button_ui_ref;
                match &open_menu.menu_type {
                    MenuType::Updates => {
                        if let Some(updates) = self.updates.as_ref() {
                            self.menu_wrapper(
                                id,
                                updates.menu_view(id).map(Message::Updates),
                                ui_ref,
                            )
                        } else {
                            Row::new().into()
                        }
                    }
                    MenuType::Tray(name) => {
                        self.menu_wrapper(id, self.tray.menu_view(name).map(Message::Tray), ui_ref)
                    }
                    MenuType::Notifications => self.menu_wrapper(
                        id,
                        self.notifications.menu_view().map(Message::Notifications),
                        ui_ref,
                    ),
                    MenuType::Settings => self.menu_wrapper(
                        id,
                        self.settings
                            .menu_view(id, use_theme(|t| t.bar_position))
                            .map(Message::Settings),
                        ui_ref,
                    ),
                    MenuType::MediaPlayer => self.menu_wrapper(
                        id,
                        self.media_player.menu_view().map(Message::MediaPlayer),
                        ui_ref,
                    ),
                    MenuType::SystemInfo => self.menu_wrapper(
                        id,
                        self.system_info.menu_view().map(Message::SystemInfo),
                        ui_ref,
                    ),
                    MenuType::Tempo => {
                        self.menu_wrapper(id, self.tempo.menu_view().map(Message::Tempo), ui_ref)
                    }
                }
            }
            Some(HasOutput::Menu(None)) => Row::new().into(),
            Some(HasOutput::Toast) => self.notifications.toast_view().map(Message::Notifications),
            Some(HasOutput::Osd) => self.osd.view().map(Message::Osd),
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
            // Always subscribe to audio/brightness services so OSD works
            // even when the Settings module isn't in the module list.
            self.settings.subscription().map(Message::Settings),
            crate::ipc::subscription().map(|cmd| match cmd {
                IpcCommand::ToggleVisibility => Message::ToggleVisibility,
                other => Message::IpcOsdCommand(other),
            }),
        ])
    }
}
