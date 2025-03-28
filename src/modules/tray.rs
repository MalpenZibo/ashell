use super::{Module, OnModulePress};
use crate::{
    app,
    components::icons::{Icons, icon},
    menu::MenuType,
    position_button::position_button,
    services::{
        ReadOnlyService, Service, ServiceEvent,
        tray::{
            TrayCommand, TrayService,
            dbus::{Layout, LayoutProps},
        },
    },
    style::GhostButtonStyle,
};
use iced::{
    Alignment, Element, Length, Subscription, Task,
    widget::{Column, Image, Row, button, horizontal_rule, row, text, toggler},
    window::Id,
};
use log::debug;

#[derive(Debug, Clone)]
pub enum TrayMessage {
    Event(ServiceEvent<TrayService>),
    ToggleSubmenu(i32),
    MenuSelected(String, i32),
}

#[derive(Debug, Default, Clone)]
pub struct TrayModule {
    pub service: Option<TrayService>,
    pub submenus: Vec<i32>,
}

impl TrayModule {
    pub fn update(&mut self, message: TrayMessage) -> Task<crate::app::Message> {
        match message {
            TrayMessage::Event(event) => match event {
                ServiceEvent::Init(service) => {
                    self.service = Some(service);
                    Task::none()
                }
                ServiceEvent::Update(data) => {
                    if let Some(service) = self.service.as_mut() {
                        service.update(data);
                    }
                    Task::none()
                }
                ServiceEvent::Error(_) => Task::none(),
            },
            TrayMessage::ToggleSubmenu(index) => {
                if self.submenus.contains(&index) {
                    self.submenus.retain(|i| i != &index);
                } else {
                    self.submenus.push(index);
                }
                Task::none()
            }
            TrayMessage::MenuSelected(name, id) => match self.service.as_mut() {
                Some(service) => {
                    debug!("Tray menu click: {}", id);
                    service
                        .command(TrayCommand::MenuSelected(name, id))
                        .map(|event| crate::app::Message::Tray(TrayMessage::Event(event)))
                }
                _ => Task::none(),
            },
        }
    }

    pub fn menu_view(&self, name: &'_ str) -> Element<TrayMessage> {
        match self
            .service
            .as_ref()
            .and_then(|service| service.data.iter().find(|item| item.name == name))
        {
            Some(item) => {
                Column::with_children(item.menu.2.iter().map(|menu| self.menu_voice(name, menu)))
                    .spacing(8)
                    .into()
            }
            _ => Row::new().into(),
        }
    }

    fn menu_voice(&self, name: &str, layout: &Layout) -> Element<TrayMessage> {
        match &layout.1 {
            LayoutProps {
                label: Some(label),
                toggle_type: Some(toggle_type),
                toggle_state: Some(state),
                ..
            } if toggle_type == "checkmark" => toggler(*state > 0)
                .label(label.replace("_", "").to_owned())
                .on_toggle({
                    let name = name.to_owned();
                    let id = layout.0;

                    move |_| TrayMessage::MenuSelected(name.to_owned(), id)
                })
                .width(Length::Fill)
                .into(),
            LayoutProps {
                children_display: Some(display),
                label: Some(label),
                ..
            } if display == "submenu" => {
                let is_open = self.submenus.contains(&layout.0);
                Column::new()
                    .push(
                        button(row!(
                            text(label.to_owned()).width(Length::Fill),
                            icon(if is_open {
                                Icons::MenuOpen
                            } else {
                                Icons::MenuClosed
                            })
                        ))
                        .style(GhostButtonStyle.into_style())
                        .padding([8, 8])
                        .on_press(TrayMessage::ToggleSubmenu(layout.0))
                        .width(Length::Fill),
                    )
                    .push_maybe(if is_open {
                        Some(
                            Column::with_children(
                                layout
                                    .2
                                    .iter()
                                    .map(|menu| self.menu_voice(name, menu))
                                    .collect::<Vec<_>>(),
                            )
                            .padding([0, 0, 0, 16])
                            .spacing(4),
                        )
                    } else {
                        None
                    })
                    .into()
            }
            LayoutProps {
                label: Some(label), ..
            } => button(text(label.replace("_", "")))
                .style(GhostButtonStyle.into_style())
                .on_press(TrayMessage::MenuSelected(name.to_owned(), layout.0))
                .width(Length::Fill)
                .padding([8, 8])
                .into(),
            LayoutProps { type_: Some(t), .. } if t == "separator" => horizontal_rule(1).into(),
            _ => Row::new().into(),
        }
    }
}

impl Module for TrayModule {
    type ViewData<'a> = Id;
    type SubscriptionData<'a> = ();

    fn view(
        &self,
        id: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)> {
        self.service
            .as_ref()
            .filter(|s| s.data.len() > 0)
            .map(|service| {
                (
                    Row::with_children(
                        service
                            .data
                            .iter()
                            .map(|item| {
                                position_button(match &item.icon_pixmap {
                                    Some(pixmap) => Into::<Element<_>>::into(
                                        Image::new(pixmap.clone()).height(Length::Fixed(14.)),
                                    ),
                                    _ => icon(Icons::Point).into(),
                                })
                                .on_press_with_position(move |button_ui_ref| {
                                    app::Message::ToggleMenu(
                                        MenuType::Tray(item.name.to_owned()),
                                        id,
                                        button_ui_ref,
                                    )
                                })
                                .padding([2, 2])
                                .style(GhostButtonStyle.into_style())
                                .into()
                            })
                            .collect::<Vec<_>>(),
                    )
                    .align_y(Alignment::Center)
                    .spacing(8)
                    .into(),
                    None,
                )
            })
    }

    fn subscription(&self, _: Self::SubscriptionData<'_>) -> Option<Subscription<app::Message>> {
        Some(TrayService::subscribe().map(|e| app::Message::Tray(TrayMessage::Event(e))))
    }
}
