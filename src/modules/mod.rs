use crate::{
    app::{self, App, Message},
    config::{AppearanceStyle, ModuleDef, ModuleName},
    menu::MenuType,
    position_button::position_button,
    style::module_button_style,
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
pub mod tray;
pub mod updates;
pub mod window_title;
pub mod workspaces;

use log::error;

#[derive(Debug, Clone)]
pub enum OnModulePress {
    Action(Box<Message>),
    ToggleMenu(MenuType),
}

pub trait Module {
    type ViewData<'a>;
    type SubscriptionData<'a>;

    fn view(
        &self,
        data: Self::ViewData<'_>,
    ) -> Option<(Element<app::Message>, Option<OnModulePress>)>;

    fn subscription(&self, _: Self::SubscriptionData<'_>) -> Option<Subscription<app::Message>> {
        None
    }
}

impl App {
    pub fn modules_section(
        &self,
        modules_def: &Vec<ModuleDef>,
        id: Id,
        opacity: f32,
    ) -> Element<Message> {
        let mut row = row!()
            .height(Length::Shrink)
            .align_y(Alignment::Center)
            .spacing(4);

        for module_def in modules_def {
            row = row.push_maybe(match module_def {
                // life parsing of string to module
                ModuleDef::Single(module) => self.single_module_wrapper(module, id, opacity),
                ModuleDef::Group(group) => self.group_module_wrapper(group, id, opacity),
            });
        }

        row.into()
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

    fn single_module_wrapper(
        &self,
        module_name: &ModuleName,
        id: Id,
        opacity: f32,
    ) -> Option<Element<Message>> {
        let module = self.get_module_view(module_name, id, opacity);

        module.map(|(content, action)| match action {
            Some(action) => {
                let button = position_button(
                    container(content)
                        .align_y(Alignment::Center)
                        .height(Length::Fill),
                )
                .padding([2, 8])
                .height(Length::Fill)
                .style(module_button_style(
                    self.config.appearance.style,
                    self.config.appearance.opacity,
                    false,
                ));

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
                    .padding([2, 8])
                    .height(Length::Fill)
                    .align_y(Alignment::Center);

                match self.config.appearance.style {
                    AppearanceStyle::Solid | AppearanceStyle::Gradient => container.into(),
                    AppearanceStyle::Islands => container
                        .style(|theme| container::Style {
                            background: Some(
                                theme
                                    .palette()
                                    .background
                                    .scale_alpha(self.config.appearance.opacity)
                                    .into(),
                            ),
                            border: Border {
                                width: 0.0,
                                radius: 12.0.into(),
                                color: Color::TRANSPARENT,
                            },
                            ..container::Style::default()
                        })
                        .into(),
                }
            }
        })
    }

    fn group_module_wrapper(
        &self,
        group: &[ModuleName],
        id: Id,
        opacity: f32,
    ) -> Option<Element<Message>> {
        let modules = group
            .iter()
            .filter_map(|module| self.get_module_view(module, id, opacity))
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
                                        .height(Length::Fill),
                                )
                                .padding([2, 8])
                                .height(Length::Fill)
                                .style(module_button_style(
                                    self.config.appearance.style,
                                    self.config.appearance.opacity,
                                    true,
                                ));

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
                                .padding([2, 8])
                                .height(Length::Fill)
                                .align_y(Alignment::Center)
                                .into(),
                        })
                        .collect::<Vec<_>>(),
                );

                match self.config.appearance.style {
                    AppearanceStyle::Solid | AppearanceStyle::Gradient => group.into(),
                    AppearanceStyle::Islands => container(group)
                        .style(|theme| container::Style {
                            background: Some(
                                theme
                                    .palette()
                                    .background
                                    .scale_alpha(self.config.appearance.opacity)
                                    .into(),
                            ),
                            border: Border {
                                width: 0.0,
                                radius: 12.0.into(),
                                color: Color::TRANSPARENT,
                            },
                            ..container::Style::default()
                        })
                        .into(),
                }
            })
        }
    }

    fn get_module_view(
        &self,
        module_name: &ModuleName,
        id: Id,
        opacity: f32,
    ) -> Option<(Element<Message>, Option<OnModulePress>)> {
        match module_name {
            ModuleName::AppLauncher => self.app_launcher.view(&self.config.app_launcher_cmd),
            ModuleName::Custom(name) => self
                .config
                .custom_modules
                .iter()
                .find(|m| &m.name == name)
                .and_then(|mc| self.custom.get(name).map(|cm| cm.view(mc)))
                .unwrap_or_else(|| {
                    error!("Custom module `{name}` not found");
                    None
                }),
            ModuleName::Updates => self.updates.view(&self.config.updates),
            ModuleName::Clipboard => self.clipboard.view(&self.config.clipboard_cmd),
            ModuleName::Workspaces => self.workspaces.view((
                &self.outputs,
                id,
                &self.config.workspaces,
                &self.config.appearance.workspace_colors,
                self.config.appearance.special_workspace_colors.as_deref(),
            )),
            ModuleName::WindowTitle => self.window_title.view(()),
            ModuleName::SystemInfo => self.system_info.view(&self.config.system),
            ModuleName::KeyboardLayout => self.keyboard_layout.view(&self.config.keyboard_layout),
            ModuleName::KeyboardSubmap => self.keyboard_submap.view(()),
            ModuleName::Tray => self.tray.view((id, opacity)),
            ModuleName::Clock => self.clock.view(&self.config.clock.format),
            ModuleName::Privacy => self.privacy.view(()),
            ModuleName::Settings => self.settings.view(()),
            ModuleName::MediaPlayer => self.media_player.view(&self.config.media_player),
        }
    }

    fn get_module_subscription(&self, module_name: &ModuleName) -> Option<Subscription<Message>> {
        match module_name {
            ModuleName::AppLauncher => self.app_launcher.subscription(()),
            ModuleName::Custom(name) => self
                .config
                .custom_modules
                .iter()
                .find(|m| &m.name == name)
                .and_then(|mc| self.custom.get(name).map(|cm| cm.subscription(mc)))
                .unwrap_or_else(|| {
                    error!("Custom module def `{name}` not found");
                    None
                }),
            ModuleName::Updates => self
                .config
                .updates
                .as_ref()
                .and_then(|updates_config| self.updates.subscription(updates_config)),
            ModuleName::Clipboard => self.clipboard.subscription(()),
            ModuleName::Workspaces => self.workspaces.subscription(&self.config.workspaces),
            ModuleName::WindowTitle => self.window_title.subscription(()),
            ModuleName::SystemInfo => self.system_info.subscription(()),
            ModuleName::KeyboardLayout => self.keyboard_layout.subscription(()),
            ModuleName::KeyboardSubmap => self.keyboard_submap.subscription(()),
            ModuleName::Tray => self.tray.subscription(()),
            ModuleName::Clock => self.clock.subscription(&self.config.clock.format),
            ModuleName::Privacy => self.privacy.subscription(()),
            ModuleName::Settings => self.settings.subscription(()),
            ModuleName::MediaPlayer => self.media_player.subscription(()),
        }
    }
}
