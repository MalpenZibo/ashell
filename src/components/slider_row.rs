use crate::theme::AshellTheme;
use iced::{Alignment, Element, widget::Row};

/// A row layout for slider controls: icon + slider + optional trailing element.
///
/// Provides consistent spacing and alignment across audio and brightness sliders.
pub fn slider_row<'a, Msg: 'static>(
    theme: &AshellTheme,
    icon_element: Element<'a, Msg>,
    slider_element: Element<'a, Msg>,
    trailing: Option<Element<'a, Msg>>,
) -> Element<'a, Msg> {
    Row::with_capacity(3)
        .push(icon_element)
        .push(slider_element)
        .push(trailing)
        .align_y(Alignment::Center)
        .spacing(theme.space.xs)
        .into()
}
