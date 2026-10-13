use crate::{components::position_button, config::ModuleStyle, theme::use_theme};
use iced::{Alignment, Element, Length, widget::container};

use super::ButtonUIRef;

/// A handler for one mouse button: a plain message, or one built from the
/// button's on-screen position (needed to place a menu popup under it).
enum Handler<'a, Msg> {
    Message(Msg),
    WithPosition(Box<dyn Fn(ButtonUIRef) -> Msg + 'a>),
}

/// Builder for a bar module item: content wrapped in a position_button
/// with optional press, right-press, middle-press, scroll-up, and scroll-down handlers.
///
/// When no handler at all is set, renders as a plain container.
pub struct ModuleItem<'a, Msg> {
    content: Element<'a, Msg>,
    // NOTE: implement styling
    #[allow(dead_code)]
    style: Option<ModuleStyle>,
    on_press: Option<Handler<'a, Msg>>,
    on_right_press: Option<Handler<'a, Msg>>,
    on_middle_press: Option<Handler<'a, Msg>>,
    on_scroll_up: Option<Msg>,
    on_scroll_down: Option<Msg>,
}

pub fn module_item<'a, Msg: 'static + Clone>(
    content: Element<'a, Msg>,
    style: Option<ModuleStyle>,
) -> ModuleItem<'a, Msg> {
    ModuleItem {
        content,
        style,
        on_press: None,
        on_right_press: None,
        on_middle_press: None,
        on_scroll_up: None,
        on_scroll_down: None,
    }
}

impl<'a, Msg: 'static + Clone> ModuleItem<'a, Msg> {
    pub fn on_press(mut self, msg: Msg) -> Self {
        self.on_press = Some(Handler::Message(msg));
        self
    }

    pub fn on_press_with_position(mut self, handler: impl Fn(ButtonUIRef) -> Msg + 'a) -> Self {
        self.on_press = Some(Handler::WithPosition(Box::new(handler)));
        self
    }

    pub fn on_right_press(mut self, msg: Msg) -> Self {
        self.on_right_press = Some(Handler::Message(msg));
        self
    }

    pub fn on_right_press_with_position(
        mut self,
        handler: impl Fn(ButtonUIRef) -> Msg + 'a,
    ) -> Self {
        self.on_right_press = Some(Handler::WithPosition(Box::new(handler)));
        self
    }

    pub fn on_middle_press(mut self, msg: Msg) -> Self {
        self.on_middle_press = Some(Handler::Message(msg));
        self
    }

    pub fn on_middle_press_with_position(
        mut self,
        handler: impl Fn(ButtonUIRef) -> Msg + 'a,
    ) -> Self {
        self.on_middle_press = Some(Handler::WithPosition(Box::new(handler)));
        self
    }

    pub fn on_scroll_up(mut self, msg: Msg) -> Self {
        self.on_scroll_up = Some(msg);
        self
    }

    pub fn on_scroll_down(mut self, msg: Msg) -> Self {
        self.on_scroll_down = Some(msg);
        self
    }
}

impl<'a, Msg: 'static + Clone> From<ModuleItem<'a, Msg>> for Element<'a, Msg> {
    fn from(item: ModuleItem<'a, Msg>) -> Self {
        let (space, module_button_style) =
            use_theme(|theme| (theme.space, theme.module_button_style()));

        // A module can bind only, say, right-click or scroll, so the button has
        // to be built whenever any handler is set, not just on left press.
        let has_action = item.on_press.is_some()
            || item.on_right_press.is_some()
            || item.on_middle_press.is_some()
            || item.on_scroll_up.is_some()
            || item.on_scroll_down.is_some();

        if has_action {
            let mut button = position_button(
                container(item.content)
                    .padding([0.0, 1.0])
                    .align_y(Alignment::Center)
                    .height(Length::Fill)
                    .clip(true),
            )
            .padding([0.0, space.xs - 1.0])
            .height(Length::Fill)
            .style(module_button_style);

            match item.on_press {
                Some(Handler::Message(msg)) => button = button.on_press(msg),
                Some(Handler::WithPosition(handler)) => {
                    button = button.on_press_with_position(handler);
                }
                None => {}
            }
            match item.on_right_press {
                Some(Handler::Message(msg)) => button = button.on_right_press(msg),
                Some(Handler::WithPosition(handler)) => {
                    button = button.on_right_press_with_position(handler);
                }
                None => {}
            }
            match item.on_middle_press {
                Some(Handler::Message(msg)) => button = button.on_middle_press(msg),
                Some(Handler::WithPosition(handler)) => {
                    button = button.on_middle_press_with_position(handler);
                }
                None => {}
            }
            if let Some(msg) = item.on_scroll_up {
                button = button.on_scroll_up(msg);
            }
            if let Some(msg) = item.on_scroll_down {
                button = button.on_scroll_down(msg);
            }

            button.into()
        } else {
            container(item.content)
                .padding([0.0, space.xs])
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .clip(true)
                .into()
        }
    }
}
