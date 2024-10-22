use crate::{
    centerbox,
    config::{self, Config},
    get_log_spec,
    menu::{menu_wrapper, Menu, MenuPosition},
    modules::{
        self, clipboard, clock::Clock, keyboard_layout::KeyboardLayout,
        keyboard_submap::KeyboardSubmap, launcher, privacy::PrivacyMessage, settings::Settings,
        system_info::SystemInfo, title::Title, updates::Updates, workspaces::Workspaces,
    },
    outputs::Outputs,
    services::{privacy::PrivacyService, ReadOnlyService, ServiceEvent},
    style::ashell_theme,
    utils, HEIGHT,
};
use flexi_logger::LoggerHandle;
use iced::{
    daemon::Appearance,
    event::listen_with,
    platform_specific::shell::commands::layer_surface::{
        get_layer_surface, Anchor, KeyboardInteractivity, Layer,
    },
    runtime::platform_specific::wayland::layer_surface::{IcedOutput, SctkLayerSurfaceSettings},
    widget::{row, Row},
    window::Id,
    Alignment, Color, Element, Length, Subscription, Task, Theme,
};
use log::{debug, info};

pub struct App {
    logger: LoggerHandle,
    config: Config,
    outputs: Outputs,
    updates: Updates,
    workspaces: Workspaces,
    window_title: Title,
    system_info: SystemInfo,
    keyboard_layout: KeyboardLayout,
    keyboard_submap: KeyboardSubmap,
    clock: Clock,
    privacy: Option<PrivacyService>,
    pub settings: Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuType {
    Updates,
    Settings,
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
    ConfigChanged(Box<Config>),
    CloseMenu,
    OpenLauncher,
    OpenClipboard,
    Updates(modules::updates::Message),
    Workspaces(modules::workspaces::Message),
    Title(modules::title::Message),
    SystemInfo(modules::system_info::Message),
    KeyboardLayout(modules::keyboard_layout::Message),
    KeyboardSubmap(modules::keyboard_submap::Message),
    Clock(modules::clock::Message),
    Privacy(modules::privacy::PrivacyMessage),
    Settings(modules::settings::Message),
    // WaylandEvent(WaylandEvent),
}

impl App {
    pub fn new((logger, config): (LoggerHandle, Config)) -> impl FnOnce() -> (Self, Task<Message>) {
        || {
            (
                App {
                    logger,
                    config,
                    outputs: Outputs::default(),
                    updates: Updates::default(),
                    workspaces: Workspaces::default(),
                    window_title: Title::default(),
                    system_info: SystemInfo::default(),
                    keyboard_layout: KeyboardLayout::default(),
                    keyboard_submap: KeyboardSubmap::default(),
                    clock: Clock::default(),
                    privacy: None,
                    settings: Settings::default(),
                },
                Task::none(),
            )
        }
    }

    pub fn title(&self, _id: Id) -> String {
        String::from("ashell")
    }

    pub fn theme(&self, _id: Id) -> Theme {
        ashell_theme(&self.config.appearance)
    }

