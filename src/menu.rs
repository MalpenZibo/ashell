use crate::app::{self};
use crate::config::Position;
use crate::position_button::ButtonUIRef;
use iced::alignment::{Horizontal, Vertical};
use iced::platform_specific::shell::commands::layer_surface::{
    set_keyboard_interactivity, set_layer, KeyboardInteractivity, Layer,
};
use iced::widget::container::Style;
use iced::widget::mouse_area;
use iced::window::Id;
use iced::{self, widget::container, Element, Task, Theme};
use iced::{Border, Length, Padding};

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum MenuType {
    Updates,
    Settings,
    Tray(String),
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
    ) -> Task<Message> {
        self.menu_info.replace((menu_type, button_ui_ref));

        Task::batch(vec![
            set_layer(self.id, Layer::Overlay),
            set_keyboard_interactivity(self.id, KeyboardInteractivity::None),
        ])
    }

    pub fn close<Message: 'static>(&mut self) -> Task<Message> {
        if self.menu_info.is_some() {
            self.menu_info.take();

            Task::batch(vec![
                set_layer(self.id, Layer::Background),
                set_keyboard_interactivity(self.id, KeyboardInteractivity::None),
            ])
        } else {
            Task::none()
        }
    }

    pub fn toggle<Message: 'static>(
        &mut self,
        menu_type: MenuType,
        button_ui_ref: ButtonUIRef,
    ) -> Task<Message> {
        match self.menu_info.as_mut() {
            None => self.open(menu_type, button_ui_ref),
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

pub enum MenuSize {
    Normal,
    Large,
}

impl MenuSize {
    fn size(&self) -> f32 {
        match self {
            MenuSize::Normal => 250.,
            MenuSize::Large => 350.,
        }
    }
}

pub fn menu_wrapper(
    id: Id,
    content: Element<app::Message>,
    menu_size: MenuSize,
    button_ui_ref: ButtonUIRef,
    bar_position: Position,
) -> Element<app::Message> {
    mouse_area(
        container(
            mouse_area(
                container(content)
                    .height(Length::Shrink)
                    .width(Length::Shrink)
                    .max_width(menu_size.size())
                    .padding(16)
                    .style(|theme: &Theme| Style {
                        background: Some(theme.palette().background.into()),
                        border: Border {
                            color: theme.extended_palette().secondary.base.color,
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

            Padding::new(0.).left(f32::min(
                f32::max(button_ui_ref.position.x - size / 2., 8.),
                button_ui_ref.viewport.0 - size - 8.,
            ))
        })
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .on_release(app::Message::CloseMenu(id))
    .into()
}
