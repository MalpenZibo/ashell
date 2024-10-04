use crate::app;
use crate::config::Position;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::container::Appearance;
use iced::widget::mouse_area;
use iced::window::Id;
use iced::{self, widget::container, Command, Element, Theme};
use iced::{Border, Length};
use iced_sctk::command::platform_specific::wayland::layer_surface::SctkLayerSurfaceSettings;
use iced_sctk::commands::layer_surface::{
    self, get_layer_surface, Anchor, KeyboardInteractivity, Layer,
};

fn open_menu<Message: 'static>() -> (Id, Command<Message>) {
    let id = Id::unique();

    (
        id,
        get_layer_surface(SctkLayerSurfaceSettings {
            id,
            keyboard_interactivity: KeyboardInteractivity::None,
            namespace: "ashell-menu".into(),
            layer: Layer::Overlay,
            size: Some((None, None)),
            anchor: Anchor::TOP
                .union(Anchor::LEFT)
                .union(Anchor::RIGHT)
                .union(Anchor::BOTTOM),
            ..Default::default()
        }),
    )
}

fn close_menu<Message: 'static>(id: Id) -> Command<Message> {
    layer_surface::destroy_layer_surface(id)
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum MenuType {
    Updates,
    Settings,
}

#[derive(Debug, Default)]
pub struct Menu {
    id: Option<Id>,
    menu_type: Option<MenuType>,
}

impl Menu {
    pub fn toggle<Message: 'static>(&mut self, menu_type: MenuType) -> Command<Message> {
        let current = self.menu_type.take();

        match current {
            None => {
                self.menu_type = Some(menu_type);
                let (id, cmd) = open_menu();
                self.id = Some(id);

                cmd
            }
            Some(current) if current == menu_type => {
                self.menu_type = None;
                if let Some(id) = self.id.take() {
                    close_menu(id)
                } else {
                    Command::none()
                }
            }
            Some(_) => {
                self.menu_type = Some(menu_type);
                Command::none()
            }
        }
    }

    pub fn close_if<Message: 'static>(&mut self, menu_type: MenuType) -> Command<Message> {
        if self.menu_type == Some(menu_type) {
            self.menu_type = None;
            if let Some(id) = self.id.take() {
                close_menu(id)
            } else {
                Command::none()
            }
        } else {
            Command::none()
        }
    }

    pub fn close<Message: 'static>(&mut self) -> Command<Message> {
        self.menu_type = None;

        if let Some(id) = self.id.take() {
            close_menu(id)
        } else {
            Command::none()
        }
    }

    pub fn set_keyboard_interactivity<Message: 'static>(&mut self) -> Command<Message> {
        if let Some(id) = self.id {
            layer_surface::set_keyboard_interactivity(id, KeyboardInteractivity::Exclusive)
        } else {
            Command::none()
        }
    }

    pub fn unset_keyboard_interactivity<Message: 'static>(&mut self) -> Command<Message> {
        if let Some(id) = self.id {
            layer_surface::set_keyboard_interactivity(id, KeyboardInteractivity::None)
        } else {
            Command::none()
        }
    }

    pub fn get_id(&self) -> Option<Id> {
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
    content: Element<app::Message>,
    position: MenuPosition,
    bar_position: Position,
) -> Element<app::Message> {
    mouse_area(
        container(
            mouse_area(
                container(content)
                    .height(Length::Shrink)
                    .width(Length::Shrink)
                    .style(|theme: &Theme| Appearance {
                        background: Some(theme.palette().background.into()),
                        border: Border {
                            color: theme.extended_palette().secondary.base.color,
                            width: 1.,
                            radius: 16.0.into(),
                        },
                        ..Default::default()
                    }),
            )
            .on_release(app::Message::None),
        )
        .align_y(match bar_position {
            Position::Top => Vertical::Top,
            Position::Bottom => Vertical::Bottom,
        })
        .align_x(match position {
            MenuPosition::Left => Horizontal::Left,
            MenuPosition::Right => Horizontal::Right,
        })
        .padding([0, 8, 8, 8])
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .on_release(app::Message::CloseMenu)
    .into()
}
