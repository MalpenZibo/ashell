use crate::{
    app::{App, Message},
    config::{ModuleDef, ModuleName},
    menu::MenuType,
    position_button::position_button,
    style::{
        module_first_label, module_label, module_last_label, module_middle_label, ModuleButtonStyle,
    },
};
use iced::{
    widget::{container, row, Row},
    window::Id,
    Alignment, Element, Length,
};

pub mod clipboard;
pub mod clock;
pub mod keyboard_layout;
pub mod keyboard_submap;
pub mod launcher;
pub mod privacy;
pub mod settings;
pub mod system_info;
pub mod tray;
pub mod updates;
pub mod window_title;
pub mod workspaces;

#[derive(Debug, Clone)]
pub enum OnModulePress {
    Action(Message),
    ToggleMenu(MenuType),
}

pub trait Module {
    type Data<'a>;

    fn view<'a>(&self, data: Self::Data<'a>) -> Option<(Element<Message>, Option<OnModulePress>)>;
}

#[derive(Debug, Clone)]
enum ModuleGroupPosition {
    Only,
    First,
    Middle,
    Last,
}

impl App {
    pub fn modules_section(&self, modules_def: &Vec<ModuleDef>, id: Id) -> Element<Message> {
        let mut row = row!()
            .height(Length::Shrink)
            .align_y(Alignment::Center)
            .spacing(4);

        for module_def in modules_def {
            row = row.push_maybe(match module_def {
                ModuleDef::Single(module) => self.single_module_wrapper(*module, id),
                ModuleDef::Group(group) => self.group_module_wrapper(group, id),
            });
        }

        row.into()
    }

    fn single_module_wrapper(&self, module_name: ModuleName, id: Id) -> Option<Element<Message>> {
        let module = self.get_module(module_name, id);

        module.map(|(content, action)| {
            if let Some(action) = action {
                let button = position_button(
                    container(content)
                        .align_y(Alignment::Center)
                        .height(Length::Fill),
                )
                .padding([2, 8])
                .height(Length::Fill)
                .style(ModuleButtonStyle::Full.into_style());

                match action {
                    OnModulePress::Action(action) => button.on_press(action).into(),
                    OnModulePress::ToggleMenu(menu_type) => {
                        button.on_press_with_position(move |button_ui_ref| {
                            Message::ToggleMenu(menu_type.clone(), id, button_ui_ref)
                        })
                    }
                }
                .into()
            } else {
                container(content)
                    .padding([2, 8])
                    .height(Length::Fill)
                    .align_y(Alignment::Center)
                    .style(module_label)
                    .into()
            }
        })
    }

    fn group_module_wrapper(&self, group: &Vec<ModuleName>, id: Id) -> Option<Element<Message>> {
        let modules = group
            .iter()
            .map(|module| self.get_module(*module, id))
            .flatten()
            .collect::<Vec<_>>();

        let modules_len = modules.len();

        if modules.is_empty() {
            None
        } else {
            Some(
                Row::with_children(
                    modules
                        .into_iter()
                        .enumerate()
                        .map(|(i, (content, action))| {
                            let group_position = match i {
                                0 => ModuleGroupPosition::First,
                                i if i == 0 && i == modules_len - 1 => ModuleGroupPosition::Only,
                                i if i == modules_len - 1 => ModuleGroupPosition::Last,
                                _ => ModuleGroupPosition::Middle,
                            };

                            if let Some(action) = action {
                                let button = position_button(
                                    container(content)
                                        .align_y(Alignment::Center)
                                        .height(Length::Fill),
                                )
                                .padding([2, 8])
                                .height(Length::Fill)
                                .style(match group_position {
                                    ModuleGroupPosition::First => {
                                        ModuleButtonStyle::First.into_style()
                                    }
                                    ModuleGroupPosition::Middle => {
                                        ModuleButtonStyle::Middle.into_style()
                                    }
                                    ModuleGroupPosition::Last => {
                                        ModuleButtonStyle::Last.into_style()
                                    }
                                    ModuleGroupPosition::Only => {
                                        ModuleButtonStyle::Full.into_style()
                                    }
                                });

                                match action {
                                    OnModulePress::Action(action) => button.on_press(action),
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
                            } else {
                                container(content)
                                    .padding([2, 8])
                                    .height(Length::Fill)
                                    .align_y(Alignment::Center)
                                    .style(match group_position {
                                        ModuleGroupPosition::First => module_first_label,
                                        ModuleGroupPosition::Middle => module_middle_label,
                                        ModuleGroupPosition::Last => module_last_label,
                                        ModuleGroupPosition::Only => module_label,
                                    })
                                    .into()
                            }
                        })
                        .collect::<Vec<_>>(),
                )
                .into(),
            )
        }
    }

    fn get_module(
        &self,
        module_name: ModuleName,
        id: Id,
    ) -> Option<(Element<Message>, Option<OnModulePress>)> {
        match module_name {
            ModuleName::AppLauncher => self.launcher.view(()),
            ModuleName::Updates => self.updates.view(()),
            ModuleName::Clipboard => self.clipboard.view(()),
            ModuleName::Workspaces => self.workspaces.view((
                &self.config.appearance.workspace_colors,
                self.config.appearance.special_workspace_colors.as_deref(),
            )),
            ModuleName::WindowTitle => self.window_title.view(()),
            ModuleName::SystemInfo => self.system_info.view(&self.config.system),
            ModuleName::KeyboardLayout => self.keyboard_layout.view(()),
            ModuleName::KeyboardSubmap => self.keyboard_submap.view(()),
            ModuleName::Tray => self.tray.view(id),
            ModuleName::Clock => self.clock.view(&self.config.clock.format),
            ModuleName::Privacy => self.privacy.as_ref().and_then(|p| p.view(())),
            ModuleName::Settings => self.settings.view(()),
        }
    }
}
