use crate::{
    components::divider,
    components::icons::{StaticIcon, icon},
    components::{
        ButtonHierarchy, ButtonKind, ButtonUIRef, IconPosition, MenuSize, position_button,
        styled_button,
    },
    config::{TrayClickAction, TrayModuleConfig},
    services::{
        ReadOnlyService, Service, ServiceEvent,
        tray::{
            TrayCommand, TrayEvent, TrayIcon, TrayService,
            dbus::{Layout, LayoutProps},
        },
    },
    theme::use_theme,
};
use iced::{
    Alignment, Element, Length, Padding, Subscription, SurfaceId, Task,
    widget::{Column, Image, Row, Svg, container, scrollable, text, toggler},
};
use log::debug;

const MENU_MAX_HEIGHT: f32 = 600.;

#[derive(Debug, Clone)]
pub enum Message {
    Event(Box<ServiceEvent<TrayService>>),
    ToggleMenu(String, SurfaceId, ButtonUIRef),
    ToggleSubmenu(i32),
    MenuSelected(String, i32),
    MenuToggled(String, i32),
    MenuOpened(String),
    Activate(String),
}

pub enum Action {
    None,
    ToggleMenu(String, SurfaceId, ButtonUIRef),
    TrayMenuCommand(Task<Message>),
    TrayMenuCommandKeepOpen(Task<Message>),
    CloseTrayMenu(String),
}

#[derive(Debug, Clone)]
pub struct TrayModule {
    service: Option<TrayService>,
    submenus: Vec<i32>,
    blocklist: Vec<crate::config::RegexCfg>,
    right_click: Option<TrayClickAction>,
}

impl TrayModule {
    pub fn new(config: TrayModuleConfig) -> Self {
        Self {
            service: None,
            submenus: Vec::new(),
            blocklist: config.blocklist.clone(),
            right_click: config.right_click,
        }
    }

