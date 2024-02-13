use crate::style::CRUST;
use iced::wayland::actions::layer_surface::SctkLayerSurfaceSettings;
use iced::wayland::layer_surface::Layer;
use iced::widget::container;
use iced::{wayland::layer_surface::Anchor, window::Id, Theme};
use iced::{Border, Command, Element};

pub fn close_menu<Message>(id: Id) -> Command<Message> {
    iced::wayland::layer_surface::destroy_layer_surface(id)
}

pub fn open_menu<Message>() -> (Id, Command<Message>) {
    let id = Id::unique();
    (
        id,
        iced::wayland::layer_surface::get_layer_surface(SctkLayerSurfaceSettings {
            id,
            layer: Layer::Overlay,
            anchor: Anchor::TOP
                .union(Anchor::LEFT)
                .union(Anchor::RIGHT)
                .union(Anchor::BOTTOM),
            exclusive_zone: 0,
            size: Some((None, None)),
            ..Default::default()
        }),
    )
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
