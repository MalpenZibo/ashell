use crate::{
    centerbox,
    menu::{close_menu, create_menu, menu_wrapper},
    modules::{
        clock::Clock, launcher, settings::Settings, system_info::SystemInfo, title::Title,
        updates::Updates, workspaces::Workspaces,
    },
    style::ashell_theme,
};
use iced::{
    widget::{column, container, row},
    window::Id,
    Alignment, Application, Color, Length, Theme,
};

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum OpenMenu {
    Updates,
    Settings,
}

pub struct App {
    menu_id: Id,
    menu_type: Option<OpenMenu>,
    updates: Updates,
    workspaces: Workspaces,
    window_title: Title,
    system_info: SystemInfo,
    clock: Clock,
    pub settings: Settings,
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
    CloseMenu,
    LauncherMessage(crate::modules::launcher::Message),
    UpdatesMessage(crate::modules::updates::Message),
    WorkspacesMessage(crate::modules::workspaces::Message),
    TitleMessage(crate::modules::title::Message),
    SystemInfoMessage(crate::modules::system_info::Message),
    ClockMessage(crate::modules::clock::Message),
    SettingsMessage(crate::modules::settings::Message),
}

impl Application for App {
    type Executor = iced::executor::Default;
    type Theme = Theme;
    type Message = Message;
    type Flags = ();

    fn new(_: ()) -> (Self, iced::Command<Self::Message>) {
        let (menu_id, cmd) = create_menu();

        (
            App {
                menu_id,
                menu_type: None,
                updates: Updates::new(),
                workspaces: Workspaces::new(),
                window_title: Title::new(),
                system_info: SystemInfo::new(),
                clock: Clock::new(),
                settings: Settings::new(),
            },
            cmd,
        )
    }

    fn theme(&self, _id: iced::window::Id) -> Self::Theme {
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

    fn title(&self, _id: iced::window::Id) -> String {
        String::from("ashell")
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            Message::None => iced::Command::none(),
            Message::CloseMenu => {
                self.menu_type = None;

                close_menu(self.menu_id)
            }
            Message::UpdatesMessage(message) => self
                .updates
                .update(message, self.menu_id, &mut self.menu_type)
                .map(Message::UpdatesMessage),
            Message::LauncherMessage(_) => {
                crate::utils::launcher::launch_rofi();
                iced::Command::none()
            }
            Message::WorkspacesMessage(msg) => {
                self.workspaces.update(msg);

                iced::Command::none()
            }
            Message::TitleMessage(message) => {
                self.window_title.update(message);
                iced::Command::none()
            }
            Message::SystemInfoMessage(message) => {
                self.system_info.update(message);
                iced::Command::none()
            }
            Message::ClockMessage(message) => {
                self.clock.update(message);
                iced::Command::none()
            }
            Message::SettingsMessage(message) => self
                .settings
                .update(message, self.menu_id, &mut self.menu_type)
                .map(Message::SettingsMessage),
        }
    }

    fn view(&self, id: Id) -> iced::Element<'_, Self::Message> {
        match self.menu_type {
            Some(menu_type) if self.menu_id == id => menu_wrapper(
                match menu_type {
                    OpenMenu::Updates => self.updates.menu_view().map(Message::UpdatesMessage),
                    OpenMenu::Settings => self.settings.menu_view().map(Message::SettingsMessage),
                },
                match menu_type {
                    OpenMenu::Updates => crate::menu::MenuPosition::Left,
                    OpenMenu::Settings => crate::menu::MenuPosition::Right,
                },
            ),
            _ if id == Id::MAIN => {
                let left = row!(
                    launcher::launcher().map(Message::LauncherMessage),
                    self.updates.view().map(Message::UpdatesMessage),
                    self.workspaces.view().map(Message::WorkspacesMessage)
                )
                .align_items(Alignment::Center)
                .spacing(4);

                let mut center = row!().spacing(4);
                if let Some(title) = self.window_title.view() {
                    center = center.push(title.map(Message::TitleMessage));
                }

                let right = row!(
                    self.system_info.view().map(Message::SystemInfoMessage),
                    row!(
                        self.clock.view().map(Message::ClockMessage),
                        self.settings.view().map(Message::SettingsMessage)
                    )
                )
                .spacing(4);

                centerbox::Centerbox::new([left.into(), center.into(), right.into()])
                    .spacing(4)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_items(Alignment::Center)
                    .padding(4)
                    .into()
            }
            _ => row!().into(),
        }
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        iced::Subscription::batch(vec![
            self.updates.subscription().map(Message::UpdatesMessage),
            self.workspaces
                .subscription()
                .map(Message::WorkspacesMessage),
            self.window_title.subscription().map(Message::TitleMessage),
            self.system_info
                .subscription()
                .map(Message::SystemInfoMessage),
            self.clock.subscription().map(Message::ClockMessage),
            self.settings.subscription().map(Message::SettingsMessage),
        ])
    }
}
