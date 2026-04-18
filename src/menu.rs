use crate::app::{self, App};
use crate::config::{AppearanceStyle, Position};
use crate::theme::backdrop_color;
use crate::widgets::{self, ButtonUIRef};
use iced::alignment::Vertical;
use iced::widget::container::Style;
use iced::{
    Anchor, Border, Element, KeyboardInteractivity, Layer, LayerShellSettings, Length, OutputId,
    Padding, Pixels, SurfaceId, Task, Theme, destroy_layer_surface, new_layer_surface,
    set_keyboard_interactivity, widget::container,
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
pub struct OpenMenu {
    pub id: SurfaceId,
    pub menu_type: MenuType,
    pub button_ui_ref: ButtonUIRef,
}

#[derive(Clone, Debug)]
pub struct Menu {
    pub open: Option<OpenMenu>,
}

impl Menu {
    pub fn new() -> Self {
        Self { open: None }
    }

    pub fn surface_id(&self) -> Option<SurfaceId> {
        self.open.as_ref().map(|o| o.id)
    }

    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    pub fn open<Message: 'static>(
        &mut self,
        menu_type: MenuType,
        button_ui_ref: ButtonUIRef,
        request_keyboard: bool,
        output_id: Option<OutputId>,
    ) -> Task<Message> {
        let keyboard_interactivity = if request_keyboard {
            KeyboardInteractivity::OnDemand
        } else {
            KeyboardInteractivity::None
        };

        let (menu_id, task) = new_layer_surface(LayerShellSettings {
            namespace: "ashell-menu-layer".to_string(),
            size: None,
            layer: Layer::Overlay,
            keyboard_interactivity,
            output: output_id,
            anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
            ..Default::default()
        });

        self.open = Some(OpenMenu {
            id: menu_id,
            menu_type,
            button_ui_ref,
        });
        task
    }

    pub fn close<Message: 'static>(&mut self) -> Task<Message> {
        if let Some(open) = self.open.take() {
            destroy_layer_surface(open.id)
        } else {
            Task::none()
        }
    }

    pub fn toggle<Message: 'static>(
        &mut self,
        menu_type: MenuType,
        button_ui_ref: ButtonUIRef,
        request_keyboard: bool,
        output_id: Option<OutputId>,
    ) -> Task<Message> {
        match &mut self.open {
            None => self.open(menu_type, button_ui_ref, request_keyboard, output_id),
            Some(open) if open.menu_type == menu_type => self.close(),
            Some(open) => {
                open.menu_type = menu_type;
                open.button_ui_ref = button_ui_ref;
                Task::none()
            }
        }
    }

    pub fn close_if<Message: 'static>(&mut self, menu_type: MenuType) -> Task<Message> {
        if self.open.as_ref().is_some_and(|o| o.menu_type == menu_type) {
            self.close()
        } else {
            Task::none()
        }
    }

    pub fn request_keyboard<Message: 'static>(&self) -> Task<Message> {
        if let Some(open) = &self.open {
            set_keyboard_interactivity(open.id, KeyboardInteractivity::OnDemand)
        } else {
            Task::none()
        }
    }

    pub fn release_keyboard<Message: 'static>(&self) -> Task<Message> {
        if let Some(open) = &self.open {
            set_keyboard_interactivity(open.id, KeyboardInteractivity::None)
        } else {
            Task::none()
        }
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
        widgets::MenuWrapper::new(
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
