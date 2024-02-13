use iced::{
    wayland::{actions::layer_surface::SctkLayerSurfaceSettings, layer_surface::{Anchor, KeyboardInteractivity}},
    widget::{button, column, horizontal_space, row, text, text_input},
    window::Id,
    Command, Element,
};

pub fn close_password_dialog<Message>(id: Id) -> Command<Message> {
    iced::wayland::layer_surface::destroy_layer_surface(id)
}

pub fn open_password_dialog<Message>() -> (Id, Command<Message>) {
    let id = Id::unique();
    (
        id,
        iced::wayland::layer_surface::get_layer_surface(SctkLayerSurfaceSettings {
            id,
            anchor: Anchor::TOP
                .union(Anchor::LEFT)
                .union(Anchor::RIGHT)
                .union(Anchor::BOTTOM),
            exclusive_zone: 0,
            keyboard_interactivity: KeyboardInteractivity::Exclusive,
            ..Default::default()
        }),
    )
}

#[derive(Debug, Clone)]
pub enum Message {
    PasswordChanged(String),
    DialogConfirmed,
    DialogCancelled,
}

pub fn view<'a>(wifi_ssid: &str, current_password: &str) -> Element<'a, Message> {
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
    .max_width(350.)
    .into()
}
