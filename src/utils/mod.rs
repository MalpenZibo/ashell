use std::time::Duration;

pub mod launcher;
pub mod remote_value;

/// Extension trait to add `push_maybe` to `Row` and `Column` in iced 0.14,
/// which removed the built-in method.
pub trait PushMaybe<'a, Message, Theme, Renderer> {
    fn push_maybe(
        self,
        child: Option<impl Into<iced::core::Element<'a, Message, Theme, Renderer>>>,
    ) -> Self;
}

impl<'a, Message, Theme, Renderer> PushMaybe<'a, Message, Theme, Renderer>
    for iced::widget::Row<'a, Message, Theme, Renderer>
where
    Renderer: iced::core::Renderer,
{
    fn push_maybe(
        self,
        child: Option<impl Into<iced::core::Element<'a, Message, Theme, Renderer>>>,
    ) -> Self {
        match child {
            Some(child) => self.push(child),
            None => self,
        }
    }
}

impl<'a, Message, Theme, Renderer> PushMaybe<'a, Message, Theme, Renderer>
    for iced::widget::Column<'a, Message, Theme, Renderer>
where
    Renderer: iced::core::Renderer,
{
    fn push_maybe(
        self,
        child: Option<impl Into<iced::core::Element<'a, Message, Theme, Renderer>>>,
    ) -> Self {
        match child {
            Some(child) => self.push(child),
            None => self,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum IndicatorState {
    Normal,
    Success,
    Warning,
    Danger,
}

pub fn format_duration(duration: &Duration) -> String {
    let h = duration.as_secs() / 60 / 60;
    let m = duration.as_secs() / 60 % 60;
    if h > 0 {
        format!("{h}h {m:>2}m")
    } else {
        format!("{m:>2}m")
    }
}

pub fn truncate_text(value: &str, max_length: u32) -> String {
    let length = value.len();

    if length > max_length as usize {
        let split = max_length as usize / 2;
        let first_part = value.chars().take(split).collect::<String>();
        let last_part = value.chars().skip(length - split).collect::<String>();
        format!("{first_part}...{last_part}")
    } else {
        value.to_string()
    }
}
