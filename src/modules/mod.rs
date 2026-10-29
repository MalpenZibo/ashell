use crate::{
    app::{App, Message},
    components::animated_size,
    components::menu::MenuType,
    components::{MouseButton, PressHandler, module_group, module_item},
    config::{MediaPlayerButtonAction, MediaPlayerScrollAction, ModuleDef, ModuleName},
    theme::use_theme,
};
use iced::{Alignment, Element, Length, Subscription, SurfaceId, widget::Row};

pub mod custom_module;
pub mod keyboard_layout;
pub mod keyboard_submap;
pub mod media_player;
pub mod notifications;
pub mod privacy;
pub mod settings;
pub mod system_info;
pub mod tempo;
pub mod tray;
pub mod updates;
pub mod window_title;
pub mod workspaces;

/// What one mouse button on a bar module does. Menu toggles are separate from
/// plain messages because opening a menu needs the button's on-screen position.
#[derive(Debug, Clone)]
enum ModuleAction {
    Message(Box<Message>),
    ToggleMenu(MenuType),
}

impl ModuleAction {
    fn message(message: Message) -> Self {
        Self::Message(Box::new(message))
    }

    fn menu(menu_type: MenuType) -> Self {
        Self::ToggleMenu(menu_type)
    }

    fn into_handler<'a>(self, id: SurfaceId) -> PressHandler<'a, Message> {
        match self {
            Self::Message(message) => PressHandler::Message(*message),
            Self::ToggleMenu(menu_type) => {
                PressHandler::WithPosition(Box::new(move |button_ui_ref| {
                    Message::ToggleMenu(menu_type.clone(), id, button_ui_ref)
                }))
            }
        }
    }
}

/// The handlers a bar module binds to its mouse buttons.
#[derive(Debug, Clone, Default)]
struct ModulePress {
    on_press: Option<ModuleAction>,
    on_right_press: Option<ModuleAction>,
    on_middle_press: Option<ModuleAction>,
    on_scroll_up: Option<Box<Message>>,
    on_scroll_down: Option<Box<Message>>,
}

impl ModulePress {
    fn press(action: ModuleAction) -> Self {
        Self {
            on_press: Some(action),
            ..Default::default()
        }
    }

    fn menu(menu_type: MenuType) -> Self {
        Self::press(ModuleAction::menu(menu_type))
    }

    fn message(message: Message) -> Self {
        Self::press(ModuleAction::message(message))
    }

    fn right_message(mut self, message: Message) -> Self {
        self.on_right_press = Some(ModuleAction::message(message));
        self
    }

    fn middle_message(mut self, message: Message) -> Self {
        self.on_middle_press = Some(ModuleAction::message(message));
        self
    }

    fn scroll_up(mut self, message: Message) -> Self {
        self.on_scroll_up = Some(Box::new(message));
        self
    }

    fn scroll_down(mut self, message: Message) -> Self {
        self.on_scroll_down = Some(Box::new(message));
        self
    }
}

