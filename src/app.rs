use crate::{
    centerbox,
    config::{self, Config, Orientation},
    get_log_spec,
    menu::{menu_wrapper, Menu, MenuType},
    modules::{
        clock::Clock, launcher, privacy::Privacy, settings::Settings, system_info::SystemInfo,
        title::Title, updates::Updates, workspaces::Workspaces,
    },
    style::ashell_theme,
};
use flexi_logger::LoggerHandle;
use iced::{
    application::Appearance,
    executor, theme,
    widget::{column, row, Column, Row},
    window::Id,
    Alignment, Color, Command, Element, Length, Subscription, Theme,
};
use iced_sctk::multi_window::Application;

pub struct App {
    logger: LoggerHandle,
    config: Config,
    menu: Menu<Message>,
    updates: Updates,
    workspaces: Workspaces,
    window_title: Title,
    system_info: SystemInfo,
    clock: Clock,
    privacy: Privacy,
    pub settings: Settings,
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
    ConfigChanged(Box<Config>),
    CloseMenu,
    OpenLauncher,
    Updates(crate::modules::updates::Message),
    Workspaces(crate::modules::workspaces::Message),
    Title(crate::modules::title::Message),
    SystemInfo(crate::modules::system_info::Message),
    Clock(crate::modules::clock::Message),
    Privacy(crate::modules::privacy::PrivacyMessage),
    Settings(crate::modules::settings::Message),
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
                menu: Menu::init(),
                updates: Updates::new(),
                workspaces: Workspaces::new(),
                window_title: Title::new(),
                system_info: SystemInfo::new(),
                clock: Clock::new(),
                privacy: Privacy::new(),
                settings: Settings::new(),
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
                log::info!("New config: {:?}", config);
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
                    crate::utils::launcher::execute_command(app_launcher_cmd.to_string());
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
            Message::Privacy(message) => self.privacy.update(message, &mut self.menu),
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
                        MenuType::Privacy => self.privacy.menu_view().map(Message::Privacy),
                        MenuType::Settings => self
                            .settings
                            .menu_view(&self.config.settings)
                            .map(Message::Settings),
                    },
                    match menu_type {
                        MenuType::Updates => crate::menu::MenuPosition::Start,
                        MenuType::Privacy => crate::menu::MenuPosition::End,
                        MenuType::Settings => crate::menu::MenuPosition::End,
                    },
                    self.config.orientation,
                )
            } else {
                row!().into()
            }
        } else {
            let start_content = vec![
                self.config
                    .app_launcher_cmd
                    .as_ref()
                    .map(|_| launcher::launcher()),
                self.config.updates.as_ref().map(|_| {
                    self.updates
                        .view(self.config.orientation)
                        .map(Message::Updates)
                }),
                Some(
                    self.workspaces
                        .view(
                            &self.config.appearance.workspace_colors,
                            self.config.orientation,
                        )
                        .map(Message::Workspaces),
                ),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
            let start: Element<Message> = match self.config.orientation {
                Orientation::Horizontal => Row::with_children(start_content)
                    .height(Length::Shrink)
                    .align_items(Alignment::Center)
                    .spacing(4)
                    .into(),
                Orientation::Vertical => Column::with_children(start_content)
                    .height(Length::Shrink)
                    .align_items(Alignment::Center)
                    .spacing(4)
                    .into(),
            };

            let center_content = vec![self.window_title.view().map(|c| c.map(Message::Title))]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            let center: Element<Message> = match self.config.orientation {
                Orientation::Horizontal => Row::with_children(center_content)
                    .height(Length::Shrink)
                    .align_items(Alignment::Center)
                    .spacing(4)
                    .into(),
                Orientation::Vertical => Column::with_children(center_content)
                    .height(Length::Shrink)
                    .align_items(Alignment::Center)
                    .spacing(4)
                    .into(),
            };

            let end_content = 
                vec![
                    self.system_info
                        .view(&self.config.system, self.config.orientation)
                        .map(|c| c.map(Message::SystemInfo)),
                    Some(
                        Row::with_children(
                            vec![
                                Some(
                                    self.clock
                                        .view(&self.config.clock.format)
                                        .map(Message::Clock),
                                ),
                                if self.privacy.applications.is_empty() {
                                    None
                                } else {
                                    Some(self.privacy.view().map(Message::Privacy))
                                },
                                Some(self.settings.view().map(Message::Settings)),
                            ]
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>(),
                        )
                        .into(),
                    ),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            let end: Element<Message> = match self.config.orientation {
                Orientation::Horizontal => Row::with_children(end_content)
                    .height(Length::Shrink)
                    .align_items(Alignment::Center)
                    .spacing(4)
                    .into(),
                Orientation::Vertical => Column::with_children(end_content)
                    .height(Length::Shrink)
                    .align_items(Alignment::Center)
                    .spacing(4)
                    .into(),
            };

            match self.config.orientation {
                Orientation::Horizontal => {
                    centerbox::Centerbox::new([start, center, end])
                        .spacing(4)
                        .padding([0, 4])
                        .width(Length::Fill)
                        .height(Length::Fixed(self.config.size as f32))
                        .align_items(Alignment::Center)
                        .into()
                }
                Orientation::Vertical => column! {
                    start,
                    center,
                    end
                }
                .spacing(4)
                .padding([0, 4])
                .height(Length::Fill)
                .width(Length::Fixed(self.config.size as f32))
                .align_items(Alignment::Center)
                .into(),
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
                Some(self.privacy.subscription().map(Message::Privacy)),
                Some(self.settings.subscription().map(Message::Settings)),
                Some(config::subscription()),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
        )
    }
}
