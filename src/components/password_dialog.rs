use super::icons::{StaticIcon, icon, icon_button};
use crate::{
    components::{ButtonHierarchy, ButtonKind, styled_button},
    theme::AshellTheme,
};
use iced::{
    Alignment, Element, Length, SurfaceId,
    widget::{column, row, space, text, text_input},
};

#[derive(Debug, Clone)]
pub enum Message {
    PasswordChanged(String),
    TogglePasswordVisibility,
    DialogConfirmed(SurfaceId),
    DialogCancelled(SurfaceId),
}

pub fn view<'a>(
    id: SurfaceId,
    theme: &'a AshellTheme,
    wifi_ssid: &str,
    current_password: &str,
    show_password: bool,
    warning_only: bool,
) -> Element<'a, Message> {
    let title = if warning_only {
        "Open network"
    } else {
        "Authentication required"
    };

    let description = if warning_only {
        format!(
            "\"{}\" is an open network. Data sent over this connection may be visible to others.
Do you want to connect anyway?",
            wifi_ssid
        )
    } else {
        format!("Insert password to connect to: {}", wifi_ssid)
    };

    column!(
        row!(
            icon(if warning_only {
                StaticIcon::Wifi4
            } else {
                StaticIcon::WifiLock4
            })
            .size(theme.font_size.xxl),
            text(title).size(theme.font_size.xl),
        )
        .spacing(theme.space.md)
        .align_y(Alignment::Center),
        text(description),
    )
    .push(
        (!warning_only).then_some(
            row!(
                text_input("", current_password)
                    .secure(!show_password)
                    .size(theme.font_size.md)
                    .padding([theme.space.xs, theme.space.md])
                    .style(theme.text_input_style())
                    .on_input(Message::PasswordChanged)
                    .on_submit(Message::DialogConfirmed(id))
                    .width(Length::Fill),
                icon_button(
                    theme,
                    if show_password {
                        StaticIcon::EyeOpened
                    } else {
                        StaticIcon::EyeClosed
                    },
                )
                .on_press(Message::TogglePasswordVisibility),
            )
            .spacing(theme.space.sm)
            .align_y(Alignment::Center),
        ),
    )
    .push(
        row!(
            space::horizontal(),
            styled_button(theme, "Cancel")
                .kind(ButtonKind::Outline)
                .height(Length::Fixed(50.))
                .on_press(Message::DialogCancelled(id)),
            styled_button(theme, "Confirm")
                .kind(ButtonKind::Solid)
                .hierarchy(ButtonHierarchy::Primary)
                .height(Length::Fixed(50.))
                .on_press_maybe(if !warning_only && current_password.is_empty() {
                    None
                } else {
                    Some(Message::DialogConfirmed(id))
                })
        )
        .spacing(theme.space.xs)
        .width(Length::Fill),
    )
    .spacing(theme.space.md)
    .padding(theme.space.md)
    .into()
}
