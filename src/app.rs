use crate::{
    HEIGHT,
    components::{Centerbox, menu::MenuType},
    config::{self, BarSurface, Config, ModuleName, Modules, WorkspaceIndicatorFormat},
    get_log_spec,
    i18n::{Localizer, init_localizer},
    ipc::IpcCommand,
    modules::{
        self,
        custom_module::Custom,
        keyboard_layout::KeyboardLayout,
        keyboard_submap::KeyboardSubmap,
        media_player::MediaPlayer,
        notifications::Notifications,
        privacy::Privacy,
        settings::{self, Settings},
        system_info::SystemInfo,
        tempo::Tempo,
        tray::TrayModule,
        updates::Updates,
        window_title::WindowTitle,
        workspaces::Workspaces,
    },
    osd::{self, Osd},
    outputs::{HasOutput, Outputs},
    services::{ReadOnlyService, xdg_icons},
    theme::{AshellTheme, BarLayout, backdrop_color, darken_color, init_theme, use_theme},
};
use flexi_logger::LoggerHandle;
use iced::futures::StreamExt;
use iced::{
    Alignment, Element, Length, OutputEvent, Subscription, SurfaceId, Task, Theme,
    event::listen_with,
    keyboard, set_exclusive_zone,
    widget::{Row, container, mouse_area},
};
use log::{debug, info, warn};
use std::{collections::HashMap, path::PathBuf};

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

mod message;
mod osd_info;

pub use message::Message;

