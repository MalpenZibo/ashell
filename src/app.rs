use crate::{
    HEIGHT, centerbox,
    config::{self, AppearanceStyle, Config, Position},
    get_log_spec,
    menu::{MenuSize, MenuType},
    modules::{
        self,
        app_launcher::{self, AppLauncher},
        clipboard::{self, Clipboard},
        clock::Clock,
        custom_module::{self, Custom},
        keyboard_layout::KeyboardLayout,
        keyboard_submap::KeyboardSubmap,
        media_player::MediaPlayer,
        privacy::Privacy,
        settings::{Settings, brightness::BrightnessMessage},
        system_info::SystemInfo,
        tray::TrayModule,
        updates::Updates,
        window_title::WindowTitle,
        workspaces::Workspaces,
    },
    outputs::{HasOutput, Outputs},
    position_button::ButtonUIRef,
    services::{Service, brightness::BrightnessCommand},
    theme::{AshellTheme, backdrop_color, darken_color},
};
use flexi_logger::LoggerHandle;
use iced::{
    Alignment, Color, Element, Gradient, Length, Radians, Subscription, Task, Theme,
    daemon::Appearance,
    event::{
        listen_with,
        wayland::{Event as WaylandEvent, OutputEvent},
    },
    gradient::Linear,
    widget::{Row, container},
    window::Id,
};
use log::{debug, info, warn};
use std::{collections::HashMap, f32::consts::PI, path::PathBuf};
use wayland_client::protocol::wl_output::WlOutput;

pub struct App {
    config_path: PathBuf,
    pub theme: AshellTheme,
    logger: LoggerHandle,
    pub config: Config,
    pub outputs: Outputs,
    pub app_launcher: Option<AppLauncher>,
    pub custom: HashMap<String, Custom>,
    pub updates: Option<Updates>,
    pub clipboard: Option<Clipboard>,
    pub workspaces: Workspaces,
    pub window_title: WindowTitle,
    pub system_info: SystemInfo,
    pub keyboard_layout: KeyboardLayout,
    pub keyboard_submap: KeyboardSubmap,
    pub tray: TrayModule,
    pub clock: Clock,
    pub privacy: Privacy,
    pub settings: Settings,
    pub media_player: MediaPlayer,
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
    ConfigChanged(Box<Config>),
    ToggleMenu(MenuType, Id, ButtonUIRef),
    CloseMenu(Id),
    Clipboard(clipboard::Message),
    AppLauncher(app_launcher::Message),
    Custom(String, custom_module::Message),
    Updates(modules::updates::Message),
    Workspaces(modules::workspaces::Message),
    WindowTitle(modules::window_title::Message),
    SystemInfo(modules::system_info::Message),
    KeyboardLayout(modules::keyboard_layout::Message),
    KeyboardSubmap(modules::keyboard_submap::Message),
    Tray(modules::tray::Message),
    Clock(modules::clock::Message),
    Privacy(modules::privacy::Message),
    Settings(modules::settings::Message),
    MediaPlayer(modules::media_player::Message),
    OutputEvent((OutputEvent, WlOutput)),
}

impl App {
    pub fn new(
        (logger, config, config_path): (LoggerHandle, Config, PathBuf),
    ) -> impl FnOnce() -> (Self, Task<Message>) {
        || {
            let (outputs, task) = Outputs::new(
                config.appearance.style,
                config.position,
                config.appearance.scale_factor,
            );

            let custom = config
                .custom_modules
                .clone()
                .into_iter()
                .map(|o| (o.name.clone(), Custom::new(o)))
                .collect();

            (
                App {
                    config_path,
                    theme: AshellTheme::new(config.position, &config.appearance),
                    logger,
                    outputs,
                    app_launcher: config
                        .app_launcher_cmd
                        .as_ref()
                        .map(|cmd| AppLauncher::new(cmd.clone())),
                    custom,
                    updates: config.clone().updates.map(Updates::new),
                    clipboard: config
                        .clipboard_cmd
                        .as_ref()
                        .map(|cmd| Clipboard::new(cmd.clone())),
                    workspaces: Workspaces::new(config.workspaces),
                    window_title: WindowTitle::new(config.window_title),
                    system_info: SystemInfo::new(config.system.clone()),
                    keyboard_layout: KeyboardLayout::new(config.keyboard_layout.clone()),
                    keyboard_submap: KeyboardSubmap::default(),
                    tray: TrayModule::default(),
                    clock: Clock::new(config.clock.clone()),
                    privacy: Privacy::default(),
                    settings: Settings::default(),
                    media_player: MediaPlayer::new(config.media_player.clone()),
                    config,
                },
                task,
            )
        }
    }

    pub fn title(&self, _id: Id) -> String {
        String::from("ashell")
    }

    pub fn theme(&self, _id: Id) -> Theme {
        self.theme.get_theme().clone()
    }

