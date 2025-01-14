use crate::{
    centerbox,
    config::{self, Config},
    get_log_spec,
    menu::{menu_wrapper, MenuSize, MenuType},
    modules::{
        self,
        clipboard::Clipboard,
        clock::Clock,
        keyboard_layout::KeyboardLayout,
        keyboard_submap::KeyboardSubmap,
        launcher::Launcher,
        privacy::PrivacyMessage,
        settings::Settings,
        system_info::SystemInfo,
        tray::{TrayMessage, TrayModule},
        updates::Updates,
        window_title::WindowTitle,
        workspaces::Workspaces,
    },
    outputs::{HasOutput, Outputs},
    position_button::ButtonUIRef,
    services::{privacy::PrivacyService, tray::TrayService, ReadOnlyService, ServiceEvent},
    style::ashell_theme,
    utils, HEIGHT,
};
use flexi_logger::LoggerHandle;
use iced::{
    daemon::Appearance,
    event::{listen_with, wayland::Event as WaylandEvent},
    widget::Row,
    window::Id,
    Alignment, Color, Element, Length, Subscription, Task, Theme,
};
use log::{debug, info, warn};

pub struct App {
    logger: LoggerHandle,
    pub config: Config,
    outputs: Outputs,
    pub launcher: Launcher,
    pub updates: Updates,
    pub clipboard: Clipboard,
    pub workspaces: Workspaces,
    pub window_title: WindowTitle,
    pub system_info: SystemInfo,
    pub keyboard_layout: KeyboardLayout,
    pub keyboard_submap: KeyboardSubmap,
    pub tray: TrayModule,
    pub clock: Clock,
    pub privacy: Option<PrivacyService>,
    pub settings: Settings,
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
    ConfigChanged(Box<Config>),
    ToggleMenu(MenuType, Id, ButtonUIRef),
    CloseMenu(Id),
    OpenLauncher,
    OpenClipboard,
    Updates(modules::updates::Message),
    Workspaces(modules::workspaces::Message),
    Title(modules::window_title::Message),
    SystemInfo(modules::system_info::Message),
    KeyboardLayout(modules::keyboard_layout::Message),
    KeyboardSubmap(modules::keyboard_submap::Message),
    Tray(modules::tray::TrayMessage),
    Clock(modules::clock::Message),
    Privacy(modules::privacy::PrivacyMessage),
    Settings(modules::settings::Message),
    WaylandEvent(WaylandEvent),
}

impl App {
    pub fn new((logger, config): (LoggerHandle, Config)) -> impl FnOnce() -> (Self, Task<Message>) {
        || {
            let (outputs, task) = Outputs::new(config.position);
            (
                App {
                    logger,
                    config,
                    outputs,
                    launcher: Launcher,
                    updates: Updates::default(),
                    clipboard: Clipboard,
                    workspaces: Workspaces::default(),
                    window_title: WindowTitle::default(),
                    system_info: SystemInfo::default(),
                    keyboard_layout: KeyboardLayout::default(),
                    keyboard_submap: KeyboardSubmap::default(),
                    tray: TrayModule::default(),
                    clock: Clock::default(),
                    privacy: None,
                    settings: Settings::default(),
                },
                task,
            )
        }
    }

    pub fn title(&self, _id: Id) -> String {
        String::from("ashell")
    }

    pub fn theme(&self, _id: Id) -> Theme {
        ashell_theme(&self.config.appearance)
    }

