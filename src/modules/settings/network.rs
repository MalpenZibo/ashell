use super::{SubMenu, quick_setting_button};
use crate::{
    components::icons::{StaticIcon, icon, icon_button},
    config::SettingsFormat,
    services::{
        ReadOnlyService, Service, ServiceEvent,
        network::{
            AccessPoint, ActiveConnectionInfo, KnownConnection, NetworkCommand, NetworkEvent,
            NetworkService, Vpn, dbus::ConnectivityState,
        },
    },
    theme::AshellTheme,
    utils::IndicatorState,
};
use iced::{
    Alignment, Element, Length, Subscription, Task, Theme,
    widget::{Column, button, column, container, horizontal_rule, row, scrollable, text, toggler},
    window::Id,
};
use log::info;

static WIFI_SIGNAL_ICONS: [StaticIcon; 6] = [
    StaticIcon::Wifi0,
    StaticIcon::Wifi1,
    StaticIcon::Wifi2,
    StaticIcon::Wifi3,
    StaticIcon::Wifi4,
    StaticIcon::Wifi5,
];

static WIFI_LOCK_SIGNAL_ICONS: [StaticIcon; 5] = [
    StaticIcon::WifiLock1,
    StaticIcon::WifiLock2,
    StaticIcon::WifiLock3,
    StaticIcon::WifiLock4,
    StaticIcon::WifiLock5,
];

impl ActiveConnectionInfo {
    pub fn get_wifi_icon(signal: u8) -> StaticIcon {
        WIFI_SIGNAL_ICONS[1 + f32::round(signal as f32 / 100. * 4.) as usize]
    }

    pub fn get_wifi_lock_icon(signal: u8) -> StaticIcon {
        WIFI_LOCK_SIGNAL_ICONS[f32::round(signal as f32 / 100. * 4.) as usize]
    }

    pub fn get_icon(&self) -> StaticIcon {
        match self {
            Self::WiFi { strength, .. } => Self::get_wifi_icon(*strength),
            Self::Wired { .. } => StaticIcon::Ethernet,
            Self::Vpn { .. } => StaticIcon::Vpn,
        }
    }

