use iced::{
    alignment::Vertical,
    widget::{button, column, horizontal_space, row, text, text_input},
    Alignment, Element, Length,
};

use crate::{
    components::icons::{icon, Icons},
    style::{ConfirmButtonStyle, OutlineButtonStyle, TextInputStyle},
};

#[derive(Debug, Clone)]
pub enum Message {
    PasswordChanged(String),
    DialogConfirmed,
    DialogCancelled,
}

pub fn view<'a>(wifi_ssid: &str, current_password: &str) -> Element<'a, Message> {
    column!(
        row!(
            icon(Icons::WifiLock4).size(32),
            text("Authentication required").size(22),
        )
        .spacing(16)
        .align_y(Alignment::Center),
        text(format!("Insert password to connect to: {}", wifi_ssid)),
        text_input("", current_password)
            .secure(true)
            .size(16)
            .padding([8, 16])
            .style(TextInputStyle.into_style())
            .on_input(Message::PasswordChanged)
            .on_submit(Message::DialogConfirmed),
        row!(
            horizontal_space(),
            button(text("Cancel").align_y(Vertical::Center))
                .padding([4, 32])
                .style(OutlineButtonStyle.into_style())
                .height(Length::Fixed(50.))
                .on_press(Message::DialogCancelled),
            button(text("Confirm").align_y(Vertical::Center))
                .padding([4, 32])
                .height(Length::Fixed(50.))
                .style(ConfirmButtonStyle.into_style())
                .on_press(Message::DialogConfirmed)
        )
        .spacing(8)
        .width(Length::Fill)
    )
    .spacing(16)
    .padding(16)
    .max_width(350.)
    .into()
}
