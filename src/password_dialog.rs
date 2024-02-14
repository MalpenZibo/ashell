use iced::{
    wayland::layer_surface::{Anchor, KeyboardInteractivity},
    widget::{button, column, container, horizontal_space, row, text, text_input},
    window::Id,
    Border, Command, Element, Length, Theme,
};

use crate::{style::CRUST, HEIGHT};

pub fn close_password_dialog<Message>(id: Id, maintain_size: bool) -> Command<Message> {
    if maintain_size {
        iced::wayland::layer_surface::set_keyboard_interactivity(id, KeyboardInteractivity::None)
    } else {
        Command::batch(vec![
            iced::wayland::layer_surface::set_anchor(
                id,
                Anchor::TOP.union(Anchor::LEFT).union(Anchor::RIGHT),
            ),
            iced::wayland::layer_surface::set_size(id, None, Some(HEIGHT)),
            iced::wayland::layer_surface::set_keyboard_interactivity(
                id,
                KeyboardInteractivity::None,
            ),
        ])
    }
}

pub fn open_password_dialog<Message>(id: Id) -> Command<Message> {
    Command::batch(vec![
        iced::wayland::layer_surface::set_anchor(
            id,
            Anchor::TOP
                .union(Anchor::LEFT)
                .union(Anchor::RIGHT)
                .union(Anchor::BOTTOM),
        ),
        iced::wayland::layer_surface::set_size(id, None, None),
        iced::wayland::layer_surface::set_keyboard_interactivity(
            id,
            KeyboardInteractivity::Exclusive,
        ),
    ])
}

#[derive(Debug, Clone)]
pub enum Message {
    PasswordChanged(String),
    DialogConfirmed,
    DialogCancelled,
}

pub fn view<'a>(wifi_ssid: &str, current_password: &str) -> Element<'a, Message> {
    container(
        container(
            column!(
                text("Authentication required").size(22),
                text(format!(
                    "Authentication is required to connect to the {} wifi network",
                    wifi_ssid
                )),
                text_input("", current_password)
                    .password()
                    .on_input(Message::PasswordChanged)
                    .on_submit(Message::DialogConfirmed),
                row!(
                    horizontal_space(iced::Length::Fill),
                    button("Cancel").on_press(Message::DialogCancelled),
                    button("Connect").on_press(Message::DialogConfirmed)
                )
                .spacing(8)
                .width(iced::Length::Fill)
                .height(iced::Length::Fixed(32.))
            )
            .spacing(16)
            .padding(16)
            .max_width(350.),
        )
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
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .into()
}