    pub fn get_indicator_state(&self) -> IndicatorState {
        match self {
            Self::WiFi {
                strength: 0 | 1, ..
            } => IndicatorState::Warning,
            _ => IndicatorState::Normal,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Event(ServiceEvent<NetworkService>),
    ToggleWiFi,
    ScanNearByWiFi,
    WiFiMore(Id),
    VpnMore(Id),
    SelectAccessPoint(AccessPoint),
    RequestWiFiPassword(Id, String),
    ToggleVpn(Vpn),
    ToggleAirplaneMode,
    ToggleWifiMenu,
    ToggleVPNMenu,
    WifiMenuOpened,
    PasswordDialogConfirmed(String, String),
    ConfigReloaded(NetworkSettingsConfig),
}

pub enum Action {
    None,
    RequestPasswordForSSID(String),
    RequestPassword(Id, String),
    Command(Task<Message>),
    ToggleWifiMenu,
    ToggleVpnMenu,
    CloseSubMenu(Task<Message>),
    CloseMenu(Id),
}

#[derive(Debug, Clone)]
pub struct NetworkSettingsConfig {
    pub wifi_more_cmd: Option<String>,
    pub vpn_more_cmd: Option<String>,
    pub remove_airplane_btn: bool,
    pub indicator_format: SettingsFormat,
}

impl NetworkSettingsConfig {
    pub fn new(
        wifi_more_cmd: Option<String>,
        vpn_more_cmd: Option<String>,
        remove_airplane_btn: bool,
        indicator_format: SettingsFormat,
    ) -> Self {
        Self {
            wifi_more_cmd,
            vpn_more_cmd,
            remove_airplane_btn,
            indicator_format,
        }
    }
}

pub struct NetworkSettings {
    config: NetworkSettingsConfig,
    service: Option<NetworkService>,
}

impl NetworkSettings {
    pub fn new(config: NetworkSettingsConfig) -> Self {
        Self {
            config,
            service: None,
        }
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::Event(event) => match event {
                ServiceEvent::Init(service) => {
                    self.service = Some(service);
                    Action::None
                }
                ServiceEvent::Update(NetworkEvent::RequestPasswordForSSID(ssid)) => {
                    Action::RequestPasswordForSSID(ssid)
                }
                ServiceEvent::Update(data) => {
                    if let Some(service) = self.service.as_mut() {
                        service.update(data);
                    }
                    Action::None
                }
                _ => Action::None,
            },
            Message::ToggleAirplaneMode => match self.service.as_mut() {
                Some(service) => Action::CloseSubMenu(
                    service
                        .command(NetworkCommand::ToggleAirplaneMode)
                        .map(Message::Event),
                ),
                _ => Action::None,
            },
            Message::ToggleWiFi => match self.service.as_mut() {
                Some(service) => Action::CloseSubMenu(
                    service
                        .command(NetworkCommand::ToggleWiFi)
                        .map(Message::Event),
                ),
                _ => Action::None,
            },
            Message::SelectAccessPoint(ac) => match self.service.as_mut() {
                Some(service) => Action::Command(
                    service
                        .command(NetworkCommand::SelectAccessPoint((ac, None)))
                        .map(Message::Event),
                ),
                _ => Action::None,
            },
            Message::RequestWiFiPassword(id, ssid) => {
                info!("Requesting password for {ssid}");
                Action::RequestPassword(id, ssid)
            }
            Message::ScanNearByWiFi => match self.service.as_mut() {
                Some(service) => Action::Command(
                    service
                        .command(NetworkCommand::ScanNearByWiFi)
                        .map(Message::Event),
                ),
                _ => Action::None,
            },
            Message::WiFiMore(id) => {
                if let Some(cmd) = &self.config.wifi_more_cmd {
                    crate::utils::launcher::execute_command(cmd.to_string());
                    Action::CloseMenu(id)
                } else {
                    Action::None
                }
            }
            Message::VpnMore(id) => {
                if let Some(cmd) = &self.config.vpn_more_cmd {
                    crate::utils::launcher::execute_command(cmd.to_string());
                    Action::CloseMenu(id)
                } else {
                    Action::None
                }
            }
            Message::ToggleVpn(vpn) => match self.service.as_mut() {
                Some(service) => Action::Command(
                    service
                        .command(NetworkCommand::ToggleVpn(vpn))
                        .map(Message::Event),
                ),
                _ => Action::None,
            },
            Message::ToggleWifiMenu => Action::ToggleWifiMenu,
            Message::ToggleVPNMenu => Action::ToggleVpnMenu,
            Message::WifiMenuOpened => {
                if let Some(service) = self.service.as_mut() {
                    Action::Command(
                        service
                            .command(NetworkCommand::ScanNearByWiFi)
                            .map(Message::Event),
                    )
                } else {
                    Action::None
                }
            }
            Message::PasswordDialogConfirmed(ssid, password) => match self.service.as_mut() {
                Some(service) => {
                    let ap = service
                        .wireless_access_points
                        .iter()
                        .find(|ap| ap.ssid == ssid)
                        .cloned();
                    if let Some(ap) = ap {
                        Action::Command(
                            service
                                .command(NetworkCommand::SelectAccessPoint((ap, Some(password))))
                                .map(Message::Event),
                        )
                    } else {
                        Action::None
                    }
                }
                _ => Action::None,
            },
            Message::ConfigReloaded(config) => {
                self.config = config;
                Action::None
            }
        }
    }

    pub fn connection_indicator<'a>(&'a self, _: &'a AshellTheme) -> Option<Element<'a, Message>> {
        self.service.as_ref().and_then(|service| {
            if service.airplane_mode || !service.wifi_present {
                None
            } else {
                Some(
                    service
                        .active_connections
                        .iter()
                        .find(|c| {
                            matches!(c, ActiveConnectionInfo::WiFi { .. })
                                || matches!(c, ActiveConnectionInfo::Wired { .. })
                        })
                        .map_or_else(
                            || match self.config.indicator_format {
                                SettingsFormat::Icon => icon(StaticIcon::Wifi0).into(),
                                SettingsFormat::Percentage => text("0%").into(),
                                SettingsFormat::IconAndPercentage => {
                                    row!(icon(StaticIcon::Wifi0), text("0%"))
                                        .spacing(4)
                                        .align_y(Alignment::Center)
                                        .into()
                                }
                            },
                            |a| {
                                let icon_type = a.get_icon();
                                let state = (service.connectivity, a.get_indicator_state());
                                let strength = match a {
                                    ActiveConnectionInfo::WiFi { strength, .. } => Some(*strength),
                                    _ => None,
                                };

                                let style_fn = move |theme: &Theme| container::Style {
                                    text_color: match state {
                                        (ConnectivityState::Full, IndicatorState::Warning) => {
                                            Some(theme.extended_palette().danger.weak.color)
                                        }
                                        (ConnectivityState::Full, _) => None,
                                        _ => Some(theme.palette().danger),
                                    },
                                    ..Default::default()
                                };

                                match self.config.indicator_format {
                                    SettingsFormat::Icon => {
                                        container(icon(icon_type)).style(style_fn).into()
                                    }
                                    SettingsFormat::Percentage => {
                                        let strength_text = strength
                                            .map_or("100%".to_string(), |s| format!("{}%", s));
                                        container(text(strength_text)).style(style_fn).into()
                                    }
                                    SettingsFormat::IconAndPercentage => {
                                        let strength_text = strength
                                            .map_or("100%".to_string(), |s| format!("{}%", s));
                                        container(
                                            row!(icon(icon_type), text(strength_text))
                                                .spacing(4)
                                                .align_y(Alignment::Center),
                                        )
                                        .style(style_fn)
                                        .into()
                                    }
                                }
                            },
                        ),
                )
            }
        })
    }

    pub fn vpn_indicator<'a>(&'a self, _: &AshellTheme) -> Option<Element<'a, Message>> {
        self.service.as_ref().and_then(|service| {
            service
                .active_connections
                .iter()
                .find(|c| matches!(c, ActiveConnectionInfo::Vpn { .. }))
                .map(|a| {
                    let icon_type = a.get_icon();

                    container(icon(icon_type))
                        .style(|theme: &Theme| container::Style {
                            text_color: Some(theme.extended_palette().danger.weak.color),
                            ..Default::default()
                        })
                        .into()
                })
        })
    }

    pub fn wifi_quick_setting_button<'a>(
        &'a self,
        id: Id,
        theme: &'a AshellTheme,
        sub_menu: Option<SubMenu>,
    ) -> Option<(Element<'a, Message>, Option<Element<'a, Message>>)> {
        self.service.as_ref().and_then(|service| {
            if service.wifi_present {
                let active_connection = service.active_connections.iter().find_map(|c| match c {
                    ActiveConnectionInfo::WiFi { name, strength, .. } => {
                        Some((name, strength, c.get_icon()))
                    }
                    _ => None,
                });

                Some((
                    quick_setting_button(
                        theme,
                        active_connection.map_or_else(|| StaticIcon::Wifi0, |(_, _, icon)| icon),
                        "Wi-Fi".to_string(),
                        active_connection.map(|(name, _, _)| name.clone()),
                        service.wifi_enabled,
                        Message::ToggleWiFi,
                        Some((SubMenu::Wifi, sub_menu, Message::ToggleWifiMenu))
                            .filter(|_| service.wifi_enabled),
                    ),
                    sub_menu
                        .filter(|menu_type| *menu_type == SubMenu::Wifi)
                        .map(|_| {
                            Self::wifi_menu(
                                service,
                                id,
                                theme,
                                active_connection
                                    .map(|(name, strengh, _)| (name.as_str(), *strengh)),
                                self.config.wifi_more_cmd.is_some(),
                            )
                        }),
                ))
            } else {
                None
            }
        })
    }

    pub fn vpn_quick_setting_button<'a>(
        &'a self,
        id: Id,
        theme: &'a AshellTheme,
        sub_menu: Option<SubMenu>,
    ) -> Option<(Element<'a, Message>, Option<Element<'a, Message>>)> {
        self.service.as_ref().and_then(|service| {
            service
                .known_connections
                .iter()
                .any(|c| matches!(c, KnownConnection::Vpn { .. }))
                .then(|| {
                    let mut known_vpn = service.known_connections.iter().filter_map(|c| match c {
                        KnownConnection::Vpn(c) => Some(c),
                        _ => None,
                    });
                    let actives = service
                        .active_connections
                        .iter()
                        .filter_map(|c| match c {
                            ActiveConnectionInfo::Vpn { name, .. } => {
                                known_vpn.find(|v| v.name == *name)
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>();

                    let subtitle = if actives.len() > 1 {
                        Some(format!("{} VPNs Connected", actives.len()))
                    } else {
                        actives.first().map(|c| c.name.clone())
                    };

                    (
                        quick_setting_button(
                            theme,
                            StaticIcon::Vpn,
                            "VPN".to_string(),
                            subtitle,
                            !actives.is_empty(),
                            if !actives.is_empty()
                                && let Some(first) = actives.first()
                            {
                                Message::ToggleVpn((*first).clone())
                            } else {
                                Message::ToggleVPNMenu
                            },
                            if !actives.is_empty() {
                                Some((SubMenu::Vpn, sub_menu, Message::ToggleVPNMenu))
                            } else {
                                None
                            },
                        ),
                        sub_menu
                            .filter(|menu_type| *menu_type == SubMenu::Vpn)
                            .map(|_| {
                                Self::vpn_menu(
                                    service,
                                    id,
                                    theme,
                                    self.config.vpn_more_cmd.is_some(),
                                )
                            }),
                    )
                })
        })
    }

    pub fn airplane_mode_quick_setting_button<'a>(
        &'a self,
        theme: &'a AshellTheme,
    ) -> Option<(Element<'a, Message>, Option<Element<'a, Message>>)> {
        if self.config.remove_airplane_btn {
            None
        } else {
            self.service.as_ref().map(|service| {
                (
                    quick_setting_button(
                        theme,
                        StaticIcon::Airplane,
                        "Airplane Mode".to_string(),
                        None,
                        service.airplane_mode,
                        Message::ToggleAirplaneMode,
                        None,
                    ),
                    None,
                )
            })
        }
    }

    fn wifi_menu<'a>(
        service: &'a NetworkService,
        id: Id,
        theme: &'a AshellTheme,
        active_connection: Option<(&str, u8)>,
        show_more_button: bool,
    ) -> Element<'a, Message> {
        let main = column!(
            row!(
                text("Nearby Wifi").width(Length::Fill),
                text(if service.scanning_nearby_wifi {
                    "Scanning..."
                } else {
                    ""
                })
                .size(theme.font_size.sm),
                icon_button(theme, StaticIcon::Refresh).on_press(Message::ScanNearByWiFi)
            )
            .spacing(theme.space.xs)
            .width(Length::Fill)
            .align_y(Alignment::Center),
            horizontal_rule(1),
            container(scrollable(
                Column::with_children(
                    service.wireless_access_points
                    .iter()
                    .filter_map(|ac| if active_connection.is_some_and(|(ssid, _)| ssid == ac.ssid) {Some((ac, true))} else {None })
                    .chain(service.wireless_access_points
                        .iter()
                        .filter_map(|ac| if active_connection.is_some_and(|(ssid, _)| ssid == ac.ssid) {None} else {Some((ac, false))})
                    )
                        .map(|(ac, is_active)| {
                            let is_known = service.known_connections.iter().any(|c| {
                                matches!(
                                    c,
                                    KnownConnection::AccessPoint(AccessPoint { ssid, .. }) if ssid == &ac.ssid
                                )
                            });

                            button(
                                container(
                                    row!(
                                        icon(if ac.public {
                                            ActiveConnectionInfo::get_wifi_icon(ac.strength)
                                        } else {
                                            ActiveConnectionInfo::get_wifi_lock_icon(ac.strength)
                                        })
                                        .width(Length::Shrink),
                                        text(ac.ssid.clone()).width(Length::Fill),
                                    )
                                    .align_y(Alignment::Center)
                                    .spacing(8),
                                )
                                .style(move |theme: &Theme| {
                                    container::Style {
                                        text_color: if is_active {
                                            Some(theme.palette().success)
                                        } else {
                                            None
                                        },
                                        ..Default::default()
                                    }
                                }),
                            )
                            .style(theme.ghost_button_style())
                            .padding([8, 8])
                            .on_press_maybe(if !is_active {
                                Some(if is_known {
                                    Message::SelectAccessPoint(ac.clone())
                                } else {
                                    Message::RequestWiFiPassword(id, ac.ssid.clone())
                                })
                            } else {
                                None
                            })
                            .width(Length::Fill)
                            .into()
                        })
                        .collect::<Vec<Element<'a, Message>>>(),
                )
                .spacing(theme.space.xxs)
            ))
            .max_height(200),
        )
        .spacing(theme.space.xs);

        if show_more_button {
            column!(
                main,
                horizontal_rule(1),
                button("More")
                    .on_press(Message::WiFiMore(id))
                    .padding([theme.space.xxs, theme.space.sm])
                    .width(Length::Fill)
                    .style(theme.ghost_button_style())
            )
            .spacing(theme.space.sm)
            .into()
        } else {
            main.into()
        }
    }

    fn vpn_menu<'a>(
        service: &'a NetworkService,
        id: Id,
        theme: &'a AshellTheme,
        show_more_button: bool,
    ) -> Element<'a, Message> {
        let main = Column::with_children(
            service.known_connections
                .iter()
                .filter_map(|c| match c {
                    KnownConnection::Vpn(vpn) => Some(vpn),
                    _ => None,
                })
                .map(|vpn| {
                    let is_active = service.active_connections.iter().any(
                        |c| matches!(c, ActiveConnectionInfo::Vpn { name, .. } if name == &vpn.name),
                    );

                    row!(
                        text(vpn.name.to_string()).width(Length::Fill),
                        toggler(is_active)
                            .on_toggle(|_| { Message::ToggleVpn(vpn.clone()) })
                            .width(Length::Shrink),
                    )
                    .into()
                })
                .collect::<Vec<Element<'a, Message>>>(),
        )
        .spacing(theme.space.xs);

        if show_more_button {
            column!(
                main,
                horizontal_rule(1),
                button("More")
                    .on_press(Message::VpnMore(id))
                    .padding([theme.space.xxs, theme.space.sm])
                    .width(Length::Fill)
                    .style(theme.ghost_button_style())
            )
            .spacing(theme.space.sm)
            .into()
        } else {
            main.into()
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        NetworkService::subscribe().map(Message::Event)
    }
}
