use crate::{
    app::{App, Message},
    config::{AppearanceStyle, ModuleDef, ModuleName},
    menu::MenuType,
    position_button::position_button,
    theme::AshellTheme,
};
use iced::{
    Alignment, Border, Color, Element, Length, Subscription,
    widget::{Row, container, row},
    window::Id,
};

pub mod app_launcher;
pub mod clipboard;
pub mod clock;
pub mod custom_module;
pub mod keyboard_layout;
pub mod keyboard_submap;
pub mod media_player;
pub mod privacy;
pub mod settings;
pub mod system_info;
pub mod tempo;
pub mod tray;
pub mod updates;
pub mod window_title;
pub mod workspaces;

#[derive(Debug, Clone)]
pub enum OnModulePress {
    Action(Box<Message>),
    ToggleMenu(MenuType),
}

impl App {
    pub fn modules_section<'a>(
        &'a self,
        id: Id,
        theme: &'a AshellTheme,
    ) -> [Element<'a, Message>; 3] {
        [
            &self.general_config.modules.left,
            &self.general_config.modules.center,
            &self.general_config.modules.right,
        ]
        .map(|modules_def| {
            let mut row = row!()
                .height(Length::Shrink)
                .align_y(Alignment::Center)
                .spacing(self.theme.space.xxs);

            for module_def in modules_def {
                row = row.push_maybe(match module_def {
                    // life parsing of string to module
                    ModuleDef::Single(module) => self.single_module_wrapper(id, theme, module),
                    ModuleDef::Group(group) => self.group_module_wrapper(id, theme, group),
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

    fn single_module_wrapper<'a>(
        &'a self,
        id: Id,
        theme: &'a AshellTheme,
        module_name: &'a ModuleName,
    ) -> Option<Element<'a, Message>> {
        let module = self.get_module_view(id, module_name);

        module.map(|(content, action)| match action {
            Some(action) => {
                let button = position_button(
                    container(content)
                        .align_y(Alignment::Center)
                        .height(Length::Fill)
                        .clip(true),
                )
                .padding([2, self.theme.space.xs])
                .height(Length::Fill)
                .style(theme.module_button_style(false));

                match action {
                    OnModulePress::Action(action) => button.on_press(*action),
                    OnModulePress::ToggleMenu(menu_type) => {
                        button.on_press_with_position(move |button_ui_ref| {
                            Message::ToggleMenu(menu_type.clone(), id, button_ui_ref)
                        })
                    }
                }
                .into()
            }
            _ => {
                let container = container(content)
                    .padding([2, self.theme.space.xs])
                    .height(Length::Fill)
                    .align_y(Alignment::Center)
                    .clip(true);

                match self.theme.bar_style {
                    AppearanceStyle::Solid | AppearanceStyle::Gradient => container.into(),
                    AppearanceStyle::Islands => container
                        .style(|theme| container::Style {
                            background: Some(
                                theme
                                    .palette()
                                    .background
                                    .scale_alpha(self.theme.opacity)
                                    .into(),
                            ),
                            border: Border {
                                width: 0.0,
                                radius: self.theme.radius.lg.into(),
                                color: Color::TRANSPARENT,
                            },
                            ..container::Style::default()
                        })
                        .into(),
                }
            }
        })
    }

    fn group_module_wrapper<'a>(
        &'a self,
        id: Id,
        theme: &'a AshellTheme,
        group: &'a [ModuleName],
    ) -> Option<Element<'a, Message>> {
        let modules = group
            .iter()
            .filter_map(|module| self.get_module_view(id, module))
            .collect::<Vec<_>>();

        if modules.is_empty() {
            None
        } else {
            Some({
                let group = Row::with_children(
                    modules
                        .into_iter()
                        .map(|(content, action)| match action {
                            Some(action) => {
                                let button = position_button(
                                    container(content)
                                        .align_y(Alignment::Center)
                                        .height(Length::Fill)
                                        .clip(true),
                                )
                                .padding([2, self.theme.space.xs])
                                .height(Length::Fill)
                                .style(theme.module_button_style(true));

                                match action {
                                    OnModulePress::Action(action) => button.on_press(*action),
                                    OnModulePress::ToggleMenu(menu_type) => button
                                        .on_press_with_position(move |button_ui_ref| {
                                            Message::ToggleMenu(
                                                menu_type.clone(),
                                                id,
                                                button_ui_ref,
                                            )
                                        }),
                                }
                                .into()
                            }
                            _ => container(content)
                                .padding([2, self.theme.space.xs])
                                .height(Length::Fill)
                                .align_y(Alignment::Center)
                                .clip(true)
                                .into(),
                        })
                        .collect::<Vec<_>>(),
                );

                match self.theme.bar_style {
                    AppearanceStyle::Solid | AppearanceStyle::Gradient => group.into(),
                    AppearanceStyle::Islands => container(group)
                        .style(|theme| container::Style {
                            background: Some(
                                theme
                                    .palette()
                                    .background
                                    .scale_alpha(self.theme.opacity)
                                    .into(),
                            ),
                            border: Border {
                                width: 0.0,
                                radius: self.theme.radius.lg.into(),
                                color: Color::TRANSPARENT,
                            },
                            ..container::Style::default()
                        })
                        .into(),
                }
            })
        }
    }

    fn get_module_view<'a>(
        &'a self,
        id: Id,
        module_name: &'a ModuleName,
    ) -> Option<(Element<'a, Message>, Option<OnModulePress>)> {
        match module_name {
            ModuleName::AppLauncher => self.app_launcher.as_ref().map(|app_launcher| {
                (
                    app_launcher.view().map(Message::AppLauncher),
                    Some(OnModulePress::Action(Box::new(Message::AppLauncher(
                        app_launcher::Message::Launch,
                    )))),
                )
            }),
            ModuleName::Custom(name) => self.custom.get(name).map(|custom| {
                (
                    custom
                        .view(&self.theme)
                        .map(|msg| Message::Custom(name.clone(), msg)),
                    Some(OnModulePress::Action(Box::new(Message::Custom(
                        name.clone(),
                        custom_module::Message::LaunchCommand,
                    )))),
                )
            }),
            ModuleName::Updates => self.updates.as_ref().map(|updates| {
                (
                    updates.view(&self.theme).map(Message::Updates),
                    Some(OnModulePress::ToggleMenu(MenuType::Updates)),
                )
            }),
            ModuleName::Clipboard => self.clipboard.as_ref().map(|clipboard| {
                (
                    clipboard.view().map(Message::Clipboard),
                    Some(OnModulePress::Action(Box::new(Message::Clipboard(
                        clipboard::Message::Launch,
                    )))),
                )
            }),
            ModuleName::Workspaces => Some((
                self.workspaces
                    .view(id, &self.theme, &self.outputs)
                    .map(Message::Workspaces),
                None,
            )),
            ModuleName::WindowTitle => self.window_title.get_value().map(|title| {
                (
                    self.window_title
                        .view(&self.theme, title)
                        .map(Message::WindowTitle),
                    None,
                )
            }),
            ModuleName::SystemInfo => Some((
                self.system_info.view(&self.theme).map(Message::SystemInfo),
                Some(OnModulePress::ToggleMenu(MenuType::SystemInfo)),
            )),
            ModuleName::KeyboardLayout => self.keyboard_layout.view(&self.theme).map(|view| {
                (
                    view.map(Message::KeyboardLayout),
                    Some(OnModulePress::Action(Box::new(Message::KeyboardLayout(
                        keyboard_layout::Message::ChangeLayout,
                    )))),
                )
            }),
            ModuleName::KeyboardSubmap => self
                .keyboard_submap
                .view(&self.theme)
                .map(|view| (view.map(Message::KeyboardSubmap), None)),
            ModuleName::Tray => self
                .tray
                .view(id, &self.theme)
                .map(|view| (view.map(Message::Tray), None)),
            ModuleName::Clock => Some((self.clock.view(&self.theme).map(Message::Clock), None)),
            ModuleName::Tempo => Some((
                self.tempo.view(&self.theme).map(Message::Tempo),
                Some(OnModulePress::ToggleMenu(MenuType::Tempo)),
            )),
            ModuleName::Privacy => self
                .privacy
                .view(&self.theme)
                .map(|view| (view.map(Message::Privacy), None)),
            ModuleName::MediaPlayer => self.media_player.view(&self.theme).map(|view| {
                (
                    view.map(Message::MediaPlayer),
                    Some(OnModulePress::ToggleMenu(MenuType::MediaPlayer)),
                )
            }),
            ModuleName::Settings => Some((
                self.settings.view(&self.theme).map(Message::Settings),
                Some(OnModulePress::ToggleMenu(MenuType::Settings)),
            )),
        }
    }

    fn get_module_subscription(&self, module_name: &ModuleName) -> Option<Subscription<Message>> {
        match module_name {
            ModuleName::AppLauncher => None,
            ModuleName::Custom(name) => self.custom.get(name).map(|custom| {
                custom
                    .subscription()
                    .map(|(name, msg)| Message::Custom(name, msg))
            }),
            ModuleName::Updates => self
                .updates
                .as_ref()
                .map(|updates| updates.subscription().map(Message::Updates)),
            ModuleName::Clipboard => None,
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
            ModuleName::Clock => Some(self.clock.subscription().map(Message::Clock)),
            ModuleName::Tempo => Some(self.tempo.subscription().map(Message::Tempo)),
            ModuleName::Privacy => Some(self.privacy.subscription().map(Message::Privacy)),
            ModuleName::MediaPlayer => {
                Some(self.media_player.subscription().map(Message::MediaPlayer))
            }
            ModuleName::Settings => Some(self.settings.subscription().map(Message::Settings)),
        }
    }
}