    pub fn style(&self, theme: &Theme) -> Appearance {
        Appearance {
            background_color: Color::TRANSPARENT,
            text_color: theme.palette().text,
            icon_color: theme.palette().text,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::None => Task::none(),
            Message::ConfigChanged(config) => {
                info!("New config: {:?}", config);
                let mut tasks = Vec::new();
                info!(
                    "Current outputs: {:?}, new outputs: {:?}",
                    self.config.outputs, config.outputs
                );
                if self.config.outputs != config.outputs || self.config.position != config.position
                {
                    warn!("Outputs changed, syncing");
                    tasks.push(self.outputs.sync(&config.outputs, config.position));
                }
                self.config = *config;
                self.logger
                    .set_new_spec(get_log_spec(&self.config.log_level));

                Task::batch(tasks)
            }
            Message::ToggleMenu(menu_type, id, button_ui_ref) => {
                match &menu_type {
                    MenuType::Updates => {
                        self.updates.is_updates_list_open = false;
                    }
                    MenuType::Tray(name) => {
                        if let Some(_tray) = self
                            .tray
                            .service
                            .as_ref()
                            .and_then(|t| t.iter().find(|t| &t.name == name))
                        {
                            self.tray.submenus.clear();
                        }
                    }
                    _ => {}
                };
                self.outputs.toggle_menu(id, menu_type, button_ui_ref)
            }
            Message::CloseMenu(id) => self.outputs.close_menu(id),
            Message::Updates(message) => {
                if let Some(updates_config) = self.config.updates.as_ref() {
                    self.updates
                        .update(message, updates_config, &mut self.outputs)
                } else {
                    Task::none()
                }
            }
            Message::OpenLauncher => {
                if let Some(app_launcher_cmd) = self.config.app_launcher_cmd.as_ref() {
                    utils::launcher::execute_command(app_launcher_cmd.to_string());
                }
                Task::none()
            }
            Message::OpenClipboard => {
                if let Some(clipboard_cmd) = self.config.clipboard_cmd.as_ref() {
                    utils::launcher::execute_command(clipboard_cmd.to_string());
                }
                Task::none()
            }
            Message::Workspaces(msg) => {
                self.workspaces.update(msg);

                Task::none()
            }
            Message::Title(message) => {
                self.window_title
                    .update(message, self.config.truncate_title_after_length);
                Task::none()
            }
            Message::SystemInfo(message) => {
                self.system_info.update(message);
                Task::none()
            }
            Message::KeyboardLayout(message) => {
                self.keyboard_layout.update(message);
                Task::none()
            }
            Message::KeyboardSubmap(message) => {
                self.keyboard_submap.update(message);
                Task::none()
            }
            Message::Tray(msg) => self.tray.update(msg),
            Message::Clock(message) => {
                self.clock.update(message);
                Task::none()
            }
            Message::Privacy(msg) => match msg {
                PrivacyMessage::Event(event) => match event {
                    ServiceEvent::Init(service) => {
                        self.privacy = Some(service);
                        Task::none()
                    }
                    ServiceEvent::Update(data) => {
                        if let Some(privacy) = self.privacy.as_mut() {
                            privacy.update(data);
                        }
                        Task::none()
                    }
                    ServiceEvent::Error(_) => Task::none(),
                },
            },
            Message::Settings(message) => {
                self.settings
                    .update(message, &self.config.settings, &mut self.outputs)
            }
            Message::WaylandEvent(event) => match event {
                WaylandEvent::Output(event, wl_output) => match event {
                    iced::event::wayland::OutputEvent::Created(info) => {
                        info!("Output created: {:?}", info);
                        let name = info
                            .as_ref()
                            .and_then(|info| info.name.as_deref())
                            .unwrap_or("");

                        self.outputs.add(
                            &self.config.outputs,
                            self.config.position,
                            name,
                            wl_output,
                        )
                    }
                    iced::event::wayland::OutputEvent::Removed => {
                        info!("Output destroyed");
                        self.outputs.remove(self.config.position, wl_output)
                    }
                    _ => Task::none(),
                },
                _ => Task::none(),
            },
        }
    }

    pub fn view(&self, id: Id) -> Element<Message> {
        match self.outputs.has(id) {
            Some(HasOutput::Main) => {
                let left = self.modules_section(&self.config.modules.left, id);
                let center = self.modules_section(&self.config.modules.center, id);
                let right = self.modules_section(&self.config.modules.right, id);

                centerbox::Centerbox::new([left, center, right])
                    .spacing(4)
                    .padding([4, 4])
                    .width(Length::Fill)
                    .height(Length::Fixed(HEIGHT as f32))
                    .align_items(Alignment::Center)
                    .into()
            }
            Some(HasOutput::Menu(menu_info)) => match menu_info {
                Some((MenuType::Updates, button_ui_ref)) => menu_wrapper(
                    id,
                    self.updates.menu_view(id).map(Message::Updates),
                    MenuSize::Normal,
                    *button_ui_ref,
                    self.config.position,
                ),
                Some((MenuType::Tray(name), button_ui_ref)) => menu_wrapper(
                    id,
                    self.tray.menu_view(name).map(Message::Tray),
                    MenuSize::Normal,
                    *button_ui_ref,
                    self.config.position,
                ),
                Some((MenuType::Settings, button_ui_ref)) => menu_wrapper(
                    id,
                    self.settings
                        .menu_view(id, &self.config.settings)
                        .map(Message::Settings),
                    MenuSize::Large,
                    *button_ui_ref,
                    self.config.position,
                ),
                None => Row::new().into(),
            },
            None => Row::new().into(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(
            vec![
                self.config.updates.as_ref().map(|updates_config| {
                    self.updates
                        .subscription(updates_config)
                        .map(Message::Updates)
                }),
                Some(self.workspaces.subscription().map(Message::Workspaces)),
                Some(self.window_title.subscription().map(Message::Title)),
                Some(self.system_info.subscription().map(Message::SystemInfo)),
                Some(
                    self.keyboard_layout
                        .subscription()
                        .map(Message::KeyboardLayout),
                ),
                Some(
                    self.keyboard_submap
                        .subscription()
                        .map(Message::KeyboardSubmap),
                ),
                Some(TrayService::subscribe().map(|e| Message::Tray(TrayMessage::Event(e)))),
                Some(self.clock.subscription().map(Message::Clock)),
                Some(
                    PrivacyService::subscribe().map(|e| Message::Privacy(PrivacyMessage::Event(e))),
                ),
                Some(self.settings.subscription().map(Message::Settings)),
                Some(config::subscription()),
                Some(listen_with(|evt, _, _| {
                    if let iced::Event::PlatformSpecific(iced::event::PlatformSpecific::Wayland(
                        evt,
                    )) = evt
                    {
                        if matches!(evt, WaylandEvent::Output(_, _)) {
                            debug!("Wayland event: {:?}", evt);
                            Some(Message::WaylandEvent(evt))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
        )
    }
}
