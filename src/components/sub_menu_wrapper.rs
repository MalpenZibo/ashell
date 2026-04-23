use crate::theme::use_theme;
use iced::{Background, Border, Element, Length, Theme, widget::container};

pub fn sub_menu_wrapper<'a, Msg: 'static>(content: Element<'a, Msg>) -> Element<'a, Msg> {
    let (opacity, radius, space) = use_theme(|theme| (theme.opacity, theme.radius, theme.space));

    container(content)
        .style(move |theme: &Theme| container::Style {
            background: Background::Color(
                theme
                    .extended_palette()
                    .background
                    .weak
                    .color
                    .scale_alpha(opacity),
            )
            .into(),
            border: Border::default().rounded(radius.lg),
            ..container::Style::default()
        })
        .padding(space.md)
        .width(Length::Fill)
        .into()
}