impl App {
    pub fn new(
        (logger, config, config_path): (LoggerHandle, Config, PathBuf),
    ) -> impl FnOnce() -> (Self, Task<Message>) {
        move || {
            let mut outputs = Outputs::new(
                BarLayout::from_appearance(&config.appearance.bar),
                config.position,
                config.layer,
                config.appearance.scale_factor,
            );
            outputs.set_animations_enabled(config.animations.enabled);

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

            let notifications = Notifications::new(config.notifications, config.animations.enabled);

            let needs_icons = config.modules.contains(&ModuleName::Tray)
                || config.workspaces.indicator_format == WorkspaceIndicatorFormat::NameAndIcons;
            let warm_icons = if needs_icons {
                Task::perform(xdg_icons::warm_cache_async(), |()| ()).discard()
            } else {
                Task::none()
            };

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
                warm_icons,
            )
        }
    }

    fn refresh_config(&mut self, config: Box<Config>) -> Task<Message> {
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

        let workspaces_task = self
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
        self.outputs
            .set_animations_enabled(config.animations.enabled);
        self.notifications
            .set_animations_enabled(config.animations.enabled);
        let _ = self
            .notifications
            .update(modules::notifications::Message::ConfigReloaded(
                config.notifications,
            ));
        self.osd.update(osd::Message::ConfigReloaded(config.osd));

        workspaces_task
    }

    pub fn theme(&self) -> Theme {
        use_theme(|t| t.iced_theme.clone())
    }

    pub fn scale_factor(&self) -> f64 {
        use_theme(|t| t.scale_factor)
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
                let (bar_position, bar_layout, scale_factor) =
                    use_theme(|t| (t.bar_position, t.bar_layout(), t.scale_factor));
                let new_layout = BarLayout::from_appearance(&config.appearance.bar);
                if self.general_config.outputs != config.outputs
                    || bar_position != config.position
                    || bar_layout != new_layout
                    || scale_factor != config.appearance.scale_factor
                    || self.general_config.layer != config.layer
                {
                    warn!("Outputs changed, syncing");
                    tasks.push(self.outputs.sync(
                        new_layout,
                        &config.outputs,
                        config.position,
                        config.layer,
                        config.appearance.scale_factor,
                    ));
                }

                self.logger.set_new_spec(get_log_spec(&config.log_level));
                tasks.push(self.refresh_config(config));

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
            Message::FinishCloseMenu(id) => self.outputs.finish_close_menu(id),
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
                modules::settings::Action::OpenTooltipMenu(id, menu_type, ui_ref) => {
                    self.outputs.toggle_menu(id, menu_type, ui_ref, false)
                }
                modules::settings::Action::CloseTooltipMenu(id, menu_type) => self
                    .outputs
                    .close_menu(id, Some(menu_type), self.general_config.enable_esc_key),
            },
            Message::OutputEvent(event) => match event {
                OutputEvent::Added(info) => {
                    info!("Output created: {info:?}");
                    // Pass both the canonical name and the full EDID
                    // description down to Outputs::add. The workspace
                    // visibility filter compares against just the
                    // canonical `info.name` (matches `w.monitor` from
                    // the compositor); name_in_config / has_name keep
                    // matching against the concatenated description
                    // too so #312's fuzzy-EDID-alias config behaviour
                    // is preserved.
                    let name = info.name.as_str();
                    let description = format!("{} {} {}", info.name, info.make, info.model);

                    let (bar_layout, bar_position, scale_factor) =
                        use_theme(|t| (t.bar_layout(), t.bar_position, t.scale_factor));
                    let task = self.outputs.add(
                        bar_layout,
                        &self.general_config.outputs,
                        bar_position,
                        self.general_config.layer,
                        name,
                        &description,
                        info.id,
                        scale_factor,
                    );

                    // After add(), so the output's entry exists to attach the height to.
                    if let Some((_, h)) = info.logical_size {
                        self.outputs.set_output_logical_height(info.id, h as u32);
                    }

                    task
                }
                OutputEvent::Removed(output_id) => {
                    info!("Output destroyed");
                    let (bar_layout, bar_position, scale_factor) =
                        use_theme(|t| (t.bar_layout(), t.bar_position, t.scale_factor));
                    self.outputs.remove(
                        bar_layout,
                        bar_position,
                        self.general_config.layer,
                        output_id,
                        scale_factor,
                    )
                }
                OutputEvent::InfoChanged(_) => Task::none(),
                OutputEvent::SurfaceEnteredOutput { surface, output } => {
                    self.outputs.surface_entered_output(surface, output)
                }
                OutputEvent::SurfaceLeftOutput { surface, output } => {
                    self.outputs.surface_left_output(surface, output);
                    Task::none()
                }
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
                let (bar_layout, bar_position, scale_factor) =
                    use_theme(|t| (t.bar_layout(), t.bar_position, t.scale_factor));
                self.outputs.sync(
                    bar_layout,
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
                    // Double width gives the card a runway to fully exit on slide-out.
                    let card_width = crate::components::MenuSize::Medium.size() as u32;
                    let width = card_width * 2;
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
                    IpcCommand::ToggleVisibility => {
                        warn!(
                            "IpcCommand::ToggleVisibility reached IpcOsdCommand handler; use Message::ToggleVisibility instead"
                        );
                        modules::settings::Action::None
                    }
                };
                if let settings::Action::Command(task) = action {
                    tasks.push(task.map(Message::Settings));
                }

                // Show OSD overlay if enabled.
                if self.osd.config().enabled && !cmd.no_osd() {
                    let osd_info = osd_info::osd_info_for(self, &cmd);

                    if let Some((kind, value, scale, muted)) = osd_info
                        && let osd::Action::Show(timer) = self.osd.update(osd::Message::Show {
                            kind,
                            value,
                            scale,
                            muted,
                        })
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
                let (bar_layout, bar_position, scale_factor) =
                    use_theme(|t| (t.bar_layout(), t.bar_position, t.scale_factor));
                let zone = if self.visible {
                    Outputs::exclusive_zone(bar_layout, bar_position, scale_factor)
                } else {
                    0
                };

                Task::batch(
                    self.outputs
                        .iter()
                        .filter_map(|(_, shell_info, _)| {
                            shell_info
                                .as_ref()
                                .map(|info| set_exclusive_zone(info.id, zone))
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

                let (space, bar_surface, opacity, menu, animations_enabled, bar_radius) =
                    use_theme(|t| {
                        (
                            t.space,
                            t.bar_surface,
                            t.opacity,
                            t.menu,
                            t.animations_enabled,
                            t.bar_border_radius(),
                        )
                    });
                let centerbox = Centerbox::new([left, center, right])
                    .animated(animations_enabled)
                    .spacing(space.xxs)
                    .width(Length::Fill)
                    .align_items(Alignment::Center)
                    .height(if bar_surface == BarSurface::Transparent {
                        HEIGHT
                    } else {
                        HEIGHT - space.xs as f64
                    } as f32)
                    .padding(if bar_surface == BarSurface::Transparent {
                        [space.xxs, space.xxs]
                    } else {
                        [0.0, 0.0]
                    });

                let menu_is_open = self.outputs.menu_is_open();
                let status_bar = container(centerbox).style(move |t: &Theme| container::Style {
                    background: match bar_surface {
                        BarSurface::Solid => Some({
                            let bg = t.palette().background.scale_alpha(opacity);
                            if menu_is_open {
                                darken_color(bg, menu.backdrop)
                            } else {
                                bg
                            }
                            .into()
                        }),
                        BarSurface::Transparent => {
                            if menu_is_open {
                                Some(backdrop_color(menu.backdrop).into())
                            } else {
                                None
                            }
                        }
                    },
                    border: iced::Border {
                        radius: bar_radius,
                        ..Default::default()
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
                    MenuType::AudioTooltip
                    | MenuType::BluetoothTooltip
                    | MenuType::WifiTooltip
                    | MenuType::VpnTooltip
                    | MenuType::BatteryTooltip
                    | MenuType::PeripheralBatteryTooltip(_) => self.menu_wrapper(
                        id,
                        self.settings
                            .tooltip_view(&open_menu.menu_type)
                            .map(Message::Settings),
                        ui_ref,
                    ),
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
            Subscription::run(|| match signal_hook_tokio::Signals::new([libc::SIGUSR1]) {
                Ok(signals) => signals
                    .filter_map(|sig| {
                        if sig == libc::SIGUSR1 {
                            iced::futures::future::ready(Some(Message::ToggleVisibility))
                        } else {
                            iced::futures::future::ready(None)
                        }
                    })
                    .boxed(),
                Err(e) => {
                    log::error!("Failed to create signal stream: {e}");
                    iced::futures::stream::empty().boxed()
                }
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
