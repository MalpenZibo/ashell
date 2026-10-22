use super::position_button::{MouseButton, PressHandler, position_button};
use crate::theme::use_theme;
use iced::{Alignment, Element, Length, widget::container};

/// Builder for a bar module item: content wrapped in a position_button
/// with optional per-button handlers and scroll-up/scroll-down handlers.
///
/// When no handler at all is set, renders as a plain container.
pub struct ModuleItem<'a, Msg> {
    content: Element<'a, Msg>,
    handlers: [Option<PressHandler<'a, Msg>>; 3],
    on_scroll_up: Option<Msg>,
    on_scroll_down: Option<Msg>,
}

pub fn module_item<'a, Msg: 'static + Clone>(content: Element<'a, Msg>) -> ModuleItem<'a, Msg> {
    ModuleItem {
        content,
        handlers: [None, None, None],
        on_scroll_up: None,
        on_scroll_down: None,
    }
}

impl<'a, Msg: 'static + Clone> ModuleItem<'a, Msg> {
    pub fn set_handler(
        mut self,
        button: MouseButton,
        handler: Option<PressHandler<'a, Msg>>,
    ) -> Self {
        self.handlers[button.index()] = handler;
        self
    }

    pub fn set_scroll_up(mut self, msg: Option<Msg>) -> Self {
        self.on_scroll_up = msg;
        self
    }

    pub fn set_scroll_down(mut self, msg: Option<Msg>) -> Self {
        self.on_scroll_down = msg;
        self
    }
}

impl<'a, Msg: 'static + Clone> From<ModuleItem<'a, Msg>> for Element<'a, Msg> {
    fn from(item: ModuleItem<'a, Msg>) -> Self {
        let (space, module_button_style) =
            use_theme(|theme| (theme.space, theme.module_button_style()));

        let ModuleItem {
            content,
            handlers,
            on_scroll_up,
            on_scroll_down,
        } = item;

        // A module can bind only, say, right-click or scroll, so the button has
        // to be built whenever any handler is set, not just on left press.
        let has_action = handlers.iter().any(Option::is_some)
            || on_scroll_up.is_some()
            || on_scroll_down.is_some();

        if has_action {
            let mut button = position_button(
                container(content)
                    .padding([0.0, 1.0])
                    .align_y(Alignment::Center)
                    .height(Length::Fill)
                    .clip(true),
            )
            .padding([0.0, space.xs - 1.0])
            .height(Length::Fill)
            .style(module_button_style);

            for (mouse_button, handler) in MouseButton::ALL.into_iter().zip(handlers) {
                if let Some(handler) = handler {
                    button = button.on(mouse_button, handler);
                }
            }
            if let Some(msg) = on_scroll_up {
                button = button.on_scroll_up(msg);
            }
            if let Some(msg) = on_scroll_down {
                button = button.on_scroll_down(msg);
            }

            button.into()
        } else {
            container(content)
                .padding([0.0, space.xs])
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .clip(true)
                .into()
        }
    }
}
