use crate::{
    HEIGHT, centerbox,
    config::{self, Config},
    get_log_spec,
    menu::{MenuSize, MenuType, menu_wrapper},
    modules::{
        self,
        app_launcher::AppLauncher,
        clipboard::Clipboard,
        clock::Clock,
        keyboard_layout::KeyboardLayout,
        keyboard_submap::KeyboardSubmap,
        media_player::MediaPlayer,
        privacy::Privacy,
        settings::{Settings, brightness::BrightnessMessage},
        system_info::SystemInfo,
        tray::{TrayMessage, TrayModule},
        updates::Updates,
        window_title::WindowTitle,
        workspaces::Workspaces,
    },
    outputs::{HasOutput, Outputs},
    position_button::ButtonUIRef,
    services::{Service, ServiceEvent, brightness::BrightnessCommand, tray::TrayEvent},
    style::ashell_theme,
    utils,
};
use flexi_logger::LoggerHandle;
use iced::{
    Alignment, Color, Element, Length, Subscription, Task, Theme,
    daemon::Appearance,
    event::{
        listen_with,
        wayland::{Event as WaylandEvent, OutputEvent},
    },
    widget::{Row, container},
    window::Id,
};
use log::{debug, info, warn};
use wayland_client::protocol::wl_output::WlOutput;

pub struct App {
    logger: LoggerHandle,
    pub config: Config,
    pub outputs: Outputs,
    pub app_launcher: AppLauncher,
    pub updates: Updates,
    pub clipboard: Clipboard,
    pub workspaces: Workspaces,
    pub window_title: WindowTitle,
    pub system_info: SystemInfo,
    pub keyboard_layout: KeyboardLayout,
    pub keyboard_submap: KeyboardSubmap,
    pub tray: TrayModule,
    pub clock: Clock,
    pub privacy: Privacy,
    pub settings: Settings,
    pub media_player: MediaPlayer,
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
    WindowTitle(modules::window_title::Message),
    SystemInfo(modules::system_info::Message),
    KeyboardLayout(modules::keyboard_layout::Message),
    KeyboardSubmap(modules::keyboard_submap::Message),
    Tray(modules::tray::TrayMessage),
    Clock(modules::clock::Message),
    Privacy(modules::privacy::PrivacyMessage),
    Settings(modules::settings::Message),
    MediaPlayer(modules::media_player::Message),
    OutputEvent((OutputEvent, WlOutput)),
}

