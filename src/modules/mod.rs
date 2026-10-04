use crate::{
    app::{App, Message},
    components::animated_size,
    components::menu::MenuType,
    components::{module_group, module_item},
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
pub enum ModuleButtonAction {
    Message(Box<Message>),
    ToggleMenu(MenuType),
}

impl ModuleButtonAction {
    pub fn message(message: Message) -> Self {
        Self::Message(Box::new(message))
    }
}

#[derive(Debug, Clone)]
pub enum OnModulePress {
    Action(Box<Message>),
    ToggleMenu(MenuType),
    ToggleMenuWithExtra {
        menu_type: MenuType,
        on_right_press: Option<Box<Message>>,
        on_scroll_up: Option<Box<Message>>,
        on_scroll_down: Option<Box<Message>>,
    },
    CustomAction {
        on_press: Option<ModuleButtonAction>,
        on_right_press: Option<ModuleButtonAction>,
        on_middle_press: Option<ModuleButtonAction>,
        on_scroll_up: Option<Box<Message>>,
        on_scroll_down: Option<Box<Message>>,
    },
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
        &'a self,
        id: SurfaceId,
        content: Element<'a, Message>,
        action: Option<OnModulePress>,
    ) -> Element<'a, Message> {
        let content = if use_theme(|t| t.animations_enabled) {
            animated_size(content).into()
        } else {
            content
        };
        match action {
            Some(action) => {
                let mut item = module_item(content);
                match action {
                    OnModulePress::Action(msg) => {
                        item = item.on_press(*msg);
                    }
                    OnModulePress::ToggleMenu(menu_type) => {
                        item = item.on_press_with_position(move |button_ui_ref| {
                            Message::ToggleMenu(menu_type.clone(), id, button_ui_ref)
                        });
                    }
                    OnModulePress::ToggleMenuWithExtra {
                        menu_type,
                        on_right_press,
                        on_scroll_up,
                        on_scroll_down,
                    } => {
                        item = item.on_press_with_position(move |button_ui_ref| {
                            Message::ToggleMenu(menu_type.clone(), id, button_ui_ref)
                        });
                        if let Some(msg) = on_right_press {
                            item = item.on_right_press(*msg);
                        }
                        if let Some(msg) = on_scroll_up {
                            item = item.on_scroll_up(*msg);
                        }
                        if let Some(msg) = on_scroll_down {
                            item = item.on_scroll_down(*msg);
                        }
                    }
                    OnModulePress::CustomAction {
                        on_press,
                        on_right_press,
                        on_middle_press,
                        on_scroll_up,
                        on_scroll_down,
                    } => {
                        match on_press {
                            Some(ModuleButtonAction::Message(msg)) => {
                                item = item.on_press(*msg);
                            }
                            Some(ModuleButtonAction::ToggleMenu(menu_type)) => {
                                item = item.on_press_with_position(move |button_ui_ref| {
                                    Message::ToggleMenu(menu_type.clone(), id, button_ui_ref)
                                });
                            }
                            None => {}
                        }
                        match on_right_press {
                            Some(ModuleButtonAction::Message(msg)) => {
                                item = item.on_right_press(*msg);
                            }
                            Some(ModuleButtonAction::ToggleMenu(menu_type)) => {
                                item = item.on_right_press_with_position(move |button_ui_ref| {
                                    Message::ToggleMenu(menu_type.clone(), id, button_ui_ref)
                                });
                            }
                            None => {}
                        }
                        match on_middle_press {
                            Some(ModuleButtonAction::Message(msg)) => {
                                item = item.on_middle_press(*msg);
                            }
                            Some(ModuleButtonAction::ToggleMenu(menu_type)) => {
                                item = item.on_middle_press_with_position(move |button_ui_ref| {
                                    Message::ToggleMenu(menu_type.clone(), id, button_ui_ref)
                                });
                            }
                            None => {}
                        }
                        if let Some(msg) = on_scroll_up {
                            item = item.on_scroll_up(*msg);
                        }
                        if let Some(msg) = on_scroll_down {
                            item = item.on_scroll_down(*msg);
                        }
                    }
                }
                item.into()
            }
            None => module_item(content).into(),
        }
    }

    fn single_module_wrapper<'a>(
        &'a self,
        id: SurfaceId,
        module_name: &'a ModuleName,
    ) -> Option<Element<'a, Message>> {
        self.get_module_view(id, module_name)
            .map(|(content, action)| module_group(self.build_module_item(id, content, action)))
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
                    .map(|(content, action)| self.build_module_item(id, content, action))
                    .collect::<Vec<_>>(),
            );
            Some(module_group(items.into()))
        }
    }

    fn get_module_view<'a>(
        &'a self,
        id: SurfaceId,
        module_name: &'a ModuleName,
    ) -> Option<(Element<'a, Message>, Option<OnModulePress>)> {
        match module_name {
            ModuleName::Custom(name) => self.custom.get(name).map(|custom| {
                let action = match custom.module_type() {
                    crate::config::CustomModuleType::Text => None,
                    crate::config::CustomModuleType::Button => {
                        let name = name.clone();
                        Some(OnModulePress::CustomAction {
                            on_press: Some(ModuleButtonAction::message(Message::Custom(
                                name.clone(),
                                custom_module::Message::LaunchCommand,
                            ))),
                            on_right_press: custom.config.on_right_click.as_ref().map(|_| {
                                ModuleButtonAction::message(Message::Custom(
                                    name.clone(),
                                    custom_module::Message::LaunchRightClickCommand,
                                ))
                            }),
                            on_middle_press: custom.config.on_middle_click.as_ref().map(|_| {
                                ModuleButtonAction::message(Message::Custom(
                                    name.clone(),
                                    custom_module::Message::LaunchMiddleClickCommand,
                                ))
                            }),
                            on_scroll_up: custom
                                .config
                                .on_scroll_up
                                .as_ref()
                                .map(|_| {
                                    Message::Custom(
                                        name.clone(),
                                        custom_module::Message::LaunchScrollUpCommand,
                                    )
                                })
                                .map(Box::new),
                            on_scroll_down: custom
                                .config
                                .on_scroll_down
                                .as_ref()
                                .map(|_| {
                                    Message::Custom(
                                        name,
                                        custom_module::Message::LaunchScrollDownCommand,
                                    )
                                })
                                .map(Box::new),
                        })
                    }
                };
                (
                    custom.view().map(|msg| Message::Custom(name.clone(), msg)),
                    action,
                )
            }),
            ModuleName::Updates => self.updates.as_ref().map(|updates| {
                (
                    updates.view().map(Message::Updates),
                    Some(OnModulePress::ToggleMenu(MenuType::Updates)),
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
                Some(OnModulePress::ToggleMenu(MenuType::SystemInfo)),
            )),
            ModuleName::KeyboardLayout => self.keyboard_layout.view().map(|view| {
                (
                    view.map(Message::KeyboardLayout),
                    Some(OnModulePress::Action(Box::new(Message::KeyboardLayout(
                        keyboard_layout::Message::ChangeLayout,
                    )))),
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
                Some(OnModulePress::ToggleMenuWithExtra {
                    menu_type: MenuType::Tempo,
                    on_right_press: Some(Box::new(Message::Tempo(tempo::Message::CycleFormat))),
                    on_scroll_up: Some(Box::new(Message::Tempo(tempo::Message::CycleTimezone(
                        tempo::TimezoneDirection::Forward,
                    )))),
                    on_scroll_down: Some(Box::new(Message::Tempo(tempo::Message::CycleTimezone(
                        tempo::TimezoneDirection::Backward,
                    )))),
                }),
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
                        Some(ModuleButtonAction::ToggleMenu(MenuType::MediaPlayer))
                    }
                    MediaPlayerButtonAction::Prev => Some(ModuleButtonAction::message(
                        Message::MediaPlayer(media_player::Message::ActivePrev),
                    )),
                    MediaPlayerButtonAction::PlayPause => Some(ModuleButtonAction::message(
                        Message::MediaPlayer(media_player::Message::ActivePlayPause),
                    )),
                    MediaPlayerButtonAction::Next => Some(ModuleButtonAction::message(
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
                let action = OnModulePress::CustomAction {
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
                Some(OnModulePress::ToggleMenu(MenuType::Settings)),
            )),
            ModuleName::Notifications => Some((
                self.notifications.view().map(Message::Notifications),
                Some(OnModulePress::ToggleMenu(MenuType::Notifications)),
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
