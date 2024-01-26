use iced::{
    wayland::layer_surface::{Anchor, KeyboardInteractivity},
    widget::{button, column, container, horizontal_space, row, text, text_input},
    window::Id,
    Command, Element, Theme,
};

use crate::style::CRUST;

pub fn open_dialog<Message>() -> (Id, Command<Message>) {
    let id = Id::unique();

    let spawn_window = iced::wayland::layer_surface::get_layer_surface(
        iced::wayland::actions::layer_surface::SctkLayerSurfaceSettings {
            id,
            layer: iced::wayland::layer_surface::Layer::Overlay,
            anchor: Anchor::TOP
                .union(Anchor::LEFT)
                .union(Anchor::RIGHT)
                .union(Anchor::BOTTOM),
            size: None,
            keyboard_interactivity: KeyboardInteractivity::Exclusive,
            ..Default::default()
        },
    );

    (id, spawn_window)
}

pub fn close_dialog<Message>(id: Id) -> Command<Message> {
    iced::wayland::layer_surface::destroy_layer_surface(id)
}

#[derive(Debug, Clone)]
pub enum Message {
    PasswordChanged(String),
    DialogConfirmed,
    DialogCancelled,
}

pub fn password_dialog<'a>(wifi_ssid: &str, current_password: &str) -> Element<'a, Message> {
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
        .spacing(16),
    )
    .padding(16)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .style(|theme: &Theme| iced::widget::container::Appearance {
        background: Some(theme.palette().background.into()),
        border_radius: 16.0.into(),
        border_width: 1.,
        border_color: CRUST,
        ..Default::default()
    })
    .max_width(400)
    .into()
}
