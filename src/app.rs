use std::cell::RefCell;

use crate::{
    centerbox,
    menu::{MenuInput, MenuOutput, MenuType},
    modules::{launcher, title::Title, updates::Updates},
};
use iced::{theme::Palette, widget::row, window::Id, Alignment, Application, Color, Length, Theme};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub struct App {
    menu_sender: UnboundedSender<MenuInput>,
    menu_receiver: RefCell<Option<UnboundedReceiver<MenuOutput>>>,
    menu_is_open: bool,
    updates: Updates,
    window_title: Title,
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
    MenuClosed,
    LauncherMessage(crate::modules::launcher::Message),
    UpdatesMessage(crate::modules::updates::Message),
    TitleMessage(crate::modules::title::Message),
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
                menu_is_open: false,
                updates: Updates::new(),
                window_title: Title::new(),
            },
            iced::Command::none(),
        )
    }

    fn theme(&self) -> Self::Theme {
        Theme::custom(Palette {
            background: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.0),
            text: Color::BLACK,
            primary: Color::from_rgb(0.5, 0.5, 0.0),
            success: Color::from_rgb(0.0, 1.0, 0.0),
            danger: Color::from_rgb(1.0, 0.0, 0.0),
        })
    }

    fn title(&self) -> String {
        String::from("ashell")
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            Message::None => {}
            Message::MenuClosed => {
                self.menu_is_open = false;
            }
            Message::UpdatesMessage(crate::modules::updates::Message::ToggleMenu) => {
                if self.menu_is_open {
                    self.menu_is_open = false;
                    self.menu_sender.send(MenuInput::Close).unwrap();
                } else {
                    self.menu_sender
                        .send(MenuInput::Open(MenuType::Updates))
                        .unwrap();
                    self.menu_is_open = true;
                }
            }
            Message::UpdatesMessage(crate::modules::updates::Message::InternalMessage(message)) => {
                self.updates.update(message);
            }
            Message::LauncherMessage(_) => {
                crate::utils::launcher::launch_rofi();
            }
            Message::TitleMessage(message) => {
                self.window_title.update(message);
            }
            Message::CloseRequest => {
                println!("Close request received");
            }
        }

        iced::Command::none()
    }

    fn view(&self, id: Id) -> iced::Element<'_, Self::Message> {
        let left = row!(
            launcher::launcher().map(Message::LauncherMessage),
            self.updates.view().map(Message::UpdatesMessage),
        )
        .spacing(4);

        let mut center = row!().spacing(4);
        if let Some(title) = self.window_title.view() {
            center = center.push(title.map(Message::TitleMessage));
        }

        let right = row!().spacing(4);

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
                                MenuOutput::Close => Message::MenuClosed,
                            },
                            receiver,
                        )
                    } else {
                        (Message::None, receiver)
                    }
                },
            ),
            self.updates.subscription().map(|msg| {
                Message::UpdatesMessage(crate::modules::updates::Message::InternalMessage(msg))
            }),
            self.window_title.subscription().map(Message::TitleMessage),
        ])
    }

    fn close_requested(&self, id: iced::window::Id) -> Self::Message {
        println!("Window {:?} has received a close request", id);
        Message::CloseRequest
    }
}
