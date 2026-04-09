use crate::{components::icons::Icon, theme::AshellTheme};
use iced::{
    Alignment, Element, Length,
    widget::{button, row, text},
};

use super::icons::icon;

pub fn ghost_menu_button<'a, Msg: 'static + Clone>(
    theme: &'a AshellTheme,
    icon_type: impl Icon,
    label: impl text::IntoFragment<'a>,
    on_press: Msg,
) -> Element<'a, Msg> {
    button(
        row![icon(icon_type), text(label)]
            .spacing(theme.space.md)
            .align_y(Alignment::Center),
    )
    .padding([theme.space.xxs, theme.space.sm])
    .on_press(on_press)
    .width(Length::Fill)
    .style(theme.ghost_button_style())
    .into()
}
