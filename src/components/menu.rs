use crate::app::{self, App};
use crate::components::{self, ButtonUIRef};
use crate::config::{AppearanceStyle, Position};
use crate::theme::backdrop_color;
use iced::alignment::Vertical;
use iced::widget::container::Style;
use iced::{
    Border, Element, KeyboardInteractivity, Layer, Length, Padding, Pixels, SurfaceId, Task, Theme,
    set_keyboard_interactivity, set_layer, widget::container,
};

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum MenuType {
    Updates,
    Settings,
    Notifications,
    Tray(String),
    MediaPlayer,
    SystemInfo,
    Tempo,
}

#[derive(Clone, Debug)]
pub struct Menu {
    pub id: SurfaceId,
    pub menu_info: Option<(MenuType, ButtonUIRef)>,
}

impl Menu {
    pub fn new(id: SurfaceId) -> Self {
        Self {
            id,
            menu_info: None,
        }
    }

    pub fn open<Message: 'static>(
        &mut self,
        menu_type: MenuType,
        button_ui_ref: ButtonUIRef,
        request_keyboard: bool,
    ) -> Task<Message> {
        self.menu_info.replace((menu_type, button_ui_ref));

        let mut tasks = vec![set_layer(self.id, Layer::Overlay)];

        if request_keyboard {
            tasks.push(set_keyboard_interactivity(
                self.id,
                KeyboardInteractivity::OnDemand,
            ));
        }

        Task::batch(tasks)
    }

    pub fn close<Message: 'static>(&mut self) -> Task<Message> {
        if self.menu_info.is_some() {
            self.menu_info.take();

            let mut tasks = vec![set_layer(self.id, Layer::Background)];

            tasks.push(set_keyboard_interactivity(
                self.id,
                KeyboardInteractivity::None,
            ));

            Task::batch(tasks)
        } else {
            Task::none()
        }
    }

    pub fn toggle<Message: 'static>(
        &mut self,
        menu_type: MenuType,
        button_ui_ref: ButtonUIRef,
        request_keyboard: bool,
    ) -> Task<Message> {
        match self.menu_info.as_mut() {
            None => self.open(menu_type, button_ui_ref, request_keyboard),
            Some((current_type, _)) if *current_type == menu_type => self.close(),
            Some((current_type, current_button_ui_ref)) => {
                *current_type = menu_type;
                *current_button_ui_ref = button_ui_ref;
                Task::none()
            }
        }
    }

    pub fn close_if<Message: 'static>(&mut self, menu_type: MenuType) -> Task<Message> {
        if let Some((current_type, _)) = self.menu_info.as_ref() {
            if *current_type == menu_type {
                self.close()
            } else {
                Task::none()
            }
        } else {
            Task::none()
        }
    }

    pub fn request_keyboard<Message: 'static>(&self) -> Task<Message> {
        set_keyboard_interactivity(self.id, KeyboardInteractivity::OnDemand)
    }

    pub fn release_keyboard<Message: 'static>(&self) -> Task<Message> {
        set_keyboard_interactivity(self.id, KeyboardInteractivity::None)
    }
}

#[allow(unused)]
pub enum MenuSize {
    Small,
    Medium,
    Large,
    XLarge,
}

impl MenuSize {
    pub fn size(&self) -> f32 {
        match self {
            MenuSize::Small => 250.,
            MenuSize::Medium => 350.,
            MenuSize::Large => 450.,
            MenuSize::XLarge => 650.,
        }
    }
}

impl From<MenuSize> for Length {
    fn from(value: MenuSize) -> Self {
        Length::Fixed(value.size())
    }
}

impl From<MenuSize> for Pixels {
    fn from(value: MenuSize) -> Self {
        Pixels::from(value.size())
    }
}

impl App {
    #[allow(clippy::too_many_arguments)]
    pub fn menu_wrapper<'a>(
        &'a self,
        id: SurfaceId,
        content: Element<'a, app::Message>,
        button_ui_ref: ButtonUIRef,
    ) -> Element<'a, app::Message> {
        components::MenuWrapper::new(
            button_ui_ref.position.x,
            container(content)
                .padding(self.theme.space.md)
                .style(move |theme: &Theme| Style {
                    background: Some(
                        theme
                            .palette()
                            .background
                            .scale_alpha(self.theme.menu.opacity)
                            .into(),
                    ),
                    border: Border {
                        color: theme
                            .extended_palette()
                            .background
                            .weakest
                            .color
                            .scale_alpha(self.theme.menu.opacity),
                        width: 1.,
                        radius: self.theme.radius.lg.into(),
                    },
                    ..Default::default()
                })
                .width(Length::Shrink)
                .into(),
        )
        .padding({
            let v_padding = match self.theme.bar_style {
                AppearanceStyle::Solid | AppearanceStyle::Gradient => 2,
                AppearanceStyle::Islands => 0,
            };

            Padding::new(0.)
                .top(if self.theme.bar_position == Position::Top {
                    v_padding
                } else {
                    0
                })
                .bottom(if self.theme.bar_position == Position::Bottom {
                    v_padding
                } else {
                    0
                })
        })
        .align_y(match self.theme.bar_position {
            Position::Top => Vertical::Top,
            Position::Bottom => Vertical::Bottom,
        })
        .backdrop(backdrop_color(self.theme.menu.backdrop))
        .on_click_outside(app::Message::CloseMenu(id))
        .into()
    }
}