    pub fn style(&self, theme: &Theme) -> Appearance {
        Appearance {
            background_color: Color::TRANSPARENT,
            text_color: theme.palette().text,
            icon_color: theme.palette().text,
        }
    }

    pub fn scale_factor(&self, _id: Id) -> f64 {
        self.config.appearance.scale_factor
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::None => Task::none(),
            Message::ConfigChanged(config) => {
                info!("New config: {config:?}");
                let mut tasks = Vec::new();
                info!(
                    "Current outputs: {:?}, new outputs: {:?}",
                    self.config.outputs, config.outputs
                );
                if self.config.outputs != config.outputs
                    || self.config.position != config.position
                    || self.config.appearance.style != config.appearance.style
                    || self.config.appearance.scale_factor != config.appearance.scale_factor
                {
                    warn!("Outputs changed, syncing");
                    tasks.push(self.outputs.sync(
                        config.appearance.style,
                        &config.outputs,
                        config.position,
                        config.appearance.scale_factor,
                    ));
                }
                let custom = config
                    .custom_modules
                    .clone()
                    .into_iter()
                    .map(|o| (o.name.clone(), Custom::new(o)))
                    .collect();

                self.config = *config;
                self.custom = custom;
                self.logger
                    .set_new_spec(get_log_spec(&self.config.log_level));

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
                        self.settings.sub_menu = None;

                        if let Some(brightness) = self.settings.brightness.as_mut() {
                            cmd.push(brightness.command(BrightnessCommand::Refresh).map(|event| {
                                crate::app::Message::Settings(
                                    crate::modules::settings::Message::Brightness(
                                        BrightnessMessage::Event(event),
                                    ),
                                )
                            }));
                        }
                    }
                    _ => {}
                };
                cmd.push(self.outputs.toggle_menu(id, menu_type, button_ui_ref));

