use std::ops::RangeInclusive;

use toml::{Table, Value};

/// `[settings]`: the commands of the panel's power buttons, one that is unset
/// or empty having no button, and the volume's limit and step.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingsConfig {
    pub lock: Option<String>,
    pub suspend: Option<String>,
    pub hibernate: Option<String>,
    pub reboot: Option<String>,
    pub shutdown: Option<String>,
    pub logout: Option<String>,
    /// The output's highest volume, in percent: above 100 it is amplified.
    pub max_volume: u32,
    /// The percent a step of the volume or the microphone moves.
    pub volume_step: u32,
}

impl Default for SettingsConfig {
    /// ashell's.
    fn default() -> Self {
        Self {
            lock: None,
            suspend: Some("systemctl suspend".into()),
            hibernate: None,
            reboot: Some("systemctl reboot".into()),
            shutdown: Some("shutdown now".into()),
            logout: Some("loginctl kill-user $(whoami)".into()),
            max_volume: 100,
            volume_step: 5,
        }
    }
}

impl SettingsConfig {
    pub fn read(table: &Table) -> (Self, Vec<String>) {
        let mut problems = Vec::new();
        let mut config = Self::default();

        for (key, value) in table {
            let problem = match key.as_str() {
                "max_volume" => percent(&mut config.max_volume, value, 1..=200),
                "volume_step" => percent(&mut config.volume_step, value, 1..=50),
                "lock" => command(&mut config.lock, value),
                "suspend" => command(&mut config.suspend, value),
                "hibernate" => command(&mut config.hibernate, value),
                "reboot" => command(&mut config.reboot, value),
                "shutdown" => command(&mut config.shutdown, value),
                "logout" => command(&mut config.logout, value),
                _ => Some("unknown key, ignored".into()),
            };

            if let Some(problem) = problem {
                problems.push(format!("settings.{key}: {problem}"));
            }
        }

        (config, problems)
    }
}

/// A limit or a step, as ashell takes it.
fn percent(slot: &mut u32, value: &Value, range: RangeInclusive<u32>) -> Option<String> {
    match value.as_integer().and_then(|n| u32::try_from(n).ok()) {
        Some(n) if range.contains(&n) => {
            *slot = n;
            None
        }
        _ => Some(format!(
            "expected a percentage from {} to {}",
            range.start(),
            range.end()
        )),
    }
}

/// A power button's command: one of spaces alone has no button.
fn command(slot: &mut Option<String>, value: &Value) -> Option<String> {
    let Some(command) = value.as_str() else {
        return Some("expected a command".into());
    };

    *slot = Some(command.trim())
        .filter(|c| !c.is_empty())
        .map(Into::into);
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> (SettingsConfig, Vec<String>) {
        SettingsConfig::read(&toml::from_str(text).unwrap())
    }

    #[test]
    fn commands_replace_ashells_and_empty_ones_remove_a_button() {
        let (config, problems) = read(
            r#"
lock = "hyprlock"
logout = ""
reboot = "  "
"#,
        );

        assert!(problems.is_empty());
        assert_eq!(config.lock.as_deref(), Some("hyprlock"));
        assert_eq!(config.logout, None);
        assert_eq!(config.reboot, None);
        assert_eq!(config.suspend.as_deref(), Some("systemctl suspend"));
    }

    #[test]
    fn what_is_not_a_command_is_named() {
        let (config, problems) = read("reboot = 1\nsleep = \"x\"");

        assert_eq!(config.reboot, SettingsConfig::default().reboot);
        assert_eq!(
            problems,
            [
                "settings.reboot: expected a command",
                "settings.sleep: unknown key, ignored"
            ]
        );
    }

    #[test]
    fn the_volume_takes_a_limit_and_a_step() {
        let (config, problems) = read("max_volume = 150\nvolume_step = 2");

        assert!(problems.is_empty());
        assert_eq!((config.max_volume, config.volume_step), (150, 2));
    }

    #[test]
    fn a_limit_or_a_step_out_of_range_keeps_ashells() {
        let (config, problems) = read("max_volume = 0\nvolume_step = \"big\"");

        assert_eq!((config.max_volume, config.volume_step), (100, 5));
        assert_eq!(
            problems,
            [
                "settings.max_volume: expected a percentage from 1 to 200",
                "settings.volume_step: expected a percentage from 1 to 50"
            ]
        );
    }
}
