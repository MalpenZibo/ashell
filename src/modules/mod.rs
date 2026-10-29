use crate::{
    app::{App, Message},
    components::animated_size,
    components::menu::MenuType,
    components::{MouseButton, module_group, module_item},
    config::{MediaPlayerButtonAction, MediaPlayerScrollAction, ModuleDef, ModuleName},
    theme::use_theme,
};
use iced::{Alignment, Element, Length, Subscription, SurfaceId, widget::Row};

mod interaction;

use interaction::{ModuleAction, ModuleInteractions, ModuleView};

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

    fn build_module_item<'a>(id: SurfaceId, module: ModuleView<'a>) -> Element<'a, Message> {
        let (content, interactions) = module.into_parts();

        let content = if use_theme(|t| t.animations_enabled) {
            animated_size(content).into()
        } else {
            content
        };

        match interactions {
            Some(interactions) => interactions.into_item(id, content).into(),
            None => module_item(content).into(),
        }
    }

    fn single_module_wrapper<'a>(
        &'a self,
        id: SurfaceId,
        module_name: &'a ModuleName,
    ) -> Option<Element<'a, Message>> {
        self.get_module_view(id, module_name)
            .map(|module| module_group(Self::build_module_item(id, module)))
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
                    .map(|module| Self::build_module_item(id, module))
                    .collect::<Vec<_>>(),
            );
            Some(module_group(items.into()))
        }
    }

    fn get_module_view<'a>(
        &'a self,
        id: SurfaceId,
        module_name: &'a ModuleName,
    ) -> Option<ModuleView<'a>> {
        match module_name {
            ModuleName::Custom(name) => self.custom.get(name).map(|custom| {
                let content = custom
                    .view(id)
                    .map(|msg| Message::Custom(name.clone(), msg));
                let interactions = match custom.module_type() {
                    crate::config::CustomModuleType::Text => None,
                    crate::config::CustomModuleType::Button => {
                        let name = name.clone();
                        Some(
                            ModuleInteractions::default()
                                .button(
                                    MouseButton::Left,
                                    ModuleAction::message(Message::Custom(
                                        name.clone(),
                                        custom_module::Message::LaunchCommand,
                                    )),
                                )
                                .button_opt(
                                    MouseButton::Right,
                                    custom.config.on_right_click.as_ref().map(|_| {
                                        ModuleAction::message(Message::Custom(
                                            name.clone(),
                                            custom_module::Message::LaunchRightClickCommand,
                                        ))
                                    }),
                                )
                                .button_opt(
                                    MouseButton::Middle,
                                    custom.config.on_middle_click.as_ref().map(|_| {
                                        ModuleAction::message(Message::Custom(
                                            name.clone(),
                                            custom_module::Message::LaunchMiddleClickCommand,
                                        ))
                                    }),
                                )
                                .scroll_up_opt(custom.config.on_scroll_up.as_ref().map(|_| {
                                    Message::Custom(
                                        name.clone(),
                                        custom_module::Message::LaunchScrollUpCommand,
                                    )
                                }))
                                .scroll_down_opt(custom.config.on_scroll_down.as_ref().map(|_| {
                                    Message::Custom(
                                        name,
                                        custom_module::Message::LaunchScrollDownCommand,
                                    )
                                })),
                        )
                    }
                };
                ModuleView::new(content, interactions)
            }),
            ModuleName::Updates => self.updates.as_ref().map(|updates| {
                ModuleView::menu(updates.view().map(Message::Updates), MenuType::Updates)
            }),
            ModuleName::Workspaces => Some(ModuleView::plain(
                self.workspaces
                    .view(id, &self.outputs)
                    .map(Message::Workspaces),
            )),
            ModuleName::WindowTitle => self.window_title.get_value().map(|title| {
                ModuleView::plain(self.window_title.view(title).map(Message::WindowTitle))
            }),
            ModuleName::SystemInfo => Some(ModuleView::menu(
                self.system_info.view().map(Message::SystemInfo),
                MenuType::SystemInfo,
            )),
            ModuleName::KeyboardLayout => self.keyboard_layout.view().map(|view| {
                ModuleView::message(
                    view.map(Message::KeyboardLayout),
                    Message::KeyboardLayout(keyboard_layout::Message::ChangeLayout),
                )
            }),
            ModuleName::KeyboardSubmap => self
                .keyboard_submap
                .view()
                .map(|view| ModuleView::plain(view.map(Message::KeyboardSubmap))),
            ModuleName::Tray => self
                .tray
                .view(id)
                .map(|view| ModuleView::plain(view.map(Message::Tray))),
            ModuleName::Tempo => Some(ModuleView::interactive(
                self.tempo.view().map(Message::Tempo),
                ModuleInteractions::menu(MenuType::Tempo)
                    .right_message(Message::Tempo(tempo::Message::CycleFormat))
                    .scroll_up(Message::Tempo(tempo::Message::CycleTimezone(
                        tempo::TimezoneDirection::Forward,
                    )))
                    .scroll_down(Message::Tempo(tempo::Message::CycleTimezone(
                        tempo::TimezoneDirection::Backward,
                    ))),
            )),
            ModuleName::Privacy => self
                .privacy
                .view()
                .map(|view| ModuleView::plain(view.map(Message::Privacy))),
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
                let interactions = ModuleInteractions::default()
                    .button_opt(MouseButton::Left, button(controls.left))
                    .button_opt(MouseButton::Right, button(controls.right))
                    .button_opt(MouseButton::Middle, button(controls.middle));
                let interactions = match controls.scroll {
                    MediaPlayerScrollAction::None => interactions,
                    MediaPlayerScrollAction::Volume => interactions
                        .scroll_up(Message::MediaPlayer(media_player::Message::ActiveVolumeUp))
                        .scroll_down(Message::MediaPlayer(
                            media_player::Message::ActiveVolumeDown,
                        )),
                };
                self.media_player.view().map(|view| {
                    ModuleView::interactive(view.map(Message::MediaPlayer), interactions)
                })
            }
            ModuleName::Settings => Some(ModuleView::menu(
                self.settings.view(id).map(Message::Settings),
                MenuType::Settings,
            )),
            ModuleName::Notifications => Some(ModuleView::interactive(
                self.notifications.view().map(Message::Notifications),
                ModuleInteractions::menu(MenuType::Notifications)
                    .right_message(Message::Notifications(notifications::Message::ToggleDnd)),
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