    fn is_blocklisted(&self, name: &str) -> bool {
        self.blocklist.iter().any(|pattern| pattern.is_match(name))
    }

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
            Message::MenuToggled(name, id) => match self.service.as_mut() {
                Some(service) => {
                    debug!("Tray menu toggle: {id}");
                    Action::TrayMenuCommandKeepOpen(
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
            Message::Activate(name) => match self.service.as_mut() {
                Some(service) => {
                    debug!("Tray item activate: {name}");
                    Action::TrayMenuCommand(
                        service
                            .command(TrayCommand::Activate(name))
                            .map(|event| Message::Event(Box::new(event))),
                    )
                }
                _ => Action::None,
            },
        }
    }

    fn menu_voice<'a>(&'a self, name: &'a str, layout: &'a Layout) -> Element<'a, Message> {
        let space = use_theme(|theme| theme.space);
        match &layout.1 {
            LayoutProps {
                label: Some(label),
                toggle_type: Some(toggle_type),
                toggle_state: Some(state),
                children_display: None,
                ..
            } if toggle_type == "checkmark" => {
                let content: Element<'a, Message> = toggler(*state > 0)
                    .label(label.replace("_", "").to_owned())
                    .on_toggle({
                        let name = name.to_owned();
                        let id = layout.0;

                        move |_| Message::MenuToggled(name.to_owned(), id)
                    })
                    .width(Length::Fill)
                    .into();
                styled_button(content)
                    .on_press(Message::MenuToggled(name.to_owned(), layout.0))
                    .width(Length::Fill)
                    .into()
            }
            LayoutProps {
                children_display: Some(display),
                label: Some(label),
                toggle_type,
                toggle_state,
                ..
            } if display == "submenu" => {
                let is_open = self.submenus.contains(&layout.0);
                let content: Element<'a, Message> = match (toggle_type.as_deref(), toggle_state) {
                    (Some("checkmark"), &Some(state)) => Row::new()
                        .push(toggler(state > 0).on_toggle({
                            let name = name.to_owned();
                            let id = layout.0;

                            move |_| Message::MenuToggled(name.to_owned(), id)
                        }))
                        .push(text(label.replace("_", "")))
                        .spacing(space.sm)
                        .align_y(Alignment::Center)
                        .into(),
                    _ => text(label.replace("_", "")).into(),
                };
                Column::with_capacity(2)
                    .push(
                        styled_button(content)
                            .icon(
                                if is_open {
                                    StaticIcon::MenuOpen
                                } else {
                                    StaticIcon::MenuClosed
                                },
                                IconPosition::After,
                            )
                            .on_press(Message::ToggleSubmenu(layout.0))
                            .width(Length::Fill),
                    )
                    .push(if is_open {
                        Some(
                            Column::with_children(
                                layout
                                    .2
                                    .iter()
                                    .filter(|menu| menu.1.visible != Some(false))
                                    .map(|menu| self.menu_voice(name, menu))
                                    .collect::<Vec<_>>(),
                            )
                            .padding(Padding::default().left(space.md))
                            .spacing(space.xxs),
                        )
                    } else {
                        None
                    })
                    .into()
            }
            LayoutProps {
                label: Some(label), ..
            } if !label.is_empty() => styled_button(label.replace("_", ""))
                .on_press(Message::MenuSelected(name.to_owned(), layout.0))
                .width(Length::Fill)
                .into(),
            LayoutProps { type_: Some(t), .. } if t == "separator" => divider(),
            _ => Row::new().into(),
        }
    }

    pub fn view<'a>(&'a self, id: SurfaceId) -> Option<Element<'a, Message>> {
        let (space, font_size, button_style) = use_theme(|theme| {
            (
                theme.space,
                theme.font_size,
                theme.button_style(ButtonKind::Transparent, ButtonHierarchy::Secondary),
            )
        });
        let button_style = std::sync::Arc::new(button_style);

        self.service
            .as_ref()
            .filter(|s| !s.data.is_empty())
            .map(|service| {
                Into::<Element<_>>::into(
                    Row::with_children(
                        service
                            .data
                            .iter()
                            .filter(|item| !self.is_blocklisted(&item.name))
                            .map(|item| {
                                let name = item.name.to_owned();
                                let button_style = button_style.clone();
                                let icon_content: Element<'_, Message> = match &item.icon {
                                    Some(TrayIcon::Image(handle)) => Image::new(handle.clone())
                                        .height(Length::Fixed(font_size.md - 2.0))
                                        .into(),
                                    Some(TrayIcon::Svg(handle)) => Svg::new(handle.clone())
                                        .height(Length::Fixed(font_size.md + 2.))
                                        .width(Length::Fixed(font_size.md + 2.))
                                        .content_fit(iced::ContentFit::Cover)
                                        .into(),
                                    _ => icon(StaticIcon::Point).into(),
                                };
                                let open_app = Message::Activate(name.clone());
                                let toggle_menu = move |r| Message::ToggleMenu(name.clone(), id, r);

                                let mut btn = position_button(icon_content);
                                btn = match &self.right_click {
                                    None => btn.on_press_with_position(toggle_menu.clone()),
                                    Some(TrayClickAction::Open) => btn
                                        .on_press_with_position(toggle_menu.clone())
                                        .on_right_press(open_app.clone()),
                                    Some(TrayClickAction::Menu) => btn
                                        .on_press(open_app.clone())
                                        .on_right_press_with_position(toggle_menu.clone()),
                                };
                                btn.padding(space.xxs)
                                    .style(move |t, s| button_style(t, s))
                                    .into()
                            })
                            .collect::<Vec<_>>(),
                    )
                    .align_y(Alignment::Center),
                )
            })
    }

    pub fn menu_view<'a>(&'a self, name: &'a str) -> Element<'a, Message> {
        let space = use_theme(|theme| theme.space);
        let items = match self
            .service
            .as_ref()
            .and_then(|service| service.data.iter().find(|item| item.name == name))
        {
            Some(item) => Column::with_children(
                item.menu
                    .2
                    .iter()
                    .filter(|menu| menu.1.visible != Some(false))
                    .map(|menu| self.menu_voice(name, menu)),
            )
            .spacing(space.xs),
            _ => Column::new(),
        };

        container(scrollable(items).spacing(space.xs))
            .width(MenuSize::Medium)
            .max_height(MENU_MAX_HEIGHT)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        TrayService::subscribe().map(|e| Message::Event(Box::new(e)))
    }
}
