use crate::{
    centerbox,
    config::{self, Config},
    get_log_spec,
    menu::{menu_wrapper, Menu, MenuPosition, MenuType},
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
    application::Appearance,
    executor, theme,
    widget::{row, Row},
    window::Id,
    Alignment, Color, Command, Element, Length, Subscription, Theme,
};
use iced_sctk::multi_window::Application;
use log::info;

pub struct App {
    logger: LoggerHandle,
    config: Config,
    menu: Menu,
    updates: Updates,
    workspaces: Workspaces,
    window_title: Title,
    system_info: SystemInfo,
    clock: Clock,
    privacy: Option<PrivacyService>,
    pub settings: Settings,
}

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
}

impl Application for App {
    type Executor = executor::Default;
    type Theme = Theme;
    type Message = Message;
    type Flags = (LoggerHandle, Config);

    fn new((logger, config): (LoggerHandle, Config)) -> (Self, Command<Self::Message>) {
        (
            App {
                logger,
                config,
                menu: Menu::default(),
                updates: Updates::default(),
                workspaces: Workspaces::default(),
                window_title: Title::default(),
                system_info: SystemInfo::default(),
                clock: Clock::default(),
                privacy: None,
                settings: Settings::default(),
            },
            Command::none(),
        )
    }

    fn theme(&self, _id: Id) -> Self::Theme {
        ashell_theme(&self.config.appearance)
    }

    fn title(&self, _id: Id) -> String {
        String::from("ashell")
    }

    fn style(&self) -> theme::Application {
        fn dark_background(theme: &Theme) -> Appearance {
            Appearance {
                background_color: Color::TRANSPARENT,
                text_color: theme.palette().text,
            }
        }

        theme::Application::custom(dark_background as fn(&Theme) -> _)
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::None => Command::none(),
            Message::ConfigChanged(config) => {
                info!("New config: {:?}", config);
                self.config = *config;
                self.logger
                    .set_new_spec(get_log_spec(self.config.log_level));
                Command::none()
            }
            Message::CloseMenu => self.menu.close(),
            Message::Updates(message) => {
                if let Some(updates_config) = self.config.updates.as_ref() {
                    self.updates.update(message, updates_config, &mut self.menu)
                } else {
                    Command::none()
                }
            }
            Message::OpenLauncher => {
                if let Some(app_launcher_cmd) = self.config.app_launcher_cmd.as_ref() {
                    utils::launcher::execute_command(app_launcher_cmd.to_string());
                }
                Command::none()
            }
            Message::Workspaces(msg) => {
                self.workspaces.update(msg);

                Command::none()
            }
            Message::Title(message) => {
                self.window_title
                    .update(message, self.config.truncate_title_after_length);
                Command::none()
            }
            Message::SystemInfo(message) => {
                self.system_info.update(message);
                Command::none()
            }
            Message::Clock(message) => {
                self.clock.update(message);
                Command::none()
            }
            Message::Privacy(msg) => match msg {
                PrivacyMessage::Event(event) => match event {
                    ServiceEvent::Init(service) => {
                        self.privacy = Some(service);
                        Command::none()
                    }
                    ServiceEvent::Update(data) => {
                        if let Some(privacy) = self.privacy.as_mut() {
                            privacy.update(data);
                        }
                        Command::none()
                    }
                    ServiceEvent::Error(_) => Command::none(),
                },
            },
            Message::Settings(message) => {
                self.settings
                    .update(message, &self.config.settings, &mut self.menu)
            }
        }
    }

    fn view(&self, id: Id) -> Element<'_, Self::Message> {
        if Some(id) == self.menu.get_id() {
            if let Some(menu_type) = self.menu.get_menu_type() {
                menu_wrapper(
                    match menu_type {
                        MenuType::Updates => self.updates.menu_view().map(Message::Updates),
                        MenuType::Settings => self
                            .settings
                            .menu_view(&self.config.settings)
                            .map(Message::Settings),
                    },
                    match menu_type {
                        MenuType::Updates => MenuPosition::Left,
                        MenuType::Settings => MenuPosition::Right,
                    },
                    self.config.position,
                )
            } else {
                row!().into()
            }
        } else {
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
                .align_items(Alignment::Center)
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
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
        )
    }
}
