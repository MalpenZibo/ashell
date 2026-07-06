use super::icons::{StaticIcon, icon, icon_button};
use crate::{
    components::{ButtonHierarchy, ButtonKind, styled_button},
    t,
    theme::use_theme,
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
    wifi_ssid: &str,
    current_password: &str,
    show_password: bool,
    warning_only: bool,
) -> Element<'a, Message> {
    let (space, font_size, text_input_style) =
        use_theme(|theme| (theme.space, theme.font_size, theme.text_input_style()));

    let title = if warning_only {
        t!("password-dialog-open-network-title")
    } else {
        t!("password-dialog-authentication-required-title")
    };

    let description = if warning_only {
        t!("password-dialog-open-network-warning", ssid = wifi_ssid)
    } else {
        t!("password-dialog-insert-password", ssid = wifi_ssid)
    };

    column!(
        row!(
            icon(if warning_only {
                StaticIcon::Wifi4
            } else {
                StaticIcon::WifiLock4
            })
            .size(font_size.xxl),
            text(title).size(font_size.xl),
        )
        .spacing(space.md)
        .align_y(Alignment::Center),
        text(description),
    )
    .push(
        (!warning_only).then_some(
            row!(
                text_input("", current_password)
                    .secure(!show_password)
                    .size(font_size.md)
                    .padding([space.xs, space.md])
                    .style(text_input_style)
                    .on_input(Message::PasswordChanged)
                    .on_submit(Message::DialogConfirmed(id))
                    .width(Length::Fill),
                icon_button(if show_password {
                    StaticIcon::EyeOpened
                } else {
                    StaticIcon::EyeClosed
                })
                .on_press(Message::TogglePasswordVisibility),
            )
            .spacing(space.sm)
            .align_y(Alignment::Center),
        ),
    )
    .push(
        row!(
            space::horizontal(),
            styled_button(t!("password-dialog-cancel"))
                .kind(ButtonKind::Outline)
                .height(Length::Fixed(50.))
                .on_press(Message::DialogCancelled(id)),
            styled_button(t!("password-dialog-confirm"))
                .kind(ButtonKind::Solid)
                .hierarchy(ButtonHierarchy::Primary)
                .height(Length::Fixed(50.))
                .on_press_maybe(if !warning_only && current_password.is_empty() {
                    None
                } else {
                    Some(Message::DialogConfirmed(id))
                })
        )
        .spacing(space.xs)
        .width(Length::Fill),
    )
    .spacing(space.md)
    .padding(space.md)
    .into()
}
