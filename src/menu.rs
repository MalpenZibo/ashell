use iced::window::Id;
use iced::Border;
use iced::{self, widget::container, Command, Element, Theme};
use iced_sctk::command::platform_specific::wayland::layer_surface::SctkLayerSurfaceSettings;
use iced_sctk::commands::layer_surface::{
    self, get_layer_surface, Anchor, KeyboardInteractivity, Layer,
};

use crate::config::Position;

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

// fn create_menu_surface<Message>() -> (Id, Command<Message>) {
//     let id = Id::unique();
//     (
//         id,
//         iced::wayland::layer_surface::get_layer_surface(SctkLayerSurfaceSettings {
//             id,
//             keyboard_interactivity: KeyboardInteractivity::None,
//             namespace: "ashell-menu".into(),
//             layer: Layer::Background,
//             size: Some((None, None)),
//             anchor: Anchor::TOP
//                 .union(Anchor::LEFT)
//                 .union(Anchor::RIGHT)
//                 .union(Anchor::BOTTOM),
//             ..Default::default()
//         }),
//     )
// }
//
// fn open_menu<Message>(id: Id) -> Command<Message> {
//     iced::Command::batch(vec![iced::wayland::layer_surface::set_layer(
//         id,
//         Layer::Overlay,
//     )])
// }
//
// fn close_menu<Message>(id: Id) -> Command<Message> {
//     iced::wayland::layer_surface::set_layer(id, Layer::Background)
// }

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum MenuType {
    Updates,
    Privacy,
    Settings,
}

pub struct Menu<Message: 'static> {
    id: Option<Id>,
    menu_type: Option<MenuType>,
    phantom: std::marker::PhantomData<Message>,
}

impl<Message: 'static> Menu<Message> {
    pub fn init() -> Self {
        Self {
            id: None,
            menu_type: None,
            phantom: std::marker::PhantomData,
        }
    }

    pub fn toggle(&mut self, menu_type: MenuType) -> Command<Message> {
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
                    iced::Command::none()
                }
            }
            Some(_) => {
                self.menu_type = Some(menu_type);
                iced::Command::none()
            }
        }
    }

    pub fn close_if(&mut self, menu_type: MenuType) -> Command<Message> {
        if self.menu_type == Some(menu_type) {
            self.menu_type = None;
            if let Some(id) = self.id.take() {
                close_menu(id)
            } else {
                iced::Command::none()
            }
        } else {
            iced::Command::none()
        }
    }

    pub fn close(&mut self) -> Command<Message> {
        self.menu_type = None;

        if let Some(id) = self.id.take() {
            close_menu(id)
        } else {
            iced::Command::none()
        }
    }

    pub fn set_keyboard_interactivity(&mut self) -> Command<Message> {
        if let Some(id) = self.id {
            layer_surface::set_keyboard_interactivity(id, KeyboardInteractivity::Exclusive)
        } else {
            iced::Command::none()
        }
    }

    pub fn unset_keyboard_interactivity(&mut self) -> Command<Message> {
        if let Some(id) = self.id {
            layer_surface::set_keyboard_interactivity(id, KeyboardInteractivity::None)
        } else {
            iced::Command::none()
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
    content: Element<crate::app::Message>,
    position: MenuPosition,
    bar_position: Position,
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
                            color: theme.extended_palette().secondary.base.color,
                            width: 1.,
                            radius: 16.0.into(),
                        },
                        ..Default::default()
                    }),
            )
            .on_release(crate::app::Message::None),
        )
        .align_y(match bar_position {
            Position::Top => iced::alignment::Vertical::Top,
            Position::Bottom => iced::alignment::Vertical::Bottom,
        })
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
