use crate::app::{self, WindowInfo};
use crate::config::Position;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::container::Style;
use iced::widget::mouse_area;
use iced::window::Id;
use iced::{self, widget::container, Element, Task, Theme};
use iced::{Border, Length, Padding};
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_runtime::{task, Action};

fn open_menu(menu_type: WindowInfo) -> Task<app::Message> {
    Task::done(app::Message::NewLayerShell {
        settings: iced_layershell::reexport::NewLayerShellSettings {
            size: None,
            layer: Layer::Overlay,
            anchor: Anchor::Top
                .union(Anchor::Left)
                .union(Anchor::Right)
                .union(Anchor::Bottom),
            exclusive_zone: None,
            margin: None,
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            use_last_output: true,
        },
        info: menu_type,
    })
}

pub fn close_menu<Message: 'static>(id: Id) -> Task<Message> {
    task::effect(Action::Window(iced_runtime::window::Action::Close(id)))
}

pub fn toggle(
    current: Option<(&Id, &mut WindowInfo)>,
    menu_type: WindowInfo,
) -> Task<app::Message> {
    match current {
        None => open_menu(menu_type),
        Some((id, current)) if *current == menu_type => close_menu(*id),
        Some((_, current)) => {
            *current = menu_type;
            Task::none()
        }
    }
}

pub fn close_if<Message: 'static>(
    current: Option<(&Id, &mut WindowInfo)>,
    menu_type: WindowInfo,
) -> Task<Message> {
    if let Some((id, current)) = current {
        if *current == menu_type {
            close_menu(*id)
        } else {
            Task::none()
        }
    } else {
        Task::none()
    }
}

pub enum MenuPosition {
    Left,
    Right,
}

pub fn menu_wrapper(
    content: Element<app::Message>,
    position: MenuPosition,
    bar_position: Position,
) -> Element<app::Message> {
    mouse_area(
        container(
            mouse_area(
                container(content)
                    .height(Length::Shrink)
                    .width(Length::Shrink)
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
        .align_x(match position {
            MenuPosition::Left => Horizontal::Left,
            MenuPosition::Right => Horizontal::Right,
        })
        .padding(Padding::new(8.).top(0))
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .on_release(app::Message::CloseMenu)
    .into()
}
