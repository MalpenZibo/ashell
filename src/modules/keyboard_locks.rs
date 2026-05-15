use crate::{
    components::icons::{IconKind, StaticIcon, icon_mono},
    config::{KeyboardLocksModuleConfig, LockIndicatorConfig, LockVisibility},
    services::{
        ReadOnlyService, Service, ServiceEvent,
        keyboard_locks::{KeyboardLocksCommand, KeyboardLocksService, LockKind},
    },
    theme::use_theme,
};
use iced::{
    Alignment, Element, Length, Subscription,
    widget::{Row, button, container},
};

#[derive(Debug, Clone)]
pub enum Message {
    ServiceEvent(ServiceEvent<KeyboardLocksService>),
    ConfigReloaded(KeyboardLocksModuleConfig),
    Toggle(LockKind),
}

pub struct KeyboardLocks {
    config: KeyboardLocksModuleConfig,
    service: Option<KeyboardLocksService>,
}

impl KeyboardLocks {
    pub fn new(config: KeyboardLocksModuleConfig) -> Self {
        Self {
            config,
            service: None,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::ServiceEvent(event) => match event {
                ServiceEvent::Init(service) => self.service = Some(service),
                ServiceEvent::Update(update) => {
                    if let Some(service) = self.service.as_mut() {
                        service.update(update);
                    }
                }
                ServiceEvent::Error(_) => {}
            },
            Message::ConfigReloaded(config) => self.config = config,
            Message::Toggle(kind) => {
                if let Some(service) = self.service.as_mut() {
                    let _ = service.command(KeyboardLocksCommand::Toggle(kind));
                }
            }
        }
    }

    pub fn view(&self) -> Option<Element<'_, Message>> {
        let data = self.service.as_ref().map(|s| s.data).unwrap_or_default();
        let inactive_color = use_theme(|t| t.iced_theme.extended_palette().background.strong.color);

        let entries = [
            (
                &self.config.caps_lock,
                data.caps_lock,
                StaticIcon::CapsLock,
                LockKind::Caps,
            ),
            (
                &self.config.num_lock,
                data.num_lock,
                StaticIcon::NumLock,
                LockKind::Num,
            ),
            (
                &self.config.scroll_lock,
                data.scroll_lock,
                StaticIcon::ScrollLock,
                LockKind::Scroll,
            ),
        ];

        let mut row = Row::new();
        let mut any_visible = false;
        for (cfg, active, default_icon, kind) in entries {
            if let Some(element) = render_indicator(cfg, active, default_icon, kind, inactive_color)
            {
                row = row.push(element);
                any_visible = true;
            }
        }

        if any_visible { Some(row.into()) } else { None }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        KeyboardLocksService::subscribe().map(Message::ServiceEvent)
    }
}

fn render_indicator<'a>(
    cfg: &'a LockIndicatorConfig,
    active: bool,
    default_icon: StaticIcon,
    kind: LockKind,
    inactive_color: iced::Color,
) -> Option<Element<'a, Message>> {
    if !cfg.enabled {
        return None;
    }
    let visible = match cfg.visibility {
        LockVisibility::ActiveOnly => active,
        LockVisibility::AlwaysVisible => true,
    };
    if !visible {
        return None;
    }

    let icon_kind: IconKind = match cfg.icon.as_ref() {
        Some(glyph) => IconKind::Dynamic(glyph.clone()),
        None => IconKind::from(default_icon),
    };

    let (font_size, space, module_button_style) =
        use_theme(|t| (t.font_size.xs, t.space, t.module_button_style()));

    let glyph = icon_mono(icon_kind)
        .size(font_size)
        .color_maybe((!active).then_some(inactive_color));

    Some(
        button(
            container(glyph)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .height(Length::Fill),
        )
        .padding([0.0, space.xxs])
        .height(Length::Fill)
        .style(module_button_style)
        .on_press(Message::Toggle(kind))
        .into(),
    )
}
