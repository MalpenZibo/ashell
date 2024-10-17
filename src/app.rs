use std::collections::HashMap;

use crate::{
    centerbox,
    config::{self, Config},
    get_log_spec,
    menu::{self, menu_wrapper, MenuPosition},
    modules::{
        self, clock::Clock, launcher, privacy::PrivacyMessage, settings::Settings,
        system_info::SystemInfo, title::Title, updates::Updates, workspaces::Workspaces,
    },
    services::{privacy::PrivacyService, ReadOnlyService, ServiceEvent},
    style::ashell_theme,
    utils, HEIGHT,
};
use flexi_logger::LoggerHandle;
use iced::{
    executor, widget::Row, window::Id, Alignment, Color, Element, Length, Subscription, Task, Theme,
};
use iced_layershell::{to_layer_message, MultiApplication};
use log::info;

pub struct App {
    logger: LoggerHandle,
    config: Config,
    updates: Updates,
    workspaces: Workspaces,
    window_title: Title,
    system_info: SystemInfo,
    clock: Clock,
    privacy: Option<PrivacyService>,
    pub settings: Settings,
    ids: HashMap<iced::window::Id, WindowInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowInfo {
    Updates,
    Settings,
}

#[to_layer_message(multi, info_name = "WindowInfo")]
#[derive(Debug, Clone)]
pub enum Message {
    None,
    ConfigChanged(Box<Config>),
    CloseMenu,
    OpenLauncher,
    Updates(modules::updates::Message),
    Workspaces(modules::workspaces::Message),
    Title(modules::title::Message),
    SystemInfo(modules::system_info::Message),
    Clock(modules::clock::Message),
    Privacy(modules::privacy::PrivacyMessage),
    Settings(modules::settings::Message),
    IcedEvent,
}

impl MultiApplication for App {
    type Executor = executor::Default;
    type Theme = Theme;
    type Message = Message;
    type Flags = (LoggerHandle, Config);
    type WindowInfo = WindowInfo;

    fn new((logger, config): (LoggerHandle, Config)) -> (Self, Task<Self::Message>) {
        (
            App {
                logger,
                config,
                updates: Updates::default(),
                workspaces: Workspaces::default(),
                window_title: Title::default(),
                system_info: SystemInfo::default(),
                clock: Clock::default(),
                privacy: None,
                settings: Settings::default(),
                ids: HashMap::new(),
            },
            Task::none(),
        )
    }

    fn id_info(&self, id: iced::window::Id) -> Option<Self::WindowInfo> {
        self.ids.get(&id).cloned()
    }

    fn set_id_info(&mut self, id: iced::window::Id, info: Self::WindowInfo) {
        self.ids.insert(id, info);
    }

    fn remove_id(&mut self, id: iced::window::Id) {
        self.ids.remove(&id);
    }

    fn theme(&self) -> Self::Theme {
        ashell_theme(&self.config.appearance)
    }

    fn namespace(&self) -> String {
        String::from("ashell-testone")
    }

    fn style(&self, theme: &Self::Theme) -> iced_layershell::Appearance {
        iced_layershell::Appearance {
            background_color: Color::TRANSPARENT,
            text_color: theme.palette().text,
        }
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::None => Task::none(),
            Message::ConfigChanged(config) => {
                info!("New config: {:?}", config);
                self.config = *config;
                self.logger
                    .set_new_spec(get_log_spec(self.config.log_level));
                Task::none()
            }
            Message::CloseMenu => {
                let mut tasks = Vec::with_capacity(self.ids.len());
                for (id, _) in self.ids.iter() {
                    tasks.push(menu::close_menu(*id));
                }

                Task::batch(tasks)
            }
            Message::Updates(message) => {
                if let Some(updates_config) = self.config.updates.as_ref() {
                    self.updates
                        .update(message, updates_config, self.ids.iter_mut().next())
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
                self.settings
                    .update(message, &self.config.settings, self.ids.iter_mut().next())
            }
            Message::IcedEvent => Task::none(),
            _ => Task::none(),
        }
    }

    fn view(&self, id: Id) -> Element<'_, Self::Message> {
        match self.id_info(id) {
            Some(WindowInfo::Updates) => {
                menu_wrapper(
                    self.updates.menu_view().map(Message::Updates),
                    MenuPosition::Left,
                    self.config.position,
                )
            }
            Some(WindowInfo::Settings) => {
                menu_wrapper(
                    self.settings
                        .menu_view(&self.config.settings)
                        .map(Message::Settings),
                    MenuPosition::Right,
                    self.config.position,
                )
            }
            None => {
                let left = Row::new()
                    .push_maybe(
                        self.config
                            .app_launcher_cmd
                            .as_ref()
                            .map(|_| launcher::launcher()),
                    )
                    .push_maybe(
                        self.config
                            .updates
                            .as_ref()
                            .map(|_| self.updates.view().map(Message::Updates)),
                    )
                    .push(
                        self.workspaces
                            .view(&self.config.appearance.workspace_colors)
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
            }
        }
    }

    fn subscription(&self) -> Subscription<Self::Message> {
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
                Some(self.clock.subscription().map(Message::Clock)),
                Some(
                    PrivacyService::subscribe().map(|e| Message::Privacy(PrivacyMessage::Event(e))),
                ),
                Some(self.settings.subscription().map(Message::Settings)),
                Some(config::subscription()),
                Some(iced::event::listen().map(|_| Message::IcedEvent)),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
        )
    }
}
