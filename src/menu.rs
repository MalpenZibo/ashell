use crate::app::{self, MenuType};
use crate::config::Position;
use iced::alignment::{Horizontal, Vertical};
use iced::platform_specific::shell::commands::layer_surface::{
    destroy_layer_surface, get_layer_surface, Anchor, KeyboardInteractivity, Layer,
};
use iced::runtime::platform_specific::wayland::layer_surface::SctkLayerSurfaceSettings;
use iced::widget::container::Style;
use iced::widget::mouse_area;
use iced::window::Id;
use iced::{self, widget::container, Element, Task, Theme};
use iced::{Border, Length, Padding};

#[derive(Debug, Default, Clone)]
pub struct Menu(Option<(Id, MenuType)>);

impl Menu {
    pub fn open(&mut self, menu_type: MenuType) -> Task<app::Message> {
        let id = Id::unique();
        let task = get_layer_surface(SctkLayerSurfaceSettings {
            id,
            size: None,
            layer: Layer::Overlay,
            pointer_interactivity: true,
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
            ..Default::default()
        });

        self.0.replace((id, menu_type));

        task
    }

    pub fn close<Message: 'static>(&mut self) -> Task<Message> {
        if let Some((id, _)) = self.0.take() {
            destroy_layer_surface(id)
        } else {
            Task::none()
        }
    }

    pub fn toggle(&mut self, menu_type: MenuType) -> Task<app::Message> {
        match self.0.as_mut() {
            None => self.open(menu_type),
            Some((_, current)) if *current == menu_type => self.close(),
            Some((_, current)) => {
                *current = menu_type;
                Task::none()
            }
        }
    }

    pub fn close_if<Message: 'static>(&mut self, menu_type: MenuType) -> Task<Message> {
        if let Some((_, current)) = self.0.as_ref() {
            if *current == menu_type {
                self.close()
            } else {
                Task::none()
            }
        } else {
            Task::none()
        }
    }

    pub fn get_menu_type_to_render(&self, id: Id) -> Option<MenuType> {
        self.0
            .as_ref()
            .filter(|(menu_id, _)| *menu_id == id)
            .map(|(_, menu_type)| *menu_type)
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
