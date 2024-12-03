use crate::app::{self, MenuType};
use crate::config::Position;
use iced::alignment::{Horizontal, Vertical};
use iced::platform_specific::shell::commands::layer_surface::{set_layer, Layer};
use iced::widget::container::Style;
use iced::widget::mouse_area;
use iced::window::Id;
use iced::{self, widget::container, Element, Task, Theme};
use iced::{Border, Length, Padding};

#[derive(Debug, Clone)]
pub struct Menu {
    id: Id,
    menu_type: Option<MenuType>,
}

impl Menu {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            menu_type: None,
        }
    }

    pub fn open(&mut self, menu_type: MenuType) -> Task<app::Message> {
        let task = set_layer(self.id, Layer::Overlay);

        self.menu_type.replace(menu_type);

        task
    }

    pub fn close<Message: 'static>(&mut self) -> Task<Message> {
        if self.menu_type.is_some() {
            self.menu_type.take();
            set_layer(self.id, Layer::Background)
        } else {
            Task::none()
        }
    }

    pub fn toggle(&mut self, menu_type: MenuType) -> Task<app::Message> {
        match self.menu_type.as_mut() {
            None => self.open(menu_type),
            Some(current) if *current == menu_type => self.close(),
            Some(current) => {
                *current = menu_type;
                Task::none()
            }
        }
    }

    pub fn close_if<Message: 'static>(&mut self, menu_type: MenuType) -> Task<Message> {
        if let Some(current) = self.menu_type.as_ref() {
            if *current == menu_type {
                self.close()
            } else {
                Task::none()
            }
        } else {
            Task::none()
        }
    }

    pub fn is_menu(&self, id: Id) -> bool {
        self.id == id
    }

    pub fn get_menu_type_to_render(&self, id: Id) -> Option<MenuType> {
        if self.id == id {
            self.menu_type
        } else {
            None
        }
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
                    .style(|theme: &Theme| Style {
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
        .padding(Padding::new(8.).top(0))
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .on_release(app::Message::CloseMenu)
    .into()
}
