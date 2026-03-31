use std::time::Duration;

pub mod launcher;
pub mod remote_value;

/// Extension trait to add `push_maybe` to `Row` and `Column` in iced 0.14,
/// which removed the built-in method.
pub trait PushMaybe<'a, Message, Theme, Renderer> {
    fn push_maybe(
        self,
        child: Option<impl Into<iced_layershell::core::Element<'a, Message, Theme, Renderer>>>,
    ) -> Self;
}

impl<'a, Message, Theme, Renderer> PushMaybe<'a, Message, Theme, Renderer>
    for iced_layershell::widget::Row<'a, Message, Theme, Renderer>
where
    Renderer: iced_layershell::core::Renderer,
{
    fn push_maybe(
        self,
        child: Option<impl Into<iced_layershell::core::Element<'a, Message, Theme, Renderer>>>,
    ) -> Self {
        match child {
            Some(child) => self.push(child),
            None => self,
        }
    }
}

impl<'a, Message, Theme, Renderer> PushMaybe<'a, Message, Theme, Renderer>
    for iced_layershell::widget::Column<'a, Message, Theme, Renderer>
where
    Renderer: iced_layershell::core::Renderer,
{
    fn push_maybe(
        self,
        child: Option<impl Into<iced_layershell::core::Element<'a, Message, Theme, Renderer>>>,
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

pub fn floor_dp(num: f32, dp: i32) -> f32 {
    (num * 10_f32.powi(dp)).floor() / 10_f32.powi(dp)
}

pub fn bytes_to_gib(bytes: u64) -> f32 {
    bytes as f32 / 1_073_741_824_f32
}
pub fn bytes_to_gb(bytes: u64) -> f32 {
    bytes as f32 / 1_000_000_000_f32
}

pub fn celsius_to_fahrenheit(cel: i32) -> i32 {
    cel * 9 / 5 + 32
}
