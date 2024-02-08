use crate::style::CRUST;
use iced::widget::container;
use iced::{wayland::layer_surface::Anchor, window::Id, Theme};
use iced::{Border, Command, Element};

pub fn close_menu<Message>() -> Command<Message> {
    Command::batch(vec![
        iced::wayland::layer_surface::set_anchor(
            Id::MAIN,
            Anchor::TOP.union(Anchor::LEFT).union(Anchor::RIGHT),
        ),
        iced::wayland::layer_surface::set_size(Id::MAIN, None, Some(34)),
    ])
}

pub fn open_menu<Message>() -> Command<Message> {
    Command::batch(vec![
        iced::wayland::layer_surface::set_anchor(
            Id::MAIN,
            Anchor::TOP.union(Anchor::LEFT).union(Anchor::RIGHT), // .union(Anchor::BOTTOM),
        ),
        iced::wayland::layer_surface::set_size(Id::MAIN, None, Some(1000)),
    ])
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
