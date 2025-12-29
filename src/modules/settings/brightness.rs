use crate::{
    components::icons::{StaticIcon, icon_mono},
    services::{
        ReadOnlyService, Service, ServiceEvent,
        brightness::{BrightnessCommand, BrightnessService},
    },
    theme::AshellTheme,
};
use iced::{
    Alignment, Element, Length, Subscription, Task,
    futures::stream,
    widget::{MouseArea, container, row, slider},
};

#[derive(Debug, Clone)]
pub enum Message {
    Event(ServiceEvent<BrightnessService>),
    Change(u32),
    MenuOpened,
    ResetUserAdjusting,
}

pub enum Action {
    None,
    Command(Task<Message>),
}

pub struct BrightnessSettings {
    service: Option<BrightnessService>,
    ui_percentage: u32,
    is_user_adjusting: bool,
    reset_timer_active: bool,
}

impl BrightnessSettings {
    pub fn new() -> Self {
        Self {
            service: None,
            ui_percentage: 50,
            is_user_adjusting: false,
            reset_timer_active: false,
        }
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::Event(event) => match event {
                ServiceEvent::Init(service) => {
                    self.ui_percentage = service.current * 100 / service.max;
                    self.service = Some(service);
                    Action::None
                }
                ServiceEvent::Update(data) => {
                    if let Some(service) = self.service.as_mut() {
                        service.update(data);
                        // Only update UI if the difference is significant and user isn't actively adjusting
                        if !self.is_user_adjusting {
                            let new_percentage = service.current * 100 / service.max;
                            if (new_percentage as i32 - self.ui_percentage as i32).abs() > 2 {
                                self.ui_percentage = new_percentage;
                            }
                        }
                    }
                    Action::None
                }
                _ => Action::None,
            },
            Message::Change(value) => {
                self.is_user_adjusting = true;
                self.reset_timer_active = true;
                self.ui_percentage = value * 100
                    / if let Some(service) = &self.service {
                        service.max
                    } else {
                        100
                    };
                match self.service.as_mut() {
                    Some(service) => Action::Command(
                        service
                            .command(BrightnessCommand::Set(value))
                            .map(Message::Event),
                    ),
                    _ => Action::None,
                }
            }
            Message::MenuOpened => {
                if let Some(service) = self.service.as_mut() {
                    Action::Command(
                        service
                            .command(BrightnessCommand::Refresh)
                            .map(Message::Event),
                    )
                } else {
                    Action::None
                }
            }
            Message::ResetUserAdjusting => {
                self.is_user_adjusting = false;
                self.reset_timer_active = false;
                Action::None
            }
        }
    }

    pub fn slider(&'_ self, theme: &AshellTheme) -> Option<Element<'_, Message>> {
        self.service.as_ref().map(|service| {
            let max = service.max;
            let current_percentage = self.ui_percentage;
            row!(
                container(icon_mono(StaticIcon::Brightness))
                    .center_x(32.)
                    .center_y(32.)
                    .clip(true),
                MouseArea::new(
                    slider(0..=100, current_percentage, move |v| {
                        Message::Change(v * max / 100)
                    })
                    .step(1_u32)
                    .width(Length::Fill),
                )
                .on_scroll(move |delta| {
                    let delta = match delta {
                        iced::mouse::ScrollDelta::Lines { y, .. } => y,
                        iced::mouse::ScrollDelta::Pixels { y, .. } => y,
                    };
                    // brightness is always changed by one less than expected
                    let new_percentage = if delta > 0.0 {
                        (current_percentage + 5 + 1).min(100)
                    } else {
                        current_percentage.saturating_sub(5 + 1)
                    };
                    let new_brightness_value = new_percentage * max / 100;
                    Message::Change(new_brightness_value)
                }),
            )
            .align_y(Alignment::Center)
            .spacing(theme.space.xs)
            .into()
        })
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            BrightnessService::subscribe().map(Message::Event),
            if self.reset_timer_active {
                Subscription::run_with_id(
                    0,
                    stream::once(async {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        Message::ResetUserAdjusting
                    }),
                )
            } else {
                Subscription::none()
            },
        ])
    }
}
