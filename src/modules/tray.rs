use crate::{
    components::icons::{icon, Icons},
    menu::MenuType,
    outputs::Outputs,
    services::{
        tray::{
            dbus::{Layout, LayoutProps},
            TrayCommand, TrayService,
        },
        ReadOnlyService, Service, ServiceEvent,
    },
    style::{header_pills, GhostButtonStyle},
};
use iced::{
    widget::{button, container, horizontal_rule, row, text, toggler, Column, Image, Row},
    window::Id,
    Alignment, Element, Length, Task,
};
use log::debug;

#[derive(Debug, Clone)]
pub enum TrayMessage {
    Event(ServiceEvent<TrayService>),
    OpenMenu(Id, String),
    ToggleSubmenu(i32),
    MenuClick(String, i32),
}

#[derive(Debug, Default, Clone)]
pub struct TrayModule {
    service: Option<TrayService>,
    submenus: Vec<i32>,
}

impl TrayModule {
    pub fn update(
        &mut self,
        message: TrayMessage,
        outputs: &mut Outputs,
    ) -> Task<crate::app::Message> {
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
            TrayMessage::OpenMenu(id, name) => {
                if let Some(_tray) = self
                    .service
                    .as_ref()
                    .and_then(|t| t.iter().find(|t| t.name == name))
                {
                    self.submenus.clear();
                    outputs.toggle_menu(id, MenuType::Tray(name))
                } else {
                    Task::none()
                }
            }
            TrayMessage::ToggleSubmenu(index) => {
                if self.submenus.contains(&index) {
                    self.submenus.retain(|i| i != &index);
                } else {
                    self.submenus.push(index);
                }
                Task::none()
            }
            TrayMessage::MenuClick(name, id) => {
                if let Some(service) = self.service.as_mut() {
                    debug!("Tray menu click: {}", id);
                    service
                        .command(TrayCommand::MenuClick(name, id))
                        .map(|event| crate::app::Message::Tray(TrayMessage::Event(event)))
                } else {
                    Task::none()
                }
            }
        }
    }

    pub fn view(&self, id: Id) -> Option<Element<TrayMessage>> {
        self.service
            .as_ref()
            .filter(|s| s.data.len() > 0)
            .map(|service| {
                container(
                    Row::with_children(
                        service
                            .data
                            .iter()
                            .map(|item| {
                                button(if let Some(pixmap) = &item.icon_pixmap {
                                    Into::<Element<_>>::into(
                                        Image::new(pixmap.clone()).height(Length::Fixed(14.)),
                                    )
                                } else {
                                    icon(Icons::Point).into()
                                })
                                .on_press(TrayMessage::OpenMenu(id, item.name.to_owned()))
                                .into()
                            })
                            .collect::<Vec<_>>(),
                    )
                    .padding([2, 0])
                    .align_y(Alignment::Center)
                    .spacing(8),
                )
                .padding([2, 8])
                .style(header_pills)
                .into()
            })
    }

    pub fn menu_view(&self, name: &'_ str) -> Element<TrayMessage> {
        if let Some(item) = self
            .service
            .as_ref()
            .and_then(|service| service.data.iter().find(|item| item.name == name))
        {
            Column::with_children(item.menu.2.iter().map(|menu| self.menu_voice(name, menu)))
                .spacing(8)
                .padding(16)
                .max_width(350.)
                .max_width(300)
                .into()
        } else {
            Row::new().into()
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

                    move |_| TrayMessage::MenuClick(name.to_owned(), id)
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
                .on_press(TrayMessage::MenuClick(name.to_owned(), layout.0))
                .width(Length::Fill)
                .padding([8, 8])
                .into(),
            LayoutProps { type_: Some(t), .. } if t == "separator" => horizontal_rule(1).into(),
            _ => Row::new().into(),
        }
    }
}
