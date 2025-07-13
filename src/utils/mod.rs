use std::time::Duration;

pub mod launcher;

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
