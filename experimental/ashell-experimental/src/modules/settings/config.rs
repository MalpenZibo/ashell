use toml::Table;

/// `[settings]`: the commands of the panel's power buttons. One that is
/// unset or empty has no button.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingsConfig {
    pub lock: Option<String>,
    pub suspend: Option<String>,
    pub hibernate: Option<String>,
    pub reboot: Option<String>,
    pub shutdown: Option<String>,
    pub logout: Option<String>,
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
        }
    }
}

impl SettingsConfig {
    pub fn read(table: &Table) -> (Self, Vec<String>) {
        let mut problems = Vec::new();
        let mut config = Self::default();

        for (key, value) in table {
            let slot = match key.as_str() {
                "lock" => &mut config.lock,
                "suspend" => &mut config.suspend,
                "hibernate" => &mut config.hibernate,
                "reboot" => &mut config.reboot,
                "shutdown" => &mut config.shutdown,
                "logout" => &mut config.logout,
                _ => {
                    problems.push(format!("settings.{key}: unknown key, ignored"));
                    continue;
                }
            };

            match value.as_str() {
                Some(command) => {
                    *slot = Some(command.trim())
                        .filter(|c| !c.is_empty())
                        .map(Into::into)
                }
                None => problems.push(format!("settings.{key}: expected a command")),
            }
        }

        (config, problems)
    }
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
}
