use crate::{
    centerbox,
    menu::{menu_wrapper, Menu, MenuType},
    modules::{
        clock::Clock, launcher, privacy::Privacy, settings::Settings, system_info::SystemInfo,
        title::Title, updates::Updates, workspaces::Workspaces,
    },
    style::ashell_theme,
    HEIGHT,
};
use iced::{
    widget::{row, Row},
    window::Id,
    Alignment, Application, Color, Length, Theme,
};

pub struct App {
    menu: Menu,
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
    CloseMenu,
    Launcher(crate::modules::launcher::Message),
    Updates(crate::modules::updates::Message),
    Workspaces(crate::modules::workspaces::Message),
    Title(crate::modules::title::Message),
    SystemInfo(crate::modules::system_info::Message),
    Clock(crate::modules::clock::Message),
    Privacy(crate::modules::privacy::PrivacyMessage),
    Settings(crate::modules::settings::Message),
}

impl Application for App {
    type Executor = iced::executor::Default;
    type Theme = Theme;
    type Message = Message;
    type Flags = ();

    fn new(_: ()) -> (Self, iced::Command<Self::Message>) {
        let (menu, cmd) = Menu::init();
        (
            App {
                menu,
                updates: Updates::new(),
                workspaces: Workspaces::new(),
                window_title: Title::new(),
                system_info: SystemInfo::new(),
                clock: Clock::new(),
                privacy: Privacy::new(),
                settings: Settings::new(),
            },
            cmd,
        )
    }

    fn theme(&self, _id: Id) -> Self::Theme {
        ashell_theme()
    }

    fn style(&self) -> iced::theme::Application {
        fn dark_background(theme: &Theme) -> iced::wayland::Appearance {
            iced::wayland::Appearance {
                background_color: Color::TRANSPARENT,
                text_color: theme.palette().text,
                icon_color: theme.palette().text,
            }
        }

        iced::theme::Application::from(dark_background as fn(&Theme) -> _)
    }

    fn title(&self, _id: Id) -> String {
        String::from("ashell")
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            Message::None => iced::Command::none(),
            Message::CloseMenu => self.menu.close(),
            Message::Updates(message) => self
                .updates
                .update(message, &mut self.menu)
                .map(Message::Updates),
            Message::Launcher(_) => {
                crate::utils::launcher::launch_rofi();
                iced::Command::none()
            }
            Message::Workspaces(msg) => {
                self.workspaces.update(msg);

                iced::Command::none()
            }
            Message::Title(message) => {
                self.window_title.update(message);
                iced::Command::none()
            }
            Message::SystemInfo(message) => {
                self.system_info.update(message);
                iced::Command::none()
            }
            Message::Clock(message) => {
                self.clock.update(message);
                iced::Command::none()
            }
            Message::Privacy(message) => self
                .privacy
                .update(message, &mut self.menu)
                .map(Message::Privacy),
            Message::Settings(message) => self
                .settings
                .update(message, &mut self.menu)
                .map(Message::Settings),
        }
    }

    fn view(&self, id: Id) -> iced::Element<'_, Self::Message> {
        if id == self.menu.get_id() {
            if let Some(menu_type) = self.menu.get_menu_type() {
                menu_wrapper(
                    match menu_type {
                        MenuType::Updates => self.updates.menu_view().map(Message::Updates),
                        MenuType::Privacy => self.privacy.menu_view().map(Message::Privacy),
                        MenuType::Settings => self.settings.menu_view().map(Message::Settings),
                    },
                    match menu_type {
                        MenuType::Updates => crate::menu::MenuPosition::Left,
                        MenuType::Privacy => crate::menu::MenuPosition::Right,
                        MenuType::Settings => crate::menu::MenuPosition::Right,
                    },
                )
            } else {
                row!().into()
            }
        } else {
            let left = row!(
                launcher::launcher().map(Message::Launcher),
                self.updates.view().map(Message::Updates),
                self.workspaces.view().map(Message::Workspaces)
            )
            .height(Length::Shrink)
            .align_items(Alignment::Center)
            .spacing(4);

            let mut center = row!().spacing(4);
            if let Some(title) = self.window_title.view() {
                center = center.push(title.map(Message::Title));
            }

            let right = row!(
                self.system_info.view().map(Message::SystemInfo),
                Row::with_children(
                    vec!(
                        Some(self.clock.view().map(Message::Clock)),
                        if self.privacy.applications.is_empty() {
                            None
                        } else {
                            Some(self.privacy.view().map(Message::Privacy))
                        },
                        Some(self.settings.view().map(Message::Settings))
                    )
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                )
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

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        iced::Subscription::batch(vec![
            self.updates.subscription().map(Message::Updates),
            self.workspaces.subscription().map(Message::Workspaces),
            self.window_title.subscription().map(Message::Title),
            self.system_info.subscription().map(Message::SystemInfo),
            self.clock.subscription().map(Message::Clock),
            self.privacy.subscription().map(Message::Privacy),
            self.settings.subscription().map(Message::Settings),
        ])
    }
}
