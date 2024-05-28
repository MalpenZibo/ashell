use crate::style::CRUST;
use iced::wayland::actions::layer_surface::SctkLayerSurfaceSettings;
use iced::wayland::layer_surface::{Anchor, KeyboardInteractivity, Layer};
use iced::widget::container;
use iced::{window::Id, Theme};
use iced::{Border, Command, Element};

fn create_menu_surface<Message>() -> (Id, Command<Message>) {
    let id = Id::unique();
    (
        id,
        iced::wayland::layer_surface::get_layer_surface(SctkLayerSurfaceSettings {
            id,
            keyboard_interactivity: KeyboardInteractivity::None,
            namespace: "ashell-menu".into(),
            layer: Layer::Background,
            size: Some((None, None)),
            anchor: Anchor::TOP
                .union(Anchor::LEFT)
                .union(Anchor::RIGHT)
                .union(Anchor::BOTTOM),
            ..Default::default()
        }),
    )
}

fn open_menu<Message>(id: Id) -> Command<Message> {
    iced::wayland::layer_surface::set_layer(id, Layer::Overlay)
}

fn close_menu<Message>(id: Id) -> Command<Message> {
    iced::wayland::layer_surface::set_layer(id, Layer::Background)
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum MenuType {
    Updates,
    Privacy,
    Settings,
}

pub struct Menu {
    id: Id,
    menu_type: Option<MenuType>,
}

impl Menu {
    pub fn init() -> (Self, Command<crate::app::Message>) {
        let (id, cmd) = create_menu_surface();
        (
            Self {
                id,
                menu_type: None,
            },
            cmd,
        )
    }

    pub fn toggle<Msg>(&mut self, menu_type: MenuType) -> Command<Msg> {
        let current = self.menu_type.take();

        match current {
            None => {
                self.menu_type = Some(menu_type);
                open_menu(self.id)
            }
            Some(current) if current == menu_type => {
                self.menu_type = None;
                close_menu(self.id)
            }
            Some(_) => {
                self.menu_type = Some(menu_type);
                iced::Command::none()
            }
        }
    }

    pub fn close_if<Msg>(&mut self, menu_type: MenuType) -> Command<Msg> {
        if self.menu_type == Some(menu_type) {
            self.menu_type = None;
            close_menu(self.id)
        } else {
            iced::Command::none()
        }
    }

    pub fn close<Msg>(&mut self) -> Command<Msg> {
        self.menu_type = None;

        close_menu(self.id)
    }

    pub fn set_keyboard_interactivity<Msg>(&mut self) -> Command<Msg> {
        iced::wayland::layer_surface::set_keyboard_interactivity(
            self.id,
            KeyboardInteractivity::Exclusive,
        )
    }

    pub fn unset_keyboard_interactivity<Msg>(&mut self) -> Command<Msg> {
        iced::wayland::layer_surface::set_keyboard_interactivity(
            self.id,
            KeyboardInteractivity::None,
        )
    }

    pub fn get_id(&self) -> Id {
        self.id
    }

    pub fn get_menu_type(&self) -> Option<MenuType> {
        self.menu_type
    }
}

pub enum MenuPosition {
    Left,
    Right,
}

pub fn menu_wrapper(
    content: Element<crate::app::Message>,
    position: MenuPosition,
) -> Element<crate::app::Message> {
    iced::widget::mouse_area(
        container(
            iced::widget::mouse_area(
                container(content)
                    .height(iced::Length::Shrink)
                    .width(iced::Length::Shrink)
                    .style(|theme: &Theme| iced::widget::container::Appearance {
                        background: Some(theme.palette().background.into()),
                        border: Border {
                            color: CRUST,
                            width: 1.,
                            radius: 16.0.into(),
                        },
                        ..Default::default()
                    }),
            )
            .on_release(crate::app::Message::None),
        )
        .align_x(match position {
            MenuPosition::Left => iced::alignment::Horizontal::Left,
            MenuPosition::Right => iced::alignment::Horizontal::Right,
        })
        .padding([0, 8, 8, 8])
        .width(iced::Length::Fill)
        .height(iced::Length::Fill),
    )
    .on_release(crate::app::Message::CloseMenu)
    .into()
}
