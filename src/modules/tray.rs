use crate::{
    components::icons::{StaticIcon, icon},
    menu::MenuSize,
    services::{
        ReadOnlyService, Service, ServiceEvent,
        tray::{
            TrayCommand, TrayEvent, TrayIcon, TrayService,
            dbus::{Layout, LayoutProps},
        },
    },
    theme::AshellTheme,
    widgets::{ButtonUIRef, position_button},
};
use iced::{
    Alignment, Element, Length, Subscription, Task,
    widget::{Column, Image, Row, Svg, button, container, horizontal_rule, row, text, toggler},
    window::Id,
};
use log::debug;

#[derive(Debug, Clone)]
pub enum Message {
    Event(Box<ServiceEvent<TrayService>>),
    ToggleMenu(String, Id, ButtonUIRef),
    ToggleSubmenu(i32),
    MenuSelected(String, i32),
    MenuOpened(String),
}

pub enum Action {
    None,
    ToggleMenu(String, Id, ButtonUIRef),
    TrayMenuCommand(Task<Message>),
    CloseTrayMenu(String),
}

#[derive(Debug, Default, Clone)]
pub struct TrayModule {
    service: Option<TrayService>,
    submenus: Vec<i32>,
}

impl TrayModule {
    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::Event(event) => match *event {
                ServiceEvent::Init(service) => {
                    self.service = Some(service);
                    Action::None
                }
                ServiceEvent::Update(data) => {
                    let action = if let TrayEvent::Unregistered(name) = &data {
                        Action::CloseTrayMenu(name.clone())
                    } else {
                        Action::None
                    };

                    if let Some(service) = self.service.as_mut() {
                        service.update(data);
                    }

                    action
                }
                ServiceEvent::Error(_) => Action::None,
            },
            Message::ToggleMenu(menu_type, id, button_ui_ref) => {
                Action::ToggleMenu(menu_type, id, button_ui_ref)
            }
            Message::ToggleSubmenu(index) => {
                if self.submenus.contains(&index) {
                    self.submenus.retain(|i| i != &index);
                } else {
                    self.submenus.push(index);
                }

                Action::None
            }
            Message::MenuSelected(name, id) => match self.service.as_mut() {
                Some(service) => {
                    debug!("Tray menu click: {id}");
                    Action::TrayMenuCommand(
                        service
                            .command(TrayCommand::MenuSelected(name, id))
                            .map(|event| Message::Event(Box::new(event))),
                    )
                }
                _ => Action::None,
            },
            Message::MenuOpened(name) => {
                if let Some(_tray) = self
                    .service
                    .as_ref()
                    .and_then(|t| t.iter().find(|t| t.name == name))
                {
                    self.submenus.clear();
                }

                Action::None
            }
        }
    }

    fn menu_voice<'a>(
        &'a self,
        theme: &'a AshellTheme,
        name: &'a str,
        layout: &'a Layout,
    ) -> Element<'a, Message> {
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

                    move |_| Message::MenuSelected(name.to_owned(), id)
                })
                .width(Length::Fill)
                .into(),
            LayoutProps {
                children_display: Some(display),
                label: Some(label),
                ..
            } if display == "submenu" => {
                let is_open = self.submenus.contains(&layout.0);
                Column::with_capacity(2)
                    .push(
                        button(row!(
                            text(label.replace("_", "").to_owned()).width(Length::Fill),
                            icon(if is_open {
                                StaticIcon::MenuOpen
                            } else {
                                StaticIcon::MenuClosed
                            })
                        ))
                        .style(theme.ghost_button_style())
                        .padding(theme.space.xs)
                        .on_press(Message::ToggleSubmenu(layout.0))
                        .width(Length::Fill),
                    )
                    .push_maybe(if is_open {
                        Some(
                            Column::with_children(
                                layout
                                    .2
                                    .iter()
                                    .map(|menu| self.menu_voice(theme, name, menu))
                                    .collect::<Vec<_>>(),
                            )
                            .padding([0, 0, 0, theme.space.md])
                            .spacing(theme.space.xxs),
                        )
                    } else {
                        None
                    })
                    .into()
            }
            LayoutProps {
                label: Some(label), ..
            } => button(text(label.replace("_", "")))
                .style(theme.ghost_button_style())
                .on_press(Message::MenuSelected(name.to_owned(), layout.0))
                .width(Length::Fill)
                .padding(theme.space.xs)
                .into(),
            LayoutProps { type_: Some(t), .. } if t == "separator" => horizontal_rule(1).into(),
            _ => Row::new().into(),
        }
    }

    pub fn view<'a>(&'a self, id: Id, theme: &'a AshellTheme) -> Option<Element<'a, Message>> {
        self.service
            .as_ref()
            .filter(|s| !s.data.is_empty())
            .map(|service| {
                Into::<Element<_>>::into(
                    Row::with_children(
                        service
                            .data
                            .iter()
                            .map(|item| {
                                position_button(match &item.icon {
                                    Some(TrayIcon::Image(handle)) => Into::<Element<_>>::into(
                                        Image::new(handle.clone())
                                            .height(Length::Fixed(theme.font_size.md as f32 - 2.0)),
                                    ),
                                    Some(TrayIcon::Svg(handle)) => Into::<Element<_>>::into(
                                        Svg::new(handle.clone())
                                            .height(Length::Fixed(theme.font_size.md as f32 + 2.))
                                            .width(Length::Fixed(theme.font_size.md as f32 + 2.))
                                            .content_fit(iced::ContentFit::Cover),
                                    ),
                                    _ => icon(StaticIcon::Point).into(),
                                })
                                .on_press_with_position(move |button_ui_ref| {
                                    Message::ToggleMenu(item.name.to_owned(), id, button_ui_ref)
                                })
                                .padding(theme.space.xxs)
                                .style(theme.ghost_button_style())
                                .into()
                            })
                            .collect::<Vec<_>>(),
                    )
                    .align_y(Alignment::Center),
                )
            })
    }

    pub fn menu_view<'a>(&'a self, theme: &'a AshellTheme, name: &'a str) -> Element<'a, Message> {
        container(
            match self
                .service
                .as_ref()
                .and_then(|service| service.data.iter().find(|item| item.name == name))
            {
                Some(item) => Column::with_children(
                    item.menu
                        .2
                        .iter()
                        .map(|menu| self.menu_voice(theme, name, menu)),
                )
                .spacing(theme.space.xs),
                _ => Column::new(),
            },
        )
        .max_width(MenuSize::Medium)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        TrayService::subscribe().map(|e| Message::Event(Box::new(e)))
    }
}
