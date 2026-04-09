use crate::theme::AshellTheme;
use iced::{Background, Border, Element, Length, Theme, widget::container};

pub fn sub_menu_wrapper<'a, Msg: 'static>(
    ashell_theme: &'a AshellTheme,
    content: Element<'a, Msg>,
) -> Element<'a, Msg> {
    container(content)
        .style(move |theme: &Theme| container::Style {
            background: Background::Color(
                theme
                    .extended_palette()
                    .background
                    .weak
                    .color
                    .scale_alpha(ashell_theme.opacity),
            )
            .into(),
            border: Border::default().rounded(ashell_theme.radius.lg),
            ..container::Style::default()
        })
        .padding(ashell_theme.space.md)
        .width(Length::Fill)
        .into()
}
