use crate::theme::use_theme;
use iced::{Element, Length, widget::container};

pub fn sub_menu_wrapper<'a, Msg: 'static>(content: Element<'a, Msg>) -> Element<'a, Msg> {
    let (radius, space) = use_theme(|theme| (theme.radius, theme.space));

    container(content)
        .style(crate::theme::card_style(radius.lg))
        .padding(space.md)
        .width(Length::Fill)
        .into()
}