    pub fn style(&self, theme: &Theme) -> Appearance {
        Appearance {
            background_color: Color::TRANSPARENT,
            text_color: theme.palette().text,
            icon_color: theme.palette().text,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::None => Task::none(),
            Message::ConfigChanged(config) => {
                info!("New config: {:?}", config);
                self.config = *config;
                self.logger
                    .set_new_spec(get_log_spec(&self.config.log_level));
                Task::none()
            }
            Message::CloseMenu => {
                // self.menu.close(),
                Task::none()
            }
            Message::Updates(message) => {
                if let Some(updates_config) = self.config.updates.as_ref() {
                    // self.updates.update(message, updates_config, &mut self.menu)
                    Task::none()
                } else {
                    Task::none()
                }
            }
            Message::OpenLauncher => {
                if let Some(app_launcher_cmd) = self.config.app_launcher_cmd.as_ref() {
                    utils::launcher::execute_command(app_launcher_cmd.to_string());
                }
                Task::none()
            }
            Message::OpenClipboard => {
                if let Some(clipboard_cmd) = self.config.clipboard_cmd.as_ref() {
                    utils::launcher::execute_command(clipboard_cmd.to_string());
                }
                Task::none()
            }
            Message::Workspaces(msg) => {
                self.workspaces.update(msg);

                Task::none()
            }
            Message::Title(message) => {
                self.window_title
                    .update(message, self.config.truncate_title_after_length);
                Task::none()
            }
            Message::SystemInfo(message) => {
                self.system_info.update(message);
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
            Message::Clock(message) => {
                self.clock.update(message);
                Task::none()
            }
            Message::Privacy(msg) => match msg {
                PrivacyMessage::Event(event) => match event {
                    ServiceEvent::Init(service) => {
                        self.privacy = Some(service);
                        Task::none()
                    }
                    ServiceEvent::Update(data) => {
                        if let Some(privacy) = self.privacy.as_mut() {
                            privacy.update(data);
                        }
                        Task::none()
                    }
                    ServiceEvent::Error(_) => Task::none(),
                },
            },
            Message::Settings(message) => {
                // self.settings
                //     .update(message, &self.config.settings, &mut self.menu)
                Task::none()
            } // Message::WaylandEvent(event) => match event {
              //     WaylandEvent::Output(event, wl_output) => match event {
              //         iced::event::wayland::OutputEvent::Created(info) => {
              //             info!("Output created: {:?}", info);
              //             self.outputs.add(
              //                 &self.config.outputs,
              //                 self.config.position,
              //                 info,
              //                 wl_output,
              //             )
              //         }
              //         iced::event::wayland::OutputEvent::Removed => {
              //             info!("Output destroyed");
              //             self.outputs.remove(self.config.position, wl_output)
              //         }
              //         _ => Task::none(),
              //     },
              //     _ => Task::none(),
              // },
        }
    }

    pub fn view(&self, id: Id) -> Element<Message> {
        // if self.menu.is_menu(id) {
        //     match self.menu.get_menu_type_to_render(id) {
        //         Some(MenuType::Updates) => menu_wrapper(
        //             self.updates.menu_view().map(Message::Updates),
        //             MenuPosition::Left,
        //             self.config.position,
        //         ),
        //         Some(MenuType::Settings) => menu_wrapper(
        //             self.settings
        //                 .menu_view(&self.config.settings)
        //                 .map(Message::Settings),
        //             MenuPosition::Right,
        //             self.config.position,
        //         ),
        //         None => Row::new().into(),
        //     }
        // } else {
        let left = Row::new()
            .push_maybe(
                self.config
                    .app_launcher_cmd
                    .as_ref()
                    .map(|_| launcher::launcher()),
            )
            .push_maybe(
                self.config
                    .clipboard_cmd
                    .as_ref()
                    .map(|_| clipboard::clipboard()),
            )
            .push_maybe(
                self.config
                    .updates
                    .as_ref()
                    .map(|_| self.updates.view().map(Message::Updates)),
            )
            .push(
                self.workspaces
                    .view(
                        &self.config.appearance.workspace_colors,
                        self.config.appearance.special_workspace_colors.as_deref(),
                    )
                    .map(Message::Workspaces),
            )
            .height(Length::Shrink)
            .align_y(Alignment::Center)
            .spacing(4);

        let center = Row::new()
            .push_maybe(self.window_title.view().map(|v| v.map(Message::Title)))
            .spacing(4);

        let right = Row::new()
            .push_maybe(
                self.system_info
                    .view(&self.config.system)
                    .map(|c| c.map(Message::SystemInfo)),
            )
            .push_maybe(
                self.keyboard_submap
                    .view(&self.config.keyboard.submap)
                    .map(|l| l.map(Message::KeyboardSubmap)),
            )
            .push_maybe(
                self.keyboard_layout
                    .view(&self.config.keyboard.layout)
                    .map(|l| l.map(Message::KeyboardLayout)),
            )
            .push(
                Row::new()
                    .push(
                        self.clock
                            .view(&self.config.clock.format)
                            .map(Message::Clock),
                    )
                    .push_maybe(
                        self.privacy
                            .as_ref()
                            .and_then(|privacy| privacy.view())
                            .map(|e| e.map(Message::Privacy)),
                    )
                    .push(self.settings.view().map(Message::Settings)),
            )
            .spacing(4);

        centerbox::Centerbox::new([left.into(), center.into(), right.into()])
            .spacing(4)
            .padding([0, 4])
            .width(Length::Fill)
            .height(Length::Fixed(HEIGHT as f32))
            .align_items(Alignment::Center)
            .into()
        // }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(
            vec![
                self.config.updates.as_ref().map(|updates_config| {
                    self.updates
                        .subscription(updates_config)
                        .map(Message::Updates)
                }),
                Some(self.workspaces.subscription().map(Message::Workspaces)),
                Some(self.window_title.subscription().map(Message::Title)),
                Some(self.system_info.subscription().map(Message::SystemInfo)),
                Some(
                    self.keyboard_layout
                        .subscription()
                        .map(Message::KeyboardLayout),
                ),
                Some(
                    self.keyboard_submap
                        .subscription()
                        .map(Message::KeyboardSubmap),
                ),
                Some(self.clock.subscription().map(Message::Clock)),
                Some(
                    PrivacyService::subscribe().map(|e| Message::Privacy(PrivacyMessage::Event(e))),
                ),
                Some(self.settings.subscription().map(Message::Settings)),
                Some(config::subscription()),
                // Some(listen_with(|evt, _, _| {
                //     if let iced::Event::PlatformSpecific(iced::event::PlatformSpecific::Wayland(
                //         evt,
                //     )) = evt
                //     {
                //         if matches!(evt, WaylandEvent::Output(_, _)) {
                //             debug!("Wayland event: {:?}", evt);
                //             Some(Message::WaylandEvent(evt))
                //         } else {
                //             None
                //         }
                //     } else {
                //         None
                //     }
                // })),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
        )
    }
}
