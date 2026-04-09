use crate::theme::AshellTheme;
use iced::{
    Alignment, Element, Length, Theme,
    widget::{button, container, row, text},
};

pub fn selectable_list_item<'a, Msg: 'static + Clone>(
    theme: &'a AshellTheme,
    icon_element: Element<'a, Msg>,
    label: impl text::IntoFragment<'a>,
    active: bool,
    on_select: Msg,
) -> Element<'a, Msg> {
    if active {
        container(
            row![icon_element, text(label)]
                .align_y(Alignment::Center)
                .spacing(theme.space.md)
                .padding([theme.space.xxs, theme.space.sm]),
        )
        .style(|theme: &Theme| container::Style {
            text_color: Some(theme.palette().success),
            ..Default::default()
        })
        .into()
    } else {
        button(
            row![icon_element, text(label)]
                .spacing(theme.space.md)
                .align_y(Alignment::Center),
        )
        .on_press(on_select)
        .padding([theme.space.xxs, theme.space.sm])
        .width(Length::Fill)
        .style(theme.ghost_button_style())
        .into()
    }
}
