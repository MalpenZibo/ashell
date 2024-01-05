use crate::style::CRUST;
use iced::widget::container;
use iced::{wayland::layer_surface::Anchor, window::Id, Theme};
use iced::{Command, Element};

pub fn create_menu<Message>() -> (Id, Command<Message>) {
    let id = Id::unique();

    let spawn_window = iced::wayland::layer_surface::get_layer_surface(
        iced::wayland::actions::layer_surface::SctkLayerSurfaceSettings {
            id,
            layer: iced::wayland::layer_surface::Layer::Overlay,
            anchor: Anchor::TOP
                .union(Anchor::LEFT)
                .union(Anchor::RIGHT)
                .union(Anchor::BOTTOM),
            size: Some((None, None)),
            ..Default::default()
        },
    );

    (id, spawn_window)
}

pub fn close_menu<Message>(id: Id) -> Command<Message> {
    Command::batch(vec![
        iced::wayland::layer_surface::set_anchor(
            id,
            Anchor::TOP
                .union(Anchor::LEFT)
                .union(Anchor::RIGHT)
        ),
        iced::wayland::layer_surface::set_size(id, None, Some(1)),
    ])
}

pub fn open_menu<Message>(id: Id) -> Command<Message> {
    Command::batch(vec![
        iced::wayland::layer_surface::set_anchor(
            id,
            Anchor::TOP
                .union(Anchor::LEFT)
                .union(Anchor::RIGHT)
                .union(Anchor::BOTTOM),
        ),
        iced::wayland::layer_surface::set_size(id, None, None),
    ])
}

pub enum MenuPosition {
    Left,
    Right,
}

pub fn menu_wrapper(
    content: Element<crate::app::Message, iced::Renderer>,
    position: MenuPosition,
) -> Element<crate::app::Message, iced::Renderer> {
    iced::widget::mouse_area(
        container(
            iced::widget::mouse_area(
                container(content)
                    .height(iced::Length::Shrink)
                    .width(iced::Length::Shrink)
                    .style(|theme: &Theme| iced::widget::container::Appearance {
                        background: Some(theme.palette().background.into()),
                        border_radius: 16.0.into(),
                        border_width: 1.,
                        border_color: CRUST,
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
