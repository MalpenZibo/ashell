use iced::{
    wayland::layer_surface::KeyboardInteractivity,
    widget::{button, column, horizontal_space, row, text, text_input},
    window::Id,
    Command, Element,
};

pub fn request_keyboard<Msg>() -> Command<Msg> {
    iced::wayland::layer_surface::set_keyboard_interactivity(
        Id::MAIN,
        KeyboardInteractivity::Exclusive,
    )
}

pub fn release_keyboard<Msg>() -> Command<Msg> {
    iced::wayland::layer_surface::set_keyboard_interactivity(
        Id::MAIN,
        KeyboardInteractivity::None,
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
