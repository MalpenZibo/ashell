use iced::{
    widget::{button, column, container, horizontal_space, row, text, text_input},
    Element, Length,
};

#[derive(Debug, Clone)]
pub enum Message {
    PasswordChanged(String),
    DialogConfirmed,
    DialogCancelled,
}

pub fn view<'a>(wifi_ssid: &str, current_password: &str) -> Element<'a, Message> {
    container(
        column!(
            text("Authentication required").size(22),
            text(format!(
                "Authentication is required to connect to the {} wifi network",
                wifi_ssid
            )),
            text_input("", current_password)
                .secure(true)
                .on_input(Message::PasswordChanged)
                .on_submit(Message::DialogConfirmed),
            row!(
                horizontal_space(),
                button("Cancel").on_press(Message::DialogCancelled),
                button("Connect").on_press(Message::DialogConfirmed)
            )
            .spacing(8)
            .width(Length::Fill)
            .height(Length::Fixed(32.))
        )
        .spacing(16)
        .padding(16)
        .max_width(350.),
    )
    .into()
}
