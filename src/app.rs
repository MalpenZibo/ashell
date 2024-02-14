use crate::{
    centerbox,
    menu::{close_menu, menu_wrapper},
    modules::{
        clock::Clock, launcher, settings::Settings, system_info::SystemInfo, title::Title,
        updates::Updates, workspaces::Workspaces,
    },
    password_dialog,
    style::ashell_theme,
    HEIGHT,
};
use iced::{
    wayland::{
        actions::layer_surface::SctkLayerSurfaceSettings,
        layer_surface::{Anchor, Layer},
    },
    widget::{row, Column},
    window::Id,
    Alignment, Application, Color, Command, Length, Theme,
};

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum OpenMenu {
    Updates,
    Settings,
}

pub struct App {
    overlay_id: Id,
    menu_type: Option<OpenMenu>,
    updates: Updates,
    workspaces: Workspaces,
    window_title: Title,
    system_info: SystemInfo,
    clock: Clock,
    pub settings: Settings,
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
    CloseMenu,
    Launcher(crate::modules::launcher::Message),
    Updates(crate::modules::updates::Message),
    Workspaces(crate::modules::workspaces::Message),
    Title(crate::modules::title::Message),
    SystemInfo(crate::modules::system_info::Message),
    Clock(crate::modules::clock::Message),
    Settings(crate::modules::settings::Message),
}

impl Application for App {
    type Executor = iced::executor::Default;
    type Theme = Theme;
    type Message = Message;
    type Flags = ();

    fn new(_: ()) -> (Self, iced::Command<Self::Message>) {
        let (overlay_id, cmd) = open_overlay_surface();
        (
            App {
                overlay_id,
                menu_type: None,
                updates: Updates::new(),
                workspaces: Workspaces::new(),
                window_title: Title::new(),
                system_info: SystemInfo::new(),
                clock: Clock::new(),
                settings: Settings::new(),
            },
            cmd,
        )
    }

    fn theme(&self, _id: Id) -> Self::Theme {
        ashell_theme()
    }

    fn style(&self) -> iced::theme::Application {
        fn dark_background(theme: &Theme) -> iced::wayland::Appearance {
            iced::wayland::Appearance {
                background_color: Color::TRANSPARENT,
                text_color: theme.palette().text,
                icon_color: theme.palette().text,
            }
        }

        iced::theme::Application::from(dark_background as fn(&Theme) -> _)
    }

    fn title(&self, _id: Id) -> String {
        String::from("ashell")
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            Message::None => iced::Command::none(),
            Message::CloseMenu => {
                let old_menu = self.menu_type.take();

                if old_menu.is_some() {
                    close_menu(self.overlay_id)
                } else {
                    iced::Command::none()
                }
            }
            Message::Updates(message) => self
                .updates
                .update(message, self.overlay_id, &mut self.menu_type)
                .map(Message::Updates),
            Message::Launcher(_) => {
                crate::utils::launcher::launch_rofi();
                iced::Command::none()
            }
            Message::Workspaces(msg) => {
                self.workspaces.update(msg);

                iced::Command::none()
            }
            Message::Title(message) => {
                self.window_title.update(message);
                iced::Command::none()
            }
            Message::SystemInfo(message) => {
                self.system_info.update(message);
                iced::Command::none()
            }
            Message::Clock(message) => {
                self.clock.update(message);
                iced::Command::none()
            }
            Message::Settings(message) => self
                .settings
                .update(message, self.overlay_id, &mut self.menu_type)
                .map(Message::Settings),
        }
    }

    fn view(&self, id: Id) -> iced::Element<'_, Self::Message> {
        match id {
            Id::MAIN => {
                let left = row!(
                    launcher::launcher().map(Message::Launcher),
                    self.updates.view().map(Message::Updates),
                    self.workspaces.view().map(Message::Workspaces)
                )
                .height(Length::Shrink)
                .align_items(Alignment::Center)
                .spacing(4);

                let mut center = row!().spacing(4);
                if let Some(title) = self.window_title.view() {
                    center = center.push(title.map(Message::Title));
                }

                let right = row!(
                    self.system_info.view().map(Message::SystemInfo),
                    row!(
                        self.clock.view().map(Message::Clock),
                        self.settings.view().map(Message::Settings)
                    )
                )
                .spacing(4);

                centerbox::Centerbox::new([left.into(), center.into(), right.into()])
                    .spacing(4)
                    .padding([0, 4])
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_items(Alignment::Center)
                    .into()
            }
            _ => {
                if id == self.overlay_id {
                    if let Some((ssid, password)) = self.settings.password_dialog.as_ref() {
                        return password_dialog::view(ssid, password).map(|msg| {
                            Message::Settings(crate::modules::settings::Message::PasswordDialog(
                                msg,
                            ))
                        });
                    }
                    if let Some(menu_type) = self.menu_type {
                        return menu_wrapper(
                            match menu_type {
                                OpenMenu::Updates => self.updates.menu_view().map(Message::Updates),
                                OpenMenu::Settings => {
                                    self.settings.menu_view().map(Message::Settings)
                                }
                            },
                            match menu_type {
                                OpenMenu::Updates => crate::menu::MenuPosition::Left,
                                OpenMenu::Settings => crate::menu::MenuPosition::Right,
                            },
                        );
                    }
                }

                Column::new().into()
            }
        }
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        iced::Subscription::batch(vec![
            self.updates.subscription().map(Message::Updates),
            self.workspaces.subscription().map(Message::Workspaces),
            self.window_title.subscription().map(Message::Title),
            self.system_info.subscription().map(Message::SystemInfo),
            self.clock.subscription().map(Message::Clock),
            self.settings.subscription().map(Message::Settings),
        ])
    }
}

fn open_overlay_surface<Msg>() -> (Id, Command<Msg>) {
    let id = Id::unique();
    (
        id,
        iced::wayland::layer_surface::get_layer_surface(SctkLayerSurfaceSettings {
            id,
            layer: Layer::Overlay,
            size: Some((None, Some(HEIGHT))),
            anchor: Anchor::TOP.union(Anchor::LEFT).union(Anchor::RIGHT),
            ..Default::default()
        }),
    )
}
