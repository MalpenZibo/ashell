use crate::{
    components::icons::icon,
    services::{
        privacy::{Media, PrivacyData, PrivacyService},
        ServiceEvent,
    },
    style::HeaderButtonStyle,
};
use iced::{
    widget::{button, container, row, text, Column, Row},
    Element, Theme,
};

#[derive(Debug, Clone)]
pub enum PrivacyMessage {
    Event(ServiceEvent<PrivacyService>),
    ToggleMenu,
}

impl PrivacyData {
    pub fn view(&self) -> Option<Element<PrivacyMessage>> {
        if !self.is_empty() {
            Some(
                button(
                    container(
                        Row::new()
                            .push_maybe(self.iter().find_map(|app| {
                                if app.media == Media::Video {
                                    Some(icon(app.media.to_icon()))
                                } else {
                                    None
                                }
                            }))
                            .push_maybe(self.iter().find_map(|app| {
                                if app.media == Media::Audio {
                                    Some(icon(app.media.to_icon()))
                                } else {
                                    None
                                }
                            }))
                            .spacing(8),
                    )
                    .style(|theme: &Theme| container::Style {
                        text_color: Some(theme.extended_palette().danger.weak.color),
                        ..Default::default()
                    }),
                )
                .style(HeaderButtonStyle::None.into_style())
                .padding([2, 8])
                .on_press(PrivacyMessage::ToggleMenu)
                .into(),
            )
        } else {
            None
        }
    }

    pub fn menu_view(&self) -> Element<PrivacyMessage> {
        Column::with_children(
            self.iter()
                .map(|app| {
                    row![icon(app.media.to_icon()), text(app.application.clone()),]
                        .spacing(8)
                        .into()
                })
                .collect::<Vec<_>>(),
        )
        .spacing(4)
        .padding(16)
        .width(250)
        .into()
    }
}
