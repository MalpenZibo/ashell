use crate::app::{self};
use crate::config::{AppearanceStyle, Position};
use crate::position_button::ButtonUIRef;
use crate::style::backdrop_color;
use iced::alignment::{Horizontal, Vertical};
use iced::platform_specific::shell::commands::layer_surface::{
    KeyboardInteractivity, Layer, set_keyboard_interactivity, set_layer,
};
use iced::widget::container::Style;
use iced::widget::mouse_area;
use iced::window::Id;
use iced::{self, Element, Task, Theme, widget::container};
use iced::{Border, Length, Padding};

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum MenuType {
    Updates,
    Settings,
    Tray(String),
    MediaPlayer,
    SystemInfo,
}

#[derive(Clone, Debug)]
pub struct Menu {
    pub id: Id,
    pub menu_info: Option<(MenuType, ButtonUIRef)>,
}

impl Menu {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            menu_info: None,
        }
    }

    pub fn open<Message: 'static>(
        &mut self,
        menu_type: MenuType,
        button_ui_ref: ButtonUIRef,
        config: &crate::config::Config,
    ) -> Task<Message> {
        self.menu_info.replace((menu_type, button_ui_ref));

        let mut tasks = vec![set_layer(self.id, Layer::Overlay)];

        if config.menu_keyboard_focus {
            tasks.push(set_keyboard_interactivity(self.id, KeyboardInteractivity::OnDemand));
        }

        Task::batch(tasks)
    }

    pub fn close<Message: 'static>(&mut self, config: &crate::config::Config) -> Task<Message> {
        if self.menu_info.is_some() {
            self.menu_info.take();

            let mut tasks = vec![set_layer(self.id, Layer::Background)];
            
            if config.menu_keyboard_focus {
                tasks.push(set_keyboard_interactivity(self.id, KeyboardInteractivity::None));
            }

            Task::batch(tasks)

        } else {
            Task::none()
        }
    }

    pub fn toggle<Message: 'static>(
        &mut self,
        menu_type: MenuType,
        button_ui_ref: ButtonUIRef,
        config: &crate::config::Config,
    ) -> Task<Message> {
        match self.menu_info.as_mut() {
            None => self.open(menu_type, button_ui_ref, config),
            Some((current_type, _)) if *current_type == menu_type => self.close(config),
            Some((current_type, current_button_ui_ref)) => {
                *current_type = menu_type;
                *current_button_ui_ref = button_ui_ref;
                Task::none()
            }
        }
    }

    pub fn close_if<Message: 'static>(&mut self, menu_type: MenuType, config: &crate::config::Config) -> Task<Message> {
        if let Some((current_type, _)) = self.menu_info.as_ref() {
            if *current_type == menu_type {
                self.close(config)
            } else {
                Task::none()
            }
        } else {
            Task::none()
        }
    }

    pub fn request_keyboard<Message: 'static>(&self, menu_keyboard_focus: bool) -> Task<Message> {
        if menu_keyboard_focus {
            set_keyboard_interactivity(self.id, KeyboardInteractivity::OnDemand)
        } else {
            Task::none()
        }
    }

    pub fn release_keyboard<Message: 'static>(&self, menu_keyboard_focus: bool) -> Task<Message> {
        if menu_keyboard_focus {
            set_keyboard_interactivity(self.id, KeyboardInteractivity::None)
        } else {
            Task::none()
        }
    }
}

pub enum MenuSize {
    Small,
    Medium,
    Large,
}

impl MenuSize {
    fn size(&self) -> f32 {
        match self {
            MenuSize::Small => 250.,
            MenuSize::Medium => 350.,
            MenuSize::Large => 450.,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn menu_wrapper(
    id: Id,
    content: Element<app::Message>,
    menu_size: MenuSize,
    button_ui_ref: ButtonUIRef,
    bar_position: Position,
    style: AppearanceStyle,
    opacity: f32,
    menu_backdrop: f32,
) -> Element<app::Message> {
    mouse_area(
        container(
            mouse_area(
                container(content)
                    .height(Length::Shrink)
                    .width(Length::Shrink)
                    .max_width(menu_size.size())
                    .padding(16)
                    .style(move |theme: &Theme| Style {
                        background: Some(theme.palette().background.scale_alpha(opacity).into()),
                        border: Border {
                            color: theme
                                .extended_palette()
                                .secondary
                                .base
                                .color
                                .scale_alpha(opacity),
                            width: 1.,
                            radius: 16.0.into(),
                        },
                        ..Default::default()
                    }),
            )
            .on_release(app::Message::None),
        )
        .align_y(match bar_position {
            Position::Top => Vertical::Top,
            Position::Bottom => Vertical::Bottom,
        })
        .align_x(Horizontal::Left)
        .padding({
            let size = menu_size.size();

            let v_padding = match style {
                AppearanceStyle::Solid | AppearanceStyle::Gradient => 2,
                AppearanceStyle::Islands => 0,
            };

            Padding::new(0.)
                .top(if bar_position == Position::Top {
                    v_padding
                } else {
                    0
                })
                .bottom(if bar_position == Position::Bottom {
                    v_padding
                } else {
                    0
                })
                .left(f32::min(
                    f32::max(button_ui_ref.position.x - size / 2., 8.),
                    button_ui_ref.viewport.0 - size - 8.,
                ))
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| Style {
            background: Some(backdrop_color(menu_backdrop).into()),
            ..Default::default()
        }),
    )
    .on_release(app::Message::CloseMenu(id))
    .into()
}