impl App {
    pub fn modules_section<'a>(&'a self, id: SurfaceId) -> [Element<'a, Message>; 3] {
        let space = use_theme(|t| t.space);
        [
            &self.general_config.modules.left,
            &self.general_config.modules.center,
            &self.general_config.modules.right,
        ]
        .map(|modules_def| {
            let mut row = Row::with_capacity(modules_def.len())
                .height(Length::Shrink)
                .align_y(Alignment::Center)
                .spacing(space.xxs);

            for module_def in modules_def {
                row = row.push(match module_def {
                    // life parsing of string to module
                    ModuleDef::Single(module) => self.single_module_wrapper(id, module),
                    ModuleDef::Group(group) => self.group_module_wrapper(id, group),
                });
            }

            row.into()
        })
    }

    pub fn modules_subscriptions(&self, modules_def: &[ModuleDef]) -> Vec<Subscription<Message>> {
        modules_def
            .iter()
            .flat_map(|module_def| match module_def {
                ModuleDef::Single(module) => {
                    vec![self.get_module_subscription(module)]
                }
                ModuleDef::Group(group) => group
                    .iter()
                    .map(|module| self.get_module_subscription(module))
                    .collect(),
            })
            .flatten()
            .collect()
    }

    fn build_module_item<'a>(
        id: SurfaceId,
        content: Element<'a, Message>,
        press: Option<ModulePress>,
    ) -> Element<'a, Message> {
        let content = if use_theme(|t| t.animations_enabled) {
            animated_size(content).into()
        } else {
            content
        };
        let Some(press) = press else {
            return module_item(content).into();
        };

        module_item(content)
            .set_handler(
                MouseButton::Press,
                press.on_press.map(|action| action.into_handler(id)),
            )
            .set_handler(
                MouseButton::RightPress,
                press.on_right_press.map(|action| action.into_handler(id)),
            )
            .set_handler(
                MouseButton::MiddlePress,
                press.on_middle_press.map(|action| action.into_handler(id)),
            )
            .set_scroll_up(press.on_scroll_up.map(|message| *message))
            .set_scroll_down(press.on_scroll_down.map(|message| *message))
            .into()
    }

    fn single_module_wrapper<'a>(
        &'a self,
        id: SurfaceId,
        module_name: &'a ModuleName,
    ) -> Option<Element<'a, Message>> {
        self.get_module_view(id, module_name)
            .map(|(content, action)| module_group(Self::build_module_item(id, content, action)))
    }

    fn group_module_wrapper<'a>(
        &'a self,
        id: SurfaceId,
        group: &'a [ModuleName],
    ) -> Option<Element<'a, Message>> {
        let modules: Vec<_> = group
            .iter()
            .filter_map(|module| self.get_module_view(id, module))
            .collect();

        if modules.is_empty() {
            None
        } else {
            let items = Row::with_children(
                modules
                    .into_iter()
                    .map(|(content, action)| Self::build_module_item(id, content, action))
                    .collect::<Vec<_>>(),
            );
            Some(module_group(items.into()))
        }
    }

    fn get_module_view<'a>(
        &'a self,
        id: SurfaceId,
        module_name: &'a ModuleName,
    ) -> Option<(Element<'a, Message>, Option<ModulePress>)> {
        match module_name {
            ModuleName::Custom(name) => self.custom.get(name).map(|custom| {
                let action = match custom.module_type() {
                    crate::config::CustomModuleType::Text => None,
                    crate::config::CustomModuleType::Button => {
                        let custom_message = |message| Message::Custom(name.clone(), message);
                        let mut press = ModulePress::message(custom_message(
                            custom_module::Message::LaunchCommand,
                        ));
                        if custom.config.on_right_click.is_some() {
                            press = press.right_message(custom_message(
                                custom_module::Message::LaunchRightClickCommand,
                            ));
                        }
                        if custom.config.on_middle_click.is_some() {
                            press = press.middle_message(custom_message(
                                custom_module::Message::LaunchMiddleClickCommand,
                            ));
                        }
                        if custom.config.on_scroll_up.is_some() {
                            press = press.scroll_up(custom_message(
                                custom_module::Message::LaunchScrollUpCommand,
                            ));
                        }
                        if custom.config.on_scroll_down.is_some() {
                            press = press.scroll_down(custom_message(
                                custom_module::Message::LaunchScrollDownCommand,
                            ));
                        }
                        Some(press)
                    }
                };
                (
                    custom
                        .view(id)
                        .map(|msg| Message::Custom(name.clone(), msg)),
                    action,
                )
            }),
            ModuleName::Updates => self.updates.as_ref().map(|updates| {
                (
                    updates.view().map(Message::Updates),
                    Some(ModulePress::menu(MenuType::Updates)),
                )
            }),
            ModuleName::Workspaces => Some((
                self.workspaces
                    .view(id, &self.outputs)
                    .map(Message::Workspaces),
                None,
            )),
            ModuleName::WindowTitle => self.window_title.get_value().map(|title| {
                (
                    self.window_title.view(title).map(Message::WindowTitle),
                    None,
                )
            }),
            ModuleName::SystemInfo => Some((
                self.system_info.view().map(Message::SystemInfo),
                Some(ModulePress::menu(MenuType::SystemInfo)),
            )),
            ModuleName::KeyboardLayout => self.keyboard_layout.view().map(|view| {
                (
                    view.map(Message::KeyboardLayout),
                    Some(ModulePress::message(Message::KeyboardLayout(
                        keyboard_layout::Message::ChangeLayout,
                    ))),
                )
            }),
            ModuleName::KeyboardSubmap => self
                .keyboard_submap
                .view()
                .map(|view| (view.map(Message::KeyboardSubmap), None)),
            ModuleName::Tray => self
                .tray
                .view(id)
                .map(|view| (view.map(Message::Tray), None)),
            ModuleName::Tempo => Some((
                self.tempo.view().map(Message::Tempo),
                Some(
                    ModulePress::menu(MenuType::Tempo)
                        .right_message(Message::Tempo(tempo::Message::CycleFormat))
                        .scroll_up(Message::Tempo(tempo::Message::CycleTimezone(
                            tempo::TimezoneDirection::Forward,
                        )))
                        .scroll_down(Message::Tempo(tempo::Message::CycleTimezone(
                            tempo::TimezoneDirection::Backward,
                        ))),
                ),
            )),
            ModuleName::Privacy => self
                .privacy
                .view()
                .map(|view| (view.map(Message::Privacy), None)),
            ModuleName::MediaPlayer => {
                let controls = self.media_player.indicator_controls();
                let button = |action: MediaPlayerButtonAction| match action {
                    MediaPlayerButtonAction::None => None,
                    MediaPlayerButtonAction::Menu => {
                        Some(ModuleAction::menu(MenuType::MediaPlayer))
                    }
                    MediaPlayerButtonAction::Prev => Some(ModuleAction::message(
                        Message::MediaPlayer(media_player::Message::ActivePrev),
                    )),
                    MediaPlayerButtonAction::PlayPause => Some(ModuleAction::message(
                        Message::MediaPlayer(media_player::Message::ActivePlayPause),
                    )),
                    MediaPlayerButtonAction::Next => Some(ModuleAction::message(
                        Message::MediaPlayer(media_player::Message::ActiveNext),
                    )),
                };
                let (on_scroll_up, on_scroll_down) = match controls.scroll {
                    MediaPlayerScrollAction::None => (None, None),
                    MediaPlayerScrollAction::Volume => (
                        Some(Box::new(Message::MediaPlayer(
                            media_player::Message::ActiveVolumeUp,
                        ))),
                        Some(Box::new(Message::MediaPlayer(
                            media_player::Message::ActiveVolumeDown,
                        ))),
                    ),
                };
                let action = ModulePress {
                    on_press: button(controls.left),
                    on_right_press: button(controls.right),
                    on_middle_press: button(controls.middle),
                    on_scroll_up,
                    on_scroll_down,
                };
                self.media_player
                    .view()
                    .map(|view| (view.map(Message::MediaPlayer), Some(action)))
            }
            ModuleName::Settings => Some((
                self.settings.view(id).map(Message::Settings),
                Some(ModulePress::menu(MenuType::Settings)),
            )),
            ModuleName::Notifications => Some((
                self.notifications.view().map(Message::Notifications),
                Some(
                    ModulePress::menu(MenuType::Notifications)
                        .right_message(Message::Notifications(notifications::Message::ToggleDnd)),
                ),
            )),
        }
    }

    fn get_module_subscription(&self, module_name: &ModuleName) -> Option<Subscription<Message>> {
        match module_name {
            ModuleName::Custom(name) => self.custom.get(name).map(|custom| {
                custom
                    .subscription()
                    .map(|(name, msg)| Message::Custom(name, msg))
            }),
            ModuleName::Updates => self
                .updates
                .as_ref()
                .map(|updates| updates.subscription().map(Message::Updates)),
            ModuleName::Workspaces => Some(self.workspaces.subscription().map(Message::Workspaces)),
            ModuleName::WindowTitle => {
                Some(self.window_title.subscription().map(Message::WindowTitle))
            }
            ModuleName::SystemInfo => {
                Some(self.system_info.subscription().map(Message::SystemInfo))
            }
            ModuleName::KeyboardLayout => Some(
                self.keyboard_layout
                    .subscription()
                    .map(Message::KeyboardLayout),
            ),
            ModuleName::KeyboardSubmap => Some(
                self.keyboard_submap
                    .subscription()
                    .map(Message::KeyboardSubmap),
            ),
            ModuleName::Tray => Some(self.tray.subscription().map(Message::Tray)),
            ModuleName::Tempo => Some(self.tempo.subscription().map(Message::Tempo)),
            ModuleName::Privacy => Some(self.privacy.subscription().map(Message::Privacy)),
            ModuleName::MediaPlayer => Some(
                self.media_player
                    .subscription(self.outputs.menu_of_type_is_open(&MenuType::MediaPlayer))
                    .map(Message::MediaPlayer),
            ),
            ModuleName::Settings => Some(self.settings.subscription().map(Message::Settings)),
            ModuleName::Notifications => Some(
                self.notifications
                    .subscription()
                    .map(Message::Notifications),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builders_bind_the_expected_buttons() {
        let press = ModulePress::menu(MenuType::Settings)
            .right_message(Message::None)
            .middle_message(Message::None);

        assert!(matches!(press.on_press, Some(ModuleAction::ToggleMenu(_))));
        assert!(matches!(
            press.on_right_press,
            Some(ModuleAction::Message(_))
        ));
        assert!(matches!(
            press.on_middle_press,
            Some(ModuleAction::Message(_))
        ));
    }
}