impl App {
    pub fn new((logger, config): (LoggerHandle, Config)) -> impl FnOnce() -> (Self, Task<Message>) {
        || {
            let (outputs, task) = Outputs::new(config.appearance.solid_style, config.position);
            let enable_workspace_filling = config.workspaces.enable_workspace_filling;
            (
                App {
                    logger,
                    config,
                    outputs,
                    app_launcher: AppLauncher,
                    updates: Updates::default(),
                    clipboard: Clipboard,
                    workspaces: Workspaces::new(enable_workspace_filling),
                    window_title: WindowTitle::default(),
                    system_info: SystemInfo::default(),
                    keyboard_layout: KeyboardLayout::default(),
                    keyboard_submap: KeyboardSubmap::default(),
                    tray: TrayModule::default(),
                    clock: Clock::default(),
                    privacy: Privacy::default(),
                    settings: Settings::default(),
                    media_player: MediaPlayer::default(),
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
                if self.config.outputs != config.outputs
                    || self.config.position != config.position
                    || self.config.appearance.solid_style != config.appearance.solid_style
                {
                    warn!("Outputs changed, syncing");
                    tasks.push(self.outputs.sync(
                        config.appearance.solid_style,
                        &config.outputs,
                        config.position,
                    ));
                }
                self.config = *config;
                self.logger
                    .set_new_spec(get_log_spec(&self.config.log_level));

                Task::batch(tasks)
            }
            Message::ToggleMenu(menu_type, id, button_ui_ref) => {
                let mut cmd = vec![];
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
                    MenuType::Settings => {
                        self.settings.sub_menu = None;

                        if let Some(brightness) = self.settings.brightness.as_mut() {
                            cmd.push(brightness.command(BrightnessCommand::Refresh).map(|event| {
                                crate::app::Message::Settings(
                                    crate::modules::settings::Message::Brightness(
                                        BrightnessMessage::Event(event),
                                    ),
                                )
                            }));
                        }
                    }
                    _ => {}
                };
                cmd.push(self.outputs.toggle_menu(id, menu_type, button_ui_ref));

                Task::batch(cmd)
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
            Message::WindowTitle(message) => {
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
            Message::Tray(msg) => {
                let close_tray = match &msg {
                    TrayMessage::Event(ServiceEvent::Update(TrayEvent::Unregistered(name))) => {
                        self.outputs.close_all_menu_if(MenuType::Tray(name.clone()))
                    }
                    _ => Task::none(),
                };

                Task::batch(vec![self.tray.update(msg), close_tray])
            }
            Message::Clock(message) => {
                self.clock.update(message);
                Task::none()
            }
            Message::Privacy(msg) => self.privacy.update(msg),
            Message::Settings(message) => {
                self.settings
                    .update(message, &self.config.settings, &mut self.outputs)
            }
            Message::OutputEvent((event, wl_output)) => match event {
                iced::event::wayland::OutputEvent::Created(info) => {
                    info!("Output created: {:?}", info);
                    let name = info
                        .as_ref()
                        .and_then(|info| info.name.as_deref())
                        .unwrap_or("");

                    self.outputs.add(
                        self.config.appearance.solid_style,
                        &self.config.outputs,
                        self.config.position,
                        name,
                        wl_output,
                    )
                }
                iced::event::wayland::OutputEvent::Removed => {
                    info!("Output destroyed");
                    self.outputs.remove(
                        self.config.appearance.solid_style,
                        self.config.position,
                        wl_output,
                    )
                }
                _ => Task::none(),
            },
            Message::MediaPlayer(msg) => self.media_player.update(msg),
        }
    }

    pub fn view(&self, id: Id) -> Element<Message> {
        match self.outputs.has(id) {
            Some(HasOutput::Main) => {
                let left = self.modules_section(&self.config.modules.left, id);
                let center = self.modules_section(&self.config.modules.center, id);
                let right = self.modules_section(&self.config.modules.right, id);

                let centerbox = centerbox::Centerbox::new([left, center, right])
                    .spacing(4)
                    .width(Length::Fill)
                    .align_items(Alignment::Center);

                if self.config.appearance.solid_style {
                    container(
                        centerbox
                            .padding([0, 4])
                            .height(Length::Fixed((HEIGHT - 8) as f32)),
                    )
                    .style(|t| container::Style {
                        background: Some(t.palette().background.into()),

                        ..Default::default()
                    })
                    .into()
                } else {
                    centerbox
                        .height(Length::Fixed(HEIGHT as f32))
                        .padding([4, 4])
                        .into()
                }
            }
            Some(HasOutput::Menu(menu_info)) => match menu_info {
                Some((MenuType::Updates, button_ui_ref)) => menu_wrapper(
                    id,
                    self.updates.menu_view(id).map(Message::Updates),
                    MenuSize::Normal,
                    *button_ui_ref,
                    self.config.position,
                    self.config.appearance.solid_style,
                ),
                Some((MenuType::Tray(name), button_ui_ref)) => menu_wrapper(
                    id,
                    self.tray.menu_view(name).map(Message::Tray),
                    MenuSize::Normal,
                    *button_ui_ref,
                    self.config.position,
                    self.config.appearance.solid_style,
                ),
                Some((MenuType::Settings, button_ui_ref)) => menu_wrapper(
                    id,
                    self.settings
                        .menu_view(id, &self.config.settings)
                        .map(Message::Settings),
                    MenuSize::Large,
                    *button_ui_ref,
                    self.config.position,
                    self.config.appearance.solid_style,
                ),
                Some((MenuType::MediaPlayer, button_ui_ref)) => menu_wrapper(
                    id,
                    self.media_player
                        .menu_view(&self.config.media_player)
                        .map(Message::MediaPlayer),
                    MenuSize::Large,
                    *button_ui_ref,
                    self.config.position,
                    self.config.appearance.solid_style,
                ),
                None => Row::new().into(),
            },
            None => Row::new().into(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            Subscription::batch(self.modules_subscriptions(&self.config.modules.left)),
            Subscription::batch(self.modules_subscriptions(&self.config.modules.center)),
            Subscription::batch(self.modules_subscriptions(&self.config.modules.right)),
            config::subscription(),
            listen_with(|evt, _, _| match evt {
                iced::Event::PlatformSpecific(iced::event::PlatformSpecific::Wayland(
                    WaylandEvent::Output(event, wl_output),
                )) => {
                    debug!("Wayland event: {:?}", event);
                    Some(Message::OutputEvent((event, wl_output)))
                }
                _ => None,
            }),
        ])
    }
}
