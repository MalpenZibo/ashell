use crate::modules::settings::{BatteryMessage, SettingsMenu, SettingsMenuMessage, AudioMessage};
use crate::modules::updates::{Update, UpdateMenu, UpdateMenuMessage, UpdateMenuOutput};
use crate::style::{ashell_theme, CRUST};
use crate::utils::audio::Sink;
use crate::utils::battery::BatteryData;
use iced::wayland::layer_surface::{set_anchor, set_size};
use iced::widget::container;
use iced::{
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
    Settings((Option<BatteryData>, Vec<Sink>)),
}

#[derive(Debug)]
pub enum MenuInput {
    Open(MenuType),
    MessageToUpdates(Vec<Update>),
    MessageToSettings(SettingsInputMessage),
    Close,
}

#[derive(Debug, Clone)]
pub enum SettingsInputMessage {
    Battery(BatteryMessage),
    Audio(AudioMessage),
}

pub enum MenuOutput {
    MessageFromUpdates(UpdateMenuOutput),
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

pub enum MenuInstance {
    Update(UpdateMenu),
    Settings(SettingsMenu),
}

pub struct Menu {
    menu_instance: Option<MenuInstance>,
    input_rx: RefCell<Option<UnboundedReceiver<MenuInput>>>,
    output_tx: UnboundedSender<MenuOutput>,
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
    OpenMenu(MenuType),
    UpdatesMenu(UpdateMenuMessage),
    SettingsMenu(SettingsMenuMessage),
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
                menu_instance: None,
                input_rx: RefCell::new(Some(flags.0)),
                output_tx: flags.1,
            },
            iced::Command::none(),
        )
    }

    fn theme(&self) -> Self::Theme {
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

                self.menu_instance = Some(MenuInstance::Update(UpdateMenu::new(
                    self.output_tx.clone(),
                    updates,
                )));

                cmd
            }
            Message::UpdatesMenu(msg) => {
                if let Some(MenuInstance::Update(updates)) = self.menu_instance.as_mut() {
                    updates.update(msg).map(Message::UpdatesMenu)
                } else {
                    iced::Command::none()
                }
            }
            Message::OpenMenu(MenuType::Settings((battery_data, sinks))) => {
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

                self.menu_instance = Some(MenuInstance::Settings(SettingsMenu::new(
                    self.output_tx.clone(),
                    battery_data,
                    sinks,
                )));

                cmd
            }
            Message::SettingsMenu(msg) => {
                if let Some(MenuInstance::Settings(settings)) = self.menu_instance.as_mut() {
                    settings.update(msg).map(Message::SettingsMenu)
                } else {
                    iced::Command::none()
                }
            }
        }
    }

    fn view(&self, _id: Id) -> iced::Element<'_, Self::Message> {
        if let Some(menu_instance) = self.menu_instance.as_ref() {
            iced::widget::mouse_area(
                container(
                    iced::widget::mouse_area(
                        container(match menu_instance {
                            MenuInstance::Update(updates_menu) => {
                                updates_menu.view().map(Message::UpdatesMenu)
                            }
                            MenuInstance::Settings(settings_menu) => {
                                settings_menu.view().map(Message::SettingsMenu)
                            }
                        })
                        .height(iced::Length::Shrink)
                        .width(iced::Length::Shrink)
                        .style(|theme: &Theme| {
                            iced::widget::container::Appearance {
                                background: Some(theme.palette().background.into()),
                                border_radius: 16.0.into(),
                                border_width: 1.,
                                border_color: CRUST,
                                ..Default::default()
                            }
                        }),
                    )
                    .on_release(Message::None),
                )
                .align_x(match menu_instance {
                    MenuInstance::Update(_) => iced::alignment::Horizontal::Left,
                    MenuInstance::Settings(_) => iced::alignment::Horizontal::Right,
                })
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
                            MenuInput::Open(MenuType::Settings(battery_data)) => {
                                Message::OpenMenu(MenuType::Settings(battery_data))
                            }
                            MenuInput::Close => Message::CloseRequest,
                            MenuInput::MessageToUpdates(msg) => {
                                Message::UpdatesMenu(UpdateMenuMessage::UpdatesCheckCompleted(msg))
                            }
                            MenuInput::MessageToSettings(msg) => {
                                Message::SettingsMenu(SettingsMenuMessage::MainMessage(msg))
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
