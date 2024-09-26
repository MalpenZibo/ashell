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
        format!("{}h {:>2}m", h, m)
    } else {
        format!("{:>2}m", m)
    }
}
