use crate::{
    app::Message,
    components::{ModuleItem, MouseButton, PressHandler, menu::MenuType, module_item},
};
use iced::{Element, SurfaceId};

/// What one mouse button on a bar module does. Menu toggles are separate from
/// plain messages because opening a menu needs the button's on-screen position.
#[derive(Debug, Clone)]
pub(crate) enum ModuleAction {
    Message(Box<Message>),
    ToggleMenu(MenuType),
}

impl ModuleAction {
    pub(crate) fn message(message: Message) -> Self {
        Self::Message(Box::new(message))
    }

    pub(crate) fn menu(menu_type: MenuType) -> Self {
        Self::ToggleMenu(menu_type)
    }

    pub(crate) fn into_handler<'a>(self, id: SurfaceId) -> PressHandler<'a, Message> {
        match self {
            Self::Message(message) => PressHandler::message(*message),
            Self::ToggleMenu(menu_type) => PressHandler::with_position(move |button_ui_ref| {
                Message::ToggleMenu(menu_type.clone(), id, button_ui_ref)
            }),
        }
    }
}

/// The interactions a bar module binds to its mouse buttons and scroll wheel.
#[derive(Debug, Clone, Default)]
pub(crate) struct ModuleInteractions {
    buttons: [Option<ModuleAction>; 3],
    scroll_up: Option<Box<Message>>,
    scroll_down: Option<Box<Message>>,
}

impl ModuleInteractions {
    pub(crate) fn button(mut self, button: MouseButton, action: ModuleAction) -> Self {
        self.buttons[button.index()] = Some(action);
        self
    }

    pub(crate) fn button_opt(mut self, button: MouseButton, action: Option<ModuleAction>) -> Self {
        self.buttons[button.index()] = action;
        self
    }

    pub(crate) fn menu(menu_type: MenuType) -> Self {
        Self::default().button(MouseButton::Left, ModuleAction::menu(menu_type))
    }

    pub(crate) fn message(message: Message) -> Self {
        Self::default().button(MouseButton::Left, ModuleAction::message(message))
    }

    pub(crate) fn right_message(self, message: Message) -> Self {
        self.button(MouseButton::Right, ModuleAction::message(message))
    }

    pub(crate) fn scroll_up(mut self, message: Message) -> Self {
        self.scroll_up = Some(Box::new(message));
        self
    }

    pub(crate) fn scroll_up_opt(mut self, message: Option<Message>) -> Self {
        self.scroll_up = message.map(Box::new);
        self
    }

    pub(crate) fn scroll_down(mut self, message: Message) -> Self {
        self.scroll_down = Some(Box::new(message));
        self
    }

    pub(crate) fn scroll_down_opt(mut self, message: Option<Message>) -> Self {
        self.scroll_down = message.map(Box::new);
        self
    }

    pub(crate) fn into_item<'a>(
        self,
        id: SurfaceId,
        content: Element<'a, Message>,
    ) -> ModuleItem<'a, Message> {
        let mut item = module_item(content);
        for (button, action) in MouseButton::ALL.into_iter().zip(self.buttons) {
            item = item.set_handler(button, action.map(|action| action.into_handler(id)));
        }
        item.set_scroll_up(self.scroll_up.map(|message| *message))
            .set_scroll_down(self.scroll_down.map(|message| *message))
    }
}

/// A module's rendered content plus the interactions it binds to it.
pub(crate) struct ModuleView<'a> {
    content: Element<'a, Message>,
    interactions: Option<ModuleInteractions>,
}

impl<'a> ModuleView<'a> {
    pub(crate) fn new(
        content: Element<'a, Message>,
        interactions: Option<ModuleInteractions>,
    ) -> Self {
        Self {
            content,
            interactions,
        }
    }

    pub(crate) fn plain(content: Element<'a, Message>) -> Self {
        Self::new(content, None)
    }

    pub(crate) fn interactive(
        content: Element<'a, Message>,
        interactions: ModuleInteractions,
    ) -> Self {
        Self::new(content, Some(interactions))
    }

    pub(crate) fn menu(content: Element<'a, Message>, menu_type: MenuType) -> Self {
        Self::interactive(content, ModuleInteractions::menu(menu_type))
    }

    pub(crate) fn message(content: Element<'a, Message>, message: Message) -> Self {
        Self::interactive(content, ModuleInteractions::message(message))
    }

    pub(crate) fn into_parts(self) -> (Element<'a, Message>, Option<ModuleInteractions>) {
        (self.content, self.interactions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_buttons_match_their_index() {
        for (index, button) in MouseButton::ALL.into_iter().enumerate() {
            assert_eq!(button.index(), index);
        }
    }

    #[test]
    fn builders_target_the_right_button() {
        let interactions = ModuleInteractions::default()
            .button(MouseButton::Right, ModuleAction::menu(MenuType::Settings))
            .button(MouseButton::Middle, ModuleAction::menu(MenuType::Settings));

        assert!(interactions.buttons[MouseButton::Left.index()].is_none());
        assert!(interactions.buttons[MouseButton::Right.index()].is_some());
        assert!(interactions.buttons[MouseButton::Middle.index()].is_some());
    }

    #[test]
    fn optional_builder_clears_a_handler() {
        let interactions = ModuleInteractions::default()
            .button(MouseButton::Left, ModuleAction::menu(MenuType::Settings))
            .button_opt(MouseButton::Left, None);

        assert!(interactions.buttons[MouseButton::Left.index()].is_none());
    }
}
