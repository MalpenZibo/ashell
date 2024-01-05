use crate::{
    centerbox,
    menu::{MenuInput, MenuOutput, MenuType},
    modules::{
        clock::Clock,
        launcher,
        settings::Settings,
        system_info::SystemInfo,
        title::Title,
        updates::{Update, UpdateMenuOutput, Updates},
        workspaces::Workspaces,
    },
    style::ashell_theme,
};
use iced::{
    widget::{container, row, text},
    window::Id,
    Alignment, Application, Color, Length, Theme,
};
use std::cell::RefCell;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub enum MenuRequest<'a> {
    Updates(&'a Vec<Update>),
    NotifyNewUpdates(&'a Vec<Update>),
    Settings,
}

#[derive(Eq, PartialEq, Debug)]
pub enum OpenMenu {
    Updates,
    Settings,
}

pub struct App {
    menu_sender: UnboundedSender<MenuInput>,
    menu_receiver: RefCell<Option<UnboundedReceiver<MenuOutput>>>,
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
    MenuClosed,
    LauncherMessage(crate::modules::launcher::Message),
    UpdatesMessage(crate::modules::updates::Message),
    WorkspacesMessage(crate::modules::workspaces::Message),
    TitleMessage(crate::modules::title::Message),
    SystemInfoMessage(crate::modules::system_info::Message),
    ClockMessage(crate::modules::clock::Message),
    SettingsMessage(crate::modules::settings::Message),
    CloseRequest,
}

impl Application for App {
    type Executor = iced::executor::Default;
    type Theme = Theme;
    type Message = Message;
    type Flags = (UnboundedSender<MenuInput>, UnboundedReceiver<MenuOutput>);

    fn new(
        flags: (UnboundedSender<MenuInput>, UnboundedReceiver<MenuOutput>),
    ) -> (Self, iced::Command<Self::Message>) {
        (
            App {
                menu_sender: flags.0,
                menu_receiver: RefCell::new(Some(flags.1)),
                menu_type: None,
                updates: Updates::new(),
                workspaces: Workspaces::new(),
                window_title: Title::new(),
                system_info: SystemInfo::new(),
                clock: Clock::new(),
                settings: Settings::new(),
            },
            iced::Command::none(),
        )
    }

    fn theme(&self, id: iced::window::Id) -> Self::Theme {
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

    fn title(&self, id: iced::window::Id) -> String {
        String::from("ashell")
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            Message::None => {}
            Message::MenuClosed => {
                self.menu_type = None;
            }
            Message::UpdatesMessage(message) => {
                let response = self.updates.update(message);

                match (&self.menu_type, response) {
                    (Some(OpenMenu::Updates), Some(MenuRequest::Updates(_))) => {
                        self.menu_type = None;
                        self.menu_sender.send(MenuInput::Close).unwrap();
                    }
                    (_, Some(MenuRequest::Updates(updates))) => {
                        self.menu_type = Some(OpenMenu::Updates);
                        self.menu_sender
                            .send(MenuInput::Open(MenuType::Updates(updates.clone())))
                            .unwrap();
                    }
                    (Some(OpenMenu::Updates), Some(MenuRequest::NotifyNewUpdates(updates))) => {
                        self.menu_sender
                            .send(MenuInput::MessageToUpdates(updates.clone()))
                            .unwrap();
                    }
                    _ => {}
                };
            }
            Message::LauncherMessage(_) => {
                crate::utils::launcher::launch_rofi();
            }
            Message::WorkspacesMessage(msg) => self.workspaces.update(msg),
            Message::TitleMessage(message) => {
                self.window_title.update(message);
            }
            Message::SystemInfoMessage(message) => {
                self.system_info.update(message);
            }
            Message::ClockMessage(message) => {
                self.clock.update(message);
            }
            Message::SettingsMessage(message) => {
                if let Some(OpenMenu::Settings) = &self.menu_type {
                    if let Some(message) = match message.clone() {
                        crate::modules::settings::Message::Battery(battery) => {
                            Some(MenuInput::MessageToSettings(
                                crate::menu::SettingsInputMessage::Battery(battery),
                            ))
                        }
                        crate::modules::settings::Message::Audio(msg) => {
                            Some(MenuInput::MessageToSettings(
                                crate::menu::SettingsInputMessage::Audio(msg),
                            ))
                        }
                        _ => None,
                    } {
                        self.menu_sender.send(message).unwrap();
                    }
                }
                let response = self.settings.update(message);

                if let (_, Some(MenuRequest::Settings)) = (&self.menu_type, response) {
                    self.menu_type = Some(OpenMenu::Settings);
                    self.menu_sender
                        .send(MenuInput::Open(MenuType::Settings((
                            self.settings.battery_data,
                            self.settings.sinks.clone(),
                        ))))
                        .unwrap();
                }
            }
            Message::CloseRequest => {
                println!("Close request received");
            }
        }

        iced::Command::none()
    }

    fn view(&self, _id: Id) -> iced::Element<'_, Self::Message> {
        let left = row!(
            launcher::launcher().map(Message::LauncherMessage),
            self.updates.view().map(Message::UpdatesMessage),
            self.workspaces.view().map(Message::WorkspacesMessage)
        )
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

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        iced::Subscription::batch(vec![
            iced::subscription::unfold(
                "menu output receiver",
                self.menu_receiver.take(),
                move |mut receiver| async move {
                    if let Some(menu_message) = receiver.as_mut().unwrap().recv().await {
                        (
                            match menu_message {
                                MenuOutput::MessageFromUpdates(
                                    UpdateMenuOutput::UpdateFinished,
                                ) => Message::UpdatesMessage(
                                    crate::modules::updates::Message::UpdateFinished,
                                ),

                                MenuOutput::MessageFromUpdates(
                                    UpdateMenuOutput::UpdatesCheckInit,
                                ) => Message::UpdatesMessage(
                                    crate::modules::updates::Message::UpdatesCheckInit,
                                ),

                                MenuOutput::MessageFromUpdates(
                                    UpdateMenuOutput::UpdatesCheckCompleted(updates),
                                ) => Message::UpdatesMessage(
                                    crate::modules::updates::Message::UpdatesRefreshFromMenu(
                                        updates,
                                    ),
                                ),
                                MenuOutput::Close => Message::MenuClosed,
                            },
                            receiver,
                        )
                    } else {
                        (Message::None, receiver)
                    }
                },
            ),
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
