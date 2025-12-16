use super::{SubMenu, quick_setting_button};
use crate::{
    components::icons::{StaticIcon, icon, icon_button},
    services::{
        ReadOnlyService, Service, ServiceEvent,
        network::{
            AccessPoint, ActiveConnectionInfo, KnownConnection, NetworkCommand, NetworkEvent,
            NetworkService, Vpn, dbus::ConnectivityState, ExitNode,
        },
    },
    theme::AshellTheme,
    utils::IndicatorState,
};
use iced::{
    Alignment, Border, Element, Length, Subscription, Task, Theme,
    widget::{Column, button, column, container, horizontal_rule, horizontal_space, row, scrollable, text, toggler},
    window::Id,
};
use log::info;
use std::collections::HashMap;

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

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum VpnTab {
    System,
    Tailscale,
}

impl Default for VpnTab {
    fn default() -> Self {
        VpnTab::System
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Event(ServiceEvent<NetworkService>),
    ToggleWiFi,
    ScanNearByWiFi,
    WiFiMore(Id),
    VpnMore(Id),
    TailscaleMore(Id),
    SelectAccessPoint(AccessPoint),
    RequestWiFiPassword(Id, String),
    ToggleVpn(Vpn),
    ToggleAirplaneMode,
    ToggleWifiMenu,
    ToggleVPNMenu,
    WifiMenuOpened,
    PasswordDialogConfirmed(String, String),
    ConfigReloaded(NetworkSettingsConfig),
    // VPN tab
    SwitchVpnTab(VpnTab),
    // Tailscale messages
    TailscaleConnect,
    TailscaleDisconnect,
    TailscaleSwitchProfile(String),
    TailscaleSetExitNode(Option<String>),
    TailscaleSetAllowLan(bool),
    TailscaleToggleCountry(String),
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
}

impl NetworkSettingsConfig {
    pub fn new(
        wifi_more_cmd: Option<String>,
        vpn_more_cmd: Option<String>,
        remove_airplane_btn: bool,
    ) -> Self {
        Self {
            wifi_more_cmd,
            vpn_more_cmd,
            remove_airplane_btn,
        }
    }
}

pub struct NetworkSettings {
    config: NetworkSettingsConfig,
    service: Option<NetworkService>,
    /// Tracks which countries are expanded in exit node list
    expanded_countries: Vec<String>,
    /// Currently selected VPN tab
    vpn_tab: VpnTab,
    /// Last active VPN (if any)
    last_active_vpn: Option<LastActiveVpn>,
}

#[derive(Debug, Clone, PartialEq)]
enum LastActiveVpn {
    System(Vpn),
    Tailscale,
}

impl NetworkSettings {
    pub fn new(config: NetworkSettingsConfig) -> Self {
        Self {
            config,
            service: None,
            expanded_countries: Vec::new(),
            vpn_tab: VpnTab::default(),
            last_active_vpn: None,
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
                        
                        // Update last active VPN
                        if service.tailscale.is_running {
                            self.last_active_vpn = Some(LastActiveVpn::Tailscale);
                        } else {
                            // Check for NM VPNs
                            let active_nm_vpn = service.active_connections.iter().find_map(|c| match c {
                                ActiveConnectionInfo::Vpn { name, .. } => {
                                    service.known_connections.iter().find_map(|k| match k {
                                        KnownConnection::Vpn(v) if v.name == *name => Some(v.clone()),
                                        _ => None
                                    })
                                }
                                _ => None
                            });
                            
                            if let Some(vpn) = active_nm_vpn {
                                self.last_active_vpn = Some(LastActiveVpn::System(vpn));
                            }
                        }
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
            Message::TailscaleMore(id) => {
                crate::utils::launcher::execute_command("xdg-open http://100.100.100.100/".to_string());
                Action::CloseMenu(id)
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
            // Tailscale handlers
            Message::TailscaleConnect => match self.service.as_mut() {
                Some(service) => Action::Command(
                    service
                        .command(NetworkCommand::TailscaleConnect)
                        .map(Message::Event),
                ),
                _ => Action::None,
            },
            Message::TailscaleDisconnect => match self.service.as_mut() {
                Some(service) => Action::Command(
                    service
                        .command(NetworkCommand::TailscaleDisconnect)
                        .map(Message::Event),
                ),
                _ => Action::None,
            },
            Message::TailscaleSwitchProfile(display_name) => match self.service.as_mut() {
                Some(service) => {
                    // Find profile ID by display name
                    let profile_id = service.tailscale.profiles.iter()
                        .find(|p| p.display_name() == display_name)
                        .map(|p| p.id.clone());
                    
                    if let Some(id) = profile_id {
                        Action::Command(
                            service
                                .command(NetworkCommand::TailscaleSwitchProfile(id))
                                .map(Message::Event),
                        )
                    } else {
                        Action::None
                    }
                }
                _ => Action::None,
            },
            Message::TailscaleSetExitNode(node_id) => match self.service.as_mut() {
                Some(service) => Action::Command(
                    service
                        .command(NetworkCommand::TailscaleSetExitNode(node_id))
                        .map(Message::Event),
                ),
                _ => Action::None,
            },
            Message::TailscaleSetAllowLan(allow) => match self.service.as_mut() {
                Some(service) => Action::Command(
                    service
                        .command(NetworkCommand::TailscaleSetAllowLan(allow))
                        .map(Message::Event),
                ),
                _ => Action::None,
            },
            Message::TailscaleToggleCountry(country) => {
                if self.expanded_countries.contains(&country) {
                    self.expanded_countries.retain(|c| c != &country);
                } else {
                    self.expanded_countries.push(country);
                }
                Action::None
            }
            Message::SwitchVpnTab(tab) => {
                self.vpn_tab = tab;
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
                            || icon(StaticIcon::Wifi0).into(),
                            |a| {
                                let icon_type = a.get_icon();
                                let state = (service.connectivity, a.get_indicator_state());

                                container(icon(icon_type))
                                    .style(move |theme: &Theme| container::Style {
                                        text_color: match state {
                                            (ConnectivityState::Full, IndicatorState::Warning) => {
                                                Some(theme.extended_palette().danger.weak.color)
                                            }
                                            (ConnectivityState::Full, _) => None,
                                            _ => Some(theme.palette().danger),
                                        },
                                        ..Default::default()
                                    })
                                    .into()
                            },
                        ),
                )
            }
        })
    }

    pub fn vpn_indicator<'a>(&'a self, _: &AshellTheme) -> Option<Element<'a, Message>> {
        self.service.as_ref().and_then(|service| {
            // Check NM VPNs
            let nm_vpn_active = service
                .active_connections
                .iter()
                .any(|c| matches!(c, ActiveConnectionInfo::Vpn { .. }));

            // Check Tailscale
            let tailscale_active = service.tailscale.is_running;

            if nm_vpn_active || tailscale_active {
                Some(
                    container(icon(StaticIcon::Vpn))
                        .style(|theme: &Theme| container::Style {
                            text_color: Some(theme.palette().success),
                            ..Default::default()
                        })
                        .into(),
                )
            } else {
                None
            }
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
            // Check if we have NM VPNs or Tailscale
            let has_nm_vpns = service
                .known_connections
                .iter()
                .any(|c| matches!(c, KnownConnection::Vpn { .. }));
            let has_tailscale = service.tailscale.available;

            if !has_nm_vpns && !has_tailscale {
                return None;
            }

            let mut known_vpn = service.known_connections.iter().filter_map(|c| match c {
                KnownConnection::Vpn(c) => Some(c),
                _ => None,
            });
            let nm_actives: Vec<_> = service
                .active_connections
                .iter()
                .filter_map(|c| match c {
                    ActiveConnectionInfo::Vpn { name, .. } => {
                        known_vpn.find(|v| v.name == *name)
                    }
                    _ => None,
                })
                .collect();

            // Determine subtitle based on active VPNs
            let subtitle = if service.tailscale.is_running {
                let profile_name = service.tailscale.current_profile
                    .as_ref()
                    .map(|p| p.display_name())
                    .unwrap_or_else(|| "Connected".to_string());
                Some(format!("Tailscale - {}", profile_name))
            } else if nm_actives.len() > 1 {
                Some(format!("{} VPNs Connected", nm_actives.len()))
            } else {
                nm_actives.first().map(|c| c.name.clone())
            };

            let any_active = !nm_actives.is_empty() || service.tailscale.is_running;

            Some((
                quick_setting_button(
                    theme,
                    StaticIcon::Vpn,
                    "VPN".to_string(),
                    subtitle,
                    any_active,
                    if any_active && !nm_actives.is_empty() {
                        if let Some(first) = nm_actives.first() {
                            Message::ToggleVpn((*first).clone())
                        } else {
                            Message::ToggleVPNMenu
                        }
                    } else if any_active && service.tailscale.is_running {
                        Message::TailscaleDisconnect
                    } else {
                        // Smart toggle: try to reconnect to last active VPN
                        match &self.last_active_vpn {
                            Some(LastActiveVpn::Tailscale) => Message::TailscaleConnect,
                            Some(LastActiveVpn::System(vpn)) => Message::ToggleVpn(vpn.clone()),
                            None => Message::ToggleVPNMenu,
                        }
                    },
                    Some((SubMenu::Vpn, sub_menu, Message::ToggleVPNMenu)),
                ),
                sub_menu
                    .filter(|menu_type| *menu_type == SubMenu::Vpn)
                    .map(|_| {
                        self.vpn_menu(
                            service,
                            id,
                            theme,
                        )
                    }),
            ))
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
        &'a self,
        service: &'a NetworkService,
        id: Id,
        theme: &'a AshellTheme,
    ) -> Element<'a, Message> {
        let mut content = Column::new().spacing(theme.space.sm);

        // Check what's available
        let nm_vpns: Vec<_> = service.known_connections
            .iter()
            .filter_map(|c| match c {
                KnownConnection::Vpn(vpn) => Some(vpn),
                _ => None,
            })
            .collect();
        let has_nm_vpns = !nm_vpns.is_empty();
        let has_tailscale = service.tailscale.available;

        // Check which VPN is currently active
        let nm_vpn_active = service.active_connections.iter()
            .any(|c| matches!(c, ActiveConnectionInfo::Vpn { .. }));
        let tailscale_running = service.tailscale.is_running;

        // Auto-select tab based on active VPN
        let effective_tab = if tailscale_running {
            VpnTab::Tailscale
        } else if nm_vpn_active {
            VpnTab::System
        } else {
            self.vpn_tab.clone()
        };

        // If both are available, show tabs
        if has_nm_vpns && has_tailscale {
            let system_active = effective_tab == VpnTab::System;
            let tailscale_active = effective_tab == VpnTab::Tailscale;

            // Pill slider tabs - always NetworkManager first
            let tabs = container(
                row![
                    button(
                        text("NetworkManager")
                            .size(theme.font_size.sm)
                            .align_x(iced::alignment::Horizontal::Center)
                    )
                    .width(Length::Fill)
                    .padding([theme.space.xs, theme.space.sm])
                    .style(theme.pill_slider_tab_style(system_active))
                    .on_press(Message::SwitchVpnTab(VpnTab::System)),
                    button(
                        text("Tailscale")
                            .size(theme.font_size.sm)
                            .align_x(iced::alignment::Horizontal::Center)
                    )
                    .width(Length::Fill)
                    .padding([theme.space.xs, theme.space.sm])
                    .style(theme.pill_slider_tab_style(tailscale_active))
                    .on_press(Message::SwitchVpnTab(VpnTab::Tailscale)),
                ]
                .spacing(theme.space.xxs)
            )
            .padding(theme.space.xxs)
            .style(move |theme: &Theme| iced::widget::container::Style {
                background: Some(theme.extended_palette().background.weak.color.into()),
                border: Border::default().rounded(32),
                ..Default::default()
            });

            content = content.push(tabs);
            content = content.push(horizontal_rule(1));

            // Show content based on selected tab
            match effective_tab {
                VpnTab::System => {
                    content = content.push(self.system_vpns_section(service, &nm_vpns, tailscale_running, theme));
                }
                VpnTab::Tailscale => {
                    content = content.push(self.tailscale_section(service, nm_vpn_active, theme));
                }
            }
        } else if has_nm_vpns {
            // Only system VPNs
            content = content.push(self.system_vpns_section(service, &nm_vpns, false, theme));
        } else if has_tailscale {
            // Only Tailscale
            content = content.push(self.tailscale_section(service, false, theme));
        }

        // More button - tab specific
        match effective_tab {
            VpnTab::System if self.config.vpn_more_cmd.is_some() => {
                content = content
                    .push(horizontal_rule(1))
                    .push(
                        button(text("More").size(theme.font_size.sm))
                            .on_press(Message::VpnMore(id))
                            .padding([theme.space.xxs, theme.space.sm])
                            .width(Length::Fill)
                            .style(theme.ghost_button_style())
                    );
            }
            VpnTab::Tailscale if has_tailscale => {
                content = content
                    .push(horizontal_rule(1))
                    .push(
                        button(text("More").size(theme.font_size.sm))
                            .on_press(Message::TailscaleMore(id))
                            .padding([theme.space.xxs, theme.space.sm])
                            .width(Length::Fill)
                            .style(theme.ghost_button_style())
                    );
            }
            _ => {}
        }

        scrollable(content).height(Length::Shrink).into()
    }

    fn system_vpns_section<'a>(
        &'a self,
        service: &'a NetworkService,
        nm_vpns: &[&'a Vpn],
        other_vpn_active: bool,
        theme: &'a AshellTheme,
    ) -> Element<'a, Message> {
        let mut section = Column::new().spacing(theme.space.xs);

        // Show warning if other VPN is active
        if other_vpn_active {
            section = section.push(
                text("Disconnect Tailscale to enable NetworkManager VPN")
                    .size(theme.font_size.sm)
                    .font(iced::Font {
                        style: iced::font::Style::Italic,
                        ..Default::default()
                    })
                    .style(|t: &Theme| text::Style {
                        color: Some(t.extended_palette().danger.weak.color),
                    })
            );
        }

        for vpn in nm_vpns {
            let is_active = service.active_connections.iter().any(
                |c| matches!(c, ActiveConnectionInfo::Vpn { name, .. } if name == &vpn.name),
            );

            let mut vpn_toggler = toggler(is_active)
                .width(Length::Shrink);
            
            // Only allow toggling if other VPN is not active
            if !other_vpn_active || is_active {
                vpn_toggler = vpn_toggler.on_toggle(|_| Message::ToggleVpn((*vpn).clone()));
            }

            section = section.push(
                row!(
                    text(vpn.name.to_string()).width(Length::Fill).size(theme.font_size.sm),
                    vpn_toggler,
                )
                .align_y(Alignment::Center)
            );
        }

        section.into()
    }

    fn tailscale_section<'a>(
        &'a self,
        service: &'a NetworkService,
        other_vpn_active: bool,
        theme: &'a AshellTheme,
    ) -> Element<'a, Message> {
        let ts = &service.tailscale;
        let mut section = Column::new().spacing(theme.space.xs);

        // Show warning if other VPN is active
        if other_vpn_active && !ts.is_running {
            section = section.push(
                text("Disconnect System VPN to enable Tailscale")
                    .size(theme.font_size.sm)
                    .font(iced::Font {
                        style: iced::font::Style::Italic,
                        ..Default::default()
                    })
                    .style(|t: &Theme| text::Style {
                        color: Some(t.extended_palette().danger.weak.color),
                    })
            );
        }

        // Profile selector (if multiple profiles)
        if ts.profiles.len() > 1 {
            let profile_names: Vec<String> = ts.profiles.iter()
                .map(|p| p.display_name())
                .collect();
            
            let current_profile = ts.current_profile.as_ref().map(|p| p.display_name());

            section = section.push(
                row![
                    text("Profile:").width(Length::Shrink).size(theme.font_size.sm),
                    iced::widget::pick_list(
                        profile_names,
                        current_profile,
                        |name| {
                            // Find the profile ID by display name
                            Message::TailscaleSwitchProfile(name)
                        }
                    )
                    .text_size(theme.font_size.sm)
                    .width(Length::Fill),
                ]
                .spacing(theme.space.xs)
                .align_y(Alignment::Center)
            );
        }

        // Connection toggle
        let mut connection_toggler = toggler(ts.is_running)
            .width(Length::Shrink);
        
        // Only allow connecting if other VPN is not active (but always allow disconnecting)
        if !other_vpn_active || ts.is_running {
            connection_toggler = connection_toggler.on_toggle(|enabled| {
                if enabled {
                    Message::TailscaleConnect
                } else {
                    Message::TailscaleDisconnect
                }
            });
        }

        section = section.push(
            row![
                text(if ts.is_running { "Connected" } else { "Disconnected" })
                    .width(Length::Fill)
                    .size(theme.font_size.sm),
                connection_toggler,
            ]
            .align_y(Alignment::Center)
        );

        // Exit nodes section (only if connected and has exit nodes)
        if ts.is_running && !ts.exit_nodes.is_empty() {
            section = section.push(horizontal_rule(1));
            section = section.push(
                text("Exit Nodes").size(theme.font_size.sm)
                    .style(|t: &Theme| text::Style {
                        color: Some(t.extended_palette().secondary.base.text),
                    })
            );

            // Current exit node status
            let current_exit = ts.exit_nodes.iter()
                .find(|n| ts.current_exit_node_id.as_ref() == Some(&n.id));

            let status_row = if let Some(node) = current_exit {
                row![
                    text("●").size(theme.font_size.sm).style(|t: &Theme| text::Style {
                        color: Some(t.palette().success),
                    }),
                    text(node.display_name()).size(theme.font_size.sm),
                    horizontal_space(),
                    button(text("Clear").size(theme.font_size.sm))
                        .padding([theme.space.xxs, theme.space.xs])
                        .style(theme.ghost_button_style())
                        .on_press(Message::TailscaleSetExitNode(None)),
                ]
                .spacing(theme.space.xs)
                .align_y(Alignment::Center)
            } else {
                row![
                    text("○").size(theme.font_size.sm),
                    text("No exit node").size(theme.font_size.sm),
                ]
                .spacing(theme.space.xs)
                .align_y(Alignment::Center)
            };

            section = section.push(status_row);

            // Group exit nodes by country
            let by_country = self.group_exit_nodes_by_country(&ts.exit_nodes);
            let mut countries: Vec<_> = by_country.keys().collect();
            countries.sort();

            let mut exit_list = Column::new().spacing(1);

            // None option
            let no_exit_selected = ts.current_exit_node_id.is_none();
            let none_btn = button(
                row![
                    text(if no_exit_selected { "●" } else { "○" }).size(theme.font_size.sm),
                    text("None (Direct)").size(theme.font_size.sm),
                ]
                .spacing(theme.space.xs)
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .padding([theme.space.xxs, theme.space.xs])
            .style(move |t, s| {
                if no_exit_selected {
                    theme.settings_button_style()(t, s)
                } else {
                    theme.ghost_button_style()(t, s)
                }
            })
            .on_press_maybe(if no_exit_selected {
                None
            } else {
                Some(Message::TailscaleSetExitNode(None))
            });

            exit_list = exit_list.push(none_btn);

            // Countries with nodes
            for country in countries {
                if let Some(nodes) = by_country.get(country) {
                    let is_expanded = self.expanded_countries.contains(country);
                    let has_selected = nodes.iter().any(|n| 
                        ts.current_exit_node_id.as_ref() == Some(&n.id)
                    );

                    // Country row
                    let country_row = button(
                        row![
                            text(if is_expanded { "▾" } else { "▸" }).size(theme.font_size.sm),
                            text(country.clone()).size(theme.font_size.sm),
                            text(format!("({})", nodes.len()))
                                .size(theme.font_size.sm)
                                .style(|t: &Theme| text::Style {
                                    color: Some(t.extended_palette().secondary.base.text),
                                }),
                            horizontal_space(),
                            if has_selected {
                                text("●").size(theme.font_size.sm)
                            } else {
                                text("").size(theme.font_size.sm)
                            },
                        ]
                        .spacing(theme.space.xxs)
                        .align_y(Alignment::Center),
                    )
                    .width(Length::Fill)
                    .padding([theme.space.xxs, theme.space.xs])
                    .style(move |t, s| {
                        if has_selected && !is_expanded {
                            theme.settings_button_style()(t, s)
                        } else {
                            theme.ghost_button_style()(t, s)
                        }
                    })
                    .on_press(Message::TailscaleToggleCountry(country.clone()));

                    exit_list = exit_list.push(country_row);

                    // Expanded nodes
                    if is_expanded {
                        for node in nodes.iter().take(15) {
                            let is_current = ts.current_exit_node_id.as_ref() == Some(&node.id);

                            let node_btn = button(
                                row![
                                    text(if is_current { "●" } else { "○" }).size(theme.font_size.sm),
                                    text(node.display_name()).size(theme.font_size.sm),
                                ]
                                .spacing(theme.space.xs)
                                .align_y(Alignment::Center),
                            )
                            .width(Length::Fill)
                            .padding([theme.space.xxs, theme.space.md])
                            .style(move |t, s| {
                                if is_current {
                                    theme.settings_button_style()(t, s)
                                } else {
                                    theme.ghost_button_style()(t, s)
                                }
                            })
                            .on_press_maybe(if is_current {
                                None
                            } else {
                                Some(Message::TailscaleSetExitNode(Some(node.id.clone())))
                            });

                            exit_list = exit_list.push(node_btn);
                        }
                    }
                }
            }

            section = section.push(
                scrollable(exit_list)
                    .height(Length::Fixed(150.0))
            );

            // Allow LAN toggle (only when using exit node)
            if ts.current_exit_node_id.is_some() {
                section = section.push(
                    row![
                        text("Allow LAN Access").width(Length::Fill).size(theme.font_size.sm),
                        toggler(ts.allow_lan)
                            .on_toggle(Message::TailscaleSetAllowLan)
                            .width(Length::Shrink),
                    ]
                    .align_y(Alignment::Center)
                );
            }
        }

        section.into()
    }

    fn group_exit_nodes_by_country<'a>(&self, nodes: &'a [ExitNode]) -> HashMap<String, Vec<&'a ExitNode>> {
        let mut by_country: HashMap<String, Vec<&ExitNode>> = HashMap::new();
        for node in nodes.iter().filter(|n| n.online) {
            let country = node.country();
            by_country.entry(country).or_default().push(node);
        }
        by_country
    }

    pub fn subscription(&self) -> Subscription<Message> {
        NetworkService::subscribe().map(Message::Event)
    }
}
