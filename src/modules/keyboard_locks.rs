use crate::{
    components::icons::{IconKind, StaticIcon, icon_mono},
    config::{KeyboardLocksModuleConfig, LockIndicatorConfig, LockVisibility, ModuleName},
    services::{ReadOnlyService, ServiceEvent, keyboard_locks::KeyboardLocksService},
    theme::use_theme,
};
use iced::{Alignment, Element, Subscription, widget::Row};

#[derive(Debug, Clone)]
pub enum Message {
    Event(ServiceEvent<KeyboardLocksService>),
    ConfigReloaded(KeyboardLocksModuleConfig),
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
            Message::Event(event) => match event {
                ServiceEvent::Init(service) => self.service = Some(service),
                ServiceEvent::Update(data) => {
                    if let Some(service) = self.service.as_mut() {
                        service.update(data);
                    }
                }
                ServiceEvent::Error(_) => {}
            },
            Message::ConfigReloaded(config) => self.config = config,
        }
    }

    pub fn view(&self) -> Option<Element<'_, Message>> {
        let service = self.service.as_ref()?;
        let (spacing, size, off_color) = use_theme(|theme| {
            (
                theme.module_spacing(&ModuleName::KeyboardLocks, theme.space.xs),
                theme.module_font_size(&ModuleName::KeyboardLocks, theme.font_size.md),
                theme.palette.text.scale_alpha(0.4),
            )
        });

        let indicators = [
            (
                &self.config.caps_lock,
                service.caps_lock,
                StaticIcon::CapsLock,
            ),
            (&self.config.num_lock, service.num_lock, StaticIcon::NumLock),
            (
                &self.config.scroll_lock,
                service.scroll_lock,
                StaticIcon::ScrollLock,
            ),
        ]
        .into_iter()
        .filter_map(|(config, on, default_icon)| {
            let on = on?;
            let shown =
                config.enabled && (on || config.visibility == LockVisibility::AlwaysVisible);
            shown.then(|| indicator(config, on, default_icon, size, off_color))
        })
        .collect::<Vec<_>>();

        (!indicators.is_empty()).then(|| {
            Row::from_vec(indicators)
                .align_y(Alignment::Center)
                .spacing(spacing)
                .into()
        })
    }

    pub fn subscription(&self) -> Subscription<Message> {
        KeyboardLocksService::subscribe().map(Message::Event)
    }
}

fn indicator<'a>(
    config: &LockIndicatorConfig,
    on: bool,
    default_icon: StaticIcon,
    size: f32,
    off_color: iced::Color,
) -> Element<'a, Message> {
    let icon = match &config.icon {
        Some(glyph) => IconKind::Dynamic(glyph.clone()),
        None => IconKind::Static(default_icon),
    };
    icon_mono(icon)
        .size(size)
        .color_maybe((!on).then_some(off_color))
        .into()
}
