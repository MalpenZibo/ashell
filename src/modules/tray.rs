use crate::{
    components::icons::{icon, Icons},
    menu::Menu,
    services::{
        tray::{TrayData, TrayService},
        ServiceEvent,
    },
    style::header_pills,
};
use iced::{
    widget::{button, column, container, text, Image, Row},
    Alignment, Command, Element, Length,
};

#[derive(Debug, Clone)]
pub enum TrayMessage {
    Event(ServiceEvent<TrayService>),
    OpenMenu(String),
}

impl TrayData {
    pub fn view(&self) -> Option<Element<TrayMessage>> {
        if self.len() > 0 {
            Some(
                container(
                    Row::with_children(
                        self.iter()
                            .map(|item| {
                                button(if let Some(pixmap) = &item.icon_pixmap {
                                    Into::<Element<_>>::into(
                                        Image::new(pixmap.clone()).height(Length::Fixed(14.)),
                                    )
                                } else {
                                    icon(Icons::Point).into()
                                })
                                .on_press(TrayMessage::OpenMenu(item.name.to_owned()))
                                .into()
                            })
                            .collect::<Vec<_>>(),
                    )
                    .padding([2, 0])
                    .align_items(Alignment::Center)
                    .spacing(8),
                )
                .padding([2, 8])
                .style(header_pills)
                .into(),
            )
        } else {
            None
        }
    }

    pub fn menu_view(&self, name: &str) -> Element<TrayMessage> {
        if let Some(item) = self.iter().find(|item| item.name == name) {
            column!(text("test")).into()
        } else {
            Row::new().into()
        }
    }
}
