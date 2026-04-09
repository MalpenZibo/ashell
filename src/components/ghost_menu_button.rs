use crate::{components::icons::IconKind, theme::AshellTheme};
use iced::{
    Alignment, Element, Length,
    widget::{button, row, text},
};

pub fn ghost_menu_button<'a, Msg: 'static + Clone>(
    theme: &'a AshellTheme,
    icon: impl Into<IconKind>,
    label: impl text::IntoFragment<'a>,
    on_press: Msg,
) -> Element<'a, Msg> {
    button(
        row![icon.into().to_text(), text(label)]
            .spacing(theme.space.md)
            .align_y(Alignment::Center),
    )
    .padding([theme.space.xxs, theme.space.sm])
    .on_press(on_press)
    .width(Length::Fill)
    .style(theme.ghost_button_style())
    .into()
}
