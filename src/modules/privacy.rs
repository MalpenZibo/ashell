use iced::{
    theme::Button,
    widget::{button, container, row, text, Column, Row},
    Command, Element, Subscription, Theme,
};

use crate::{
    app::Message,
    components::icons::{icon, Icons},
    menu::{Menu, MenuType},
    style::HeaderButtonStyle,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Media {
    Video,
    Audio,
}

impl Media {
    pub fn to_icon(self) -> Icons {
        match self {
            Media::Video => Icons::ScreenShare,
            Media::Audio => Icons::Mic1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApplicationNode {
    pub id: u32,
    pub media: Media,
    pub application: String,
}

#[derive(Debug, Clone)]
pub enum PrivacyMessage {
    Applications(Vec<ApplicationNode>),
    ToggleMenu,
}

pub struct Privacy {
    pub applications: Vec<ApplicationNode>,
}

impl Privacy {
    pub fn new() -> Self {
        Self {
            applications: vec![],
        }
    }

    pub fn update(
        &mut self,
        message: PrivacyMessage,
        menu: &mut Menu<Message>,
    ) -> Command<Message> {
        match message {
            PrivacyMessage::Applications(applications) => {
                self.applications = applications;

                Command::none()
            }
            PrivacyMessage::ToggleMenu => menu.toggle(MenuType::Privacy),
        }
    }

    pub fn view(&self) -> Element<PrivacyMessage> {
        button(
            container(
                Row::with_children(
                    vec![
                        self.applications.iter().find_map(|app| {
                            if app.media == Media::Video {
                                Some(icon(app.media.to_icon()).into())
                            } else {
                                None
                            }
                        }),
                        self.applications.iter().find_map(|app| {
                            if app.media == Media::Audio {
                                Some(icon(app.media.to_icon()).into())
                            } else {
                                None
                            }
                        }),
                    ]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>(),
                )
                .spacing(8),
            )
            .style(|theme: &Theme| container::Appearance {
                text_color: Some(theme.extended_palette().danger.weak.color),
                ..Default::default()
            }),
        )
        .style(Button::custom(HeaderButtonStyle::None))
        .padding([2, 8])
        .on_press(PrivacyMessage::ToggleMenu)
        .into()
    }

    pub fn menu_view(&self) -> Element<PrivacyMessage> {
        Column::with_children(
            self.applications
                .iter()
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

    pub fn subscription(&self) -> Subscription<PrivacyMessage> {
        crate::utils::privacy::subscription()
    }
}
