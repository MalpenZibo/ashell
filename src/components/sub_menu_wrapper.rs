use crate::theme::use_theme;
use iced::{Background, Border, Element, Length, Theme, widget::container};

pub fn sub_menu_wrapper<'a, Msg: 'static>(content: Element<'a, Msg>) -> Element<'a, Msg> {
    let (opacity, radius_lg, padding_md) =
        use_theme(|theme| (theme.opacity, theme.radius.lg, theme.space.md));

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
            border: Border::default().rounded(radius_lg),
            ..container::Style::default()
        })
        .padding(padding_md)
        .width(Length::Fill)
        .into()
}
