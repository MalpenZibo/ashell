use crate::{
    components::icons::{StaticIcon, icon},
    theme::AshellTheme,
};
use iced::{
    Alignment, Element, Length,
    alignment::Vertical,
    widget::{button, column, horizontal_space, row, text, text_input},
    window::Id,
};

#[derive(Debug, Clone)]
pub enum Message {
    PasswordChanged(String),
    DialogConfirmed(Id),
    DialogCancelled(Id),
}

pub fn view<'a>(
    id: Id,
    theme: &'a AshellTheme,
    wifi_ssid: &str,
    current_password: &str,
) -> Element<'a, Message> {
    column!(
        row!(
            icon(StaticIcon::WifiLock4).size(theme.font_size.xxl),
            text("Authentication required").size(theme.font_size.xl),
        )
        .spacing(theme.space.md)
        .align_y(Alignment::Center),
        text(format!("Insert password to connect to: {wifi_ssid}")),
        text_input("", current_password)
            .secure(true)
            .size(theme.font_size.md)
            .padding([theme.space.xs, theme.space.md])
            .style(theme.text_input_style())
            .on_input(Message::PasswordChanged)
            .on_submit(Message::DialogConfirmed(id)),
        row!(
            horizontal_space(),
            button(text("Cancel").align_y(Vertical::Center))
                .padding([theme.space.xxs, theme.space.xl])
                .style(theme.outline_button_style())
                .height(Length::Fixed(50.))
                .on_press(Message::DialogCancelled(id)),
            button(text("Confirm").align_y(Vertical::Center))
                .padding([theme.space.xxs, theme.space.xl])
                .height(Length::Fixed(50.))
                .style(theme.confirm_button_style())
                .on_press(Message::DialogConfirmed(id))
        )
        .spacing(theme.space.xs)
        .width(Length::Fill)
    )
    .spacing(theme.space.md)
    .padding(theme.space.md)
    .into()
}
