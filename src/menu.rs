use crate::modules::updates::{Update, UpdateMenu, UpdateMenuMessage};
use iced::wayland::layer_surface::{set_anchor, set_size};
use iced::widget::container;
use iced::{
    theme::Palette,
    wayland::{
        actions::layer_surface::SctkLayerSurfaceSettings,
        layer_surface::{Anchor, KeyboardInteractivity, Layer},
        InitialSurface,
    },
    window::Id,
    Application, Color, Font, Settings, Theme,
};
use std::{cell::RefCell, thread};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

#[derive(Debug, Clone)]
pub enum MenuType {
    Updates(Vec<Update>),
}

#[derive(Debug)]
pub enum MenuInput {
    Open(MenuType),
    MessageToUpdates(Vec<Update>),
    Close,
}

pub enum MenuOutput {
    Close,
}

pub fn create_menu() -> (UnboundedSender<MenuInput>, UnboundedReceiver<MenuOutput>) {
    let (input_tx, input_rx) = mpsc::unbounded_channel();
    let (output_tx, output_rx) = mpsc::unbounded_channel();

    thread::spawn(|| {
        Menu::run(
            Settings::<(UnboundedReceiver<MenuInput>, UnboundedSender<MenuOutput>)> {
                antialiasing: true,
                exit_on_close_request: false,
                flags: (input_rx, output_tx),
                initial_surface: InitialSurface::LayerSurface(SctkLayerSurfaceSettings {
                    id: Id(1),
                    keyboard_interactivity: KeyboardInteractivity::None,
                    namespace: "ashell2-menu".into(),
                    layer: Layer::Overlay,
                    size: Some((None, Some(1))),
                    anchor: Anchor::TOP.union(Anchor::LEFT).union(Anchor::RIGHT),
                    ..Default::default()
                }),
                id: None,
                default_font: Font::default(),
                default_text_size: 14.,
            },
        )
    });

    (input_tx, output_rx)
}

pub struct Menu {
    updates: Option<UpdateMenu>,
    input_rx: RefCell<Option<UnboundedReceiver<MenuInput>>>,
    output_tx: UnboundedSender<MenuOutput>,
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
    OpenMenu(MenuType),
    UpdatesMenu(UpdateMenuMessage),
    CloseRequest,
}

impl Application for Menu {
    type Executor = iced::executor::Default;
    type Theme = Theme;
    type Message = Message;
    type Flags = (UnboundedReceiver<MenuInput>, UnboundedSender<MenuOutput>);

    fn new(
        flags: (UnboundedReceiver<MenuInput>, UnboundedSender<MenuOutput>),
    ) -> (Self, iced::Command<Self::Message>) {
        (
            Menu {
                updates: None,
                input_rx: RefCell::new(Some(flags.0)),
                output_tx: flags.1,
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
        String::from("ashell-menu")
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            Message::CloseRequest => {
                self.output_tx.send(MenuOutput::Close).unwrap();
                iced::Command::batch([
                    set_size(Id(1), None, Some(1)),
                    set_anchor(Id(1), Anchor::TOP.union(Anchor::LEFT).union(Anchor::RIGHT)),
                ])
            }
            Message::None => iced::Command::none(),
            Message::OpenMenu(MenuType::Updates(updates)) => {
                let cmd = iced::Command::batch([
                    set_anchor(
                        Id(1),
                        Anchor::TOP
                            .union(Anchor::LEFT)
                            .union(Anchor::RIGHT)
                            .union(Anchor::BOTTOM),
                    ),
                    set_size(Id(1), None, None),
                ]);

                self.updates = Some(UpdateMenu { updates });

                cmd
            }
            Message::UpdatesMenu(msg) => {
                if let Some(updates) = self.updates.as_mut() {
                    updates.update(msg);
                }
                iced::Command::none()
            }
        }
    }

    fn view(&self, _id: Id) -> iced::Element<'_, Self::Message> {
        if let Some(updates_menu) = self.updates.as_ref() {
            iced::widget::mouse_area(
                container(
                    iced::widget::mouse_area(
                        container(updates_menu.view().map(Message::UpdatesMenu))
                            .height(iced::Length::Shrink)
                            .style(|_: &_| iced::widget::container::Appearance {
                                background: Some(iced::Background::Color(Color::from_rgb(
                                    0.0, 0.0, 0.0,
                                ))),
                                border_radius: 16.0.into(),
                                ..Default::default()
                            })
                            .padding(16),
                    )
                    .on_release(Message::None),
                )
                .padding([0, 8, 8, 8])
                .width(iced::Length::Fill)
                .height(iced::Length::Fill),
            )
            .on_release(Message::CloseRequest)
            .into()
        } else {
            iced::widget::text("should not appear").into()
        }
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        iced::subscription::unfold(
            "menu input receiver",
            self.input_rx.take(),
            move |mut receiver| async move {
                if let Some(menu_message) = receiver.as_mut().unwrap().recv().await {
                    (
                        match menu_message {
                            MenuInput::Open(MenuType::Updates(updates)) => {
                                Message::OpenMenu(MenuType::Updates(updates))
                            }
                            MenuInput::Close => Message::CloseRequest,
                            MenuInput::MessageToUpdates(msg) => {
                                Message::UpdatesMenu(UpdateMenuMessage::UpdatesCheckCompleted(msg))
                            }
                        },
                        receiver,
                    )
                } else {
                    (Message::None, receiver)
                }
            },
        )
    }

    fn close_requested(&self, id: iced::window::Id) -> Self::Message {
        println!("Window {:?} has received a close request", id);
        Message::CloseRequest
    }
}