                Task::batch(cmd)
            }
            Message::CloseMenu(id) => self.outputs.close_menu(id),
            Message::AppLauncher(msg) => {
                if let Some(app_launcher) = self.app_launcher.as_mut() {
                    app_launcher.update(msg);
                }

                Task::none()
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
                            self.outputs.close_menu_if(id, MenuType::Updates),
                        ]),
                    }
                } else {
                    Task::none()
                }
            }
            Message::Clipboard(msg) => {
                if let Some(clipboard) = self.clipboard.as_mut() {
                    clipboard.update(msg);
                }

                Task::none()
            }
            Message::Workspaces(msg) => {
                self.workspaces.update(msg);
                Task::none()
            }
            Message::WindowTitle(msg) => {
                self.window_title.update(msg);
                Task::none()
            }
            Message::SystemInfo(msg) => {
                self.system_info.update(msg);
                Task::none()
            }
            Message::KeyboardLayout(message) => {
                self.keyboard_layout.update(message);
                Task::none()
            }
            Message::KeyboardSubmap(message) => {
                self.keyboard_submap.update(message);
                Task::none()
            }
            Message::Tray(msg) => match self.tray.update(msg) {
                modules::tray::Action::None => Task::none(),
                modules::tray::Action::ToggleMenu(name, id, button_ui_ref) => self
                    .outputs
                    .toggle_menu(id, MenuType::Tray(name), button_ui_ref),
                modules::tray::Action::TrayMenuCommand(task) => task.map(Message::Tray),
                modules::tray::Action::CloseTrayMenu(name) => {
                    self.outputs.close_all_menu_if(MenuType::Tray(name))
                }
            },
            Message::Clock(message) => {
                self.clock.update(message);
                Task::none()
            }
            Message::Privacy(msg) => {
                self.privacy.update(msg);
                Task::none()
            }
            Message::Settings(message) => {
                self.settings
                    .update(message, &self.config.settings, &mut self.outputs)
            }
            Message::OutputEvent((event, wl_output)) => match event {
                iced::event::wayland::OutputEvent::Created(info) => {
                    info!("Output created: {info:?}");
                    let name = info
                        .as_ref()
                        .and_then(|info| info.name.as_deref())
                        .unwrap_or("");

                    self.outputs.add(
                        self.config.appearance.style,
                        &self.config.outputs,
                        self.config.position,
                        name,
                        wl_output,
                        self.config.appearance.scale_factor,
                    )
                }
                iced::event::wayland::OutputEvent::Removed => {
                    info!("Output destroyed");
                    self.outputs.remove(
                        self.config.appearance.style,
                        self.config.position,
                        wl_output,
                        self.config.appearance.scale_factor,
                    )
                }
                _ => Task::none(),
            },
            Message::MediaPlayer(msg) => match self.media_player.update(msg) {
                modules::media_player::Action::None => Task::none(),
                modules::media_player::Action::Command(task) => task.map(Message::MediaPlayer),
            },
        }
    }

    pub fn view(&self, id: Id) -> Element<Message> {
        match self.outputs.has(id) {
            Some(HasOutput::Main) => {
                let [left, center, right] = self.modules_section(id);

                let centerbox = centerbox::Centerbox::new([left, center, right])
                    .spacing(self.theme.space.xxs)
                    .width(Length::Fill)
                    .align_items(Alignment::Center)
                    .height(
                        if self.config.appearance.style == AppearanceStyle::Islands {
                            HEIGHT
                        } else {
                            HEIGHT - 8.
                        } as f32,
                    )
                    .padding(
                        if self.config.appearance.style == AppearanceStyle::Islands {
                            [self.theme.space.xxs, self.theme.space.xxs]
                        } else {
                            [0, 0]
                        },
                    );

                container(centerbox)
                    .style(|t: &Theme| container::Style {
                        background: match self.config.appearance.style {
                            AppearanceStyle::Gradient => Some({
                                let start_color = t
                                    .palette()
                                    .background
                                    .scale_alpha(self.config.appearance.opacity);

                                let start_color = if self.outputs.menu_is_open() {
                                    darken_color(start_color, self.config.appearance.menu.backdrop)
                                } else {
                                    start_color
                                };

                                let end_color = if self.outputs.menu_is_open() {
                                    backdrop_color(self.config.appearance.menu.backdrop)
                                } else {
                                    Color::TRANSPARENT
                                };

                                Gradient::Linear(
                                    Linear::new(Radians(PI))
                                        .add_stop(
                                            0.0,
                                            match self.config.position {
                                                Position::Top => start_color,
                                                Position::Bottom => end_color,
                                            },
                                        )
                                        .add_stop(
                                            1.0,
                                            match self.config.position {
                                                Position::Top => end_color,
                                                Position::Bottom => start_color,
                                            },
                                        ),
                                )
                                .into()
                            }),
                            AppearanceStyle::Solid => Some({
                                let bg = t
                                    .palette()
                                    .background
                                    .scale_alpha(self.config.appearance.opacity);
                                if self.outputs.menu_is_open() {
                                    darken_color(bg, self.config.appearance.menu.backdrop)
                                } else {
                                    bg
                                }
                                .into()
                            }),
                            AppearanceStyle::Islands => {
                                if self.outputs.menu_is_open() {
                                    Some(
                                        backdrop_color(self.config.appearance.menu.backdrop).into(),
                                    )
                                } else {
                                    None
                                }
                            }
                        },
                        ..Default::default()
                    })
                    .into()
            }
            Some(HasOutput::Menu(menu_info)) => match menu_info {
                Some((MenuType::Updates, button_ui_ref)) => {
                    if let Some(updates) = self.updates.as_ref() {
                        self.menu_wrapper(
                            id,
                            updates.menu_view(id, &self.theme).map(Message::Updates),
                            MenuSize::Medium,
                            *button_ui_ref,
                        )
                    } else {
                        Row::new().into()
                    }
                }
                Some((MenuType::Tray(name), button_ui_ref)) => self.menu_wrapper(
                    id,
                    self.tray.menu_view(&self.theme, name).map(Message::Tray),
                    MenuSize::Medium,
                    *button_ui_ref,
                ),
                // Some((MenuType::Settings, button_ui_ref)) => self.menu_wrapper(
                //     id,
                //     self.settings
                //         .menu_view(
                //             id,
                //             &self.config.settings,
                //             self.config.appearance.menu.opacity,
                //             self.config.position,
                //         )
                //         .map(Message::Settings),
                //     MenuSize::Large,
                //     *button_ui_ref,
                // ),
                Some((MenuType::MediaPlayer, button_ui_ref)) => self.menu_wrapper(
                    id,
                    self.media_player
                        .menu_view(&self.theme)
                        .map(Message::MediaPlayer),
                    MenuSize::Large,
                    *button_ui_ref,
                ),
                Some((MenuType::SystemInfo, button_ui_ref)) => self.menu_wrapper(
                    id,
                    self.system_info.menu_view().map(Message::SystemInfo),
                    MenuSize::Large,
                    *button_ui_ref,
                ),
                None => Row::new().into(),
                _ => Row::new().into(),
            },
            None => Row::new().into(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            Subscription::batch(self.modules_subscriptions(&self.config.modules.left)),
            Subscription::batch(self.modules_subscriptions(&self.config.modules.center)),
            Subscription::batch(self.modules_subscriptions(&self.config.modules.right)),
            config::subscription(&self.config_path),
            listen_with(|evt, _, _| match evt {
                iced::Event::PlatformSpecific(iced::event::PlatformSpecific::Wayland(
                    WaylandEvent::Output(event, wl_output),
                )) => {
                    debug!("Wayland event: {event:?}");
                    Some(Message::OutputEvent((event, wl_output)))
                }
                _ => None,
            }),
        ])
    }
}
