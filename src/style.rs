use crate::config::{Appearance, AppearanceColor, AppearanceStyle};
use iced::{
    Background, Border, Color, Theme,
    daemon::DefaultStyle,
    runtime::default,
    theme::{Palette, palette},
    widget::{
        button::{self, Status},
        text_input::{self},
    },
};

#[derive(Debug, Default, Clone)]
pub struct AshellTheme {
    iced_theme: Theme,
}

impl AshellTheme {
    pub fn new(appearance: &Appearance) -> Self {
        AshellTheme {
            iced_theme: Theme::custom_with_fn(
                "local".to_string(),
                Palette {
                    background: appearance.background_color.get_base(),
                    text: appearance.text_color.get_base(),
                    primary: appearance.primary_color.get_base(),
                    success: appearance.success_color.get_base(),
                    danger: appearance.danger_color.get_base(),
                },
                |palette| {
                    let default_bg = palette::Background::new(
                        palette.background,
                        appearance
                            .background_color
                            .get_text()
                            .unwrap_or(palette.text),
                    );
                    let default_primary = palette::Primary::generate(
                        palette.primary,
                        palette.background,
                        appearance.primary_color.get_text().unwrap_or(palette.text),
                    );
                    let default_secondary = palette::Primary::generate(
                        appearance.secondary_color.get_base(),
                        palette.background,
                        appearance
                            .secondary_color
                            .get_text()
                            .unwrap_or(palette.text),
                    );
                    let default_success = palette::Success::generate(
                        palette.success,
                        palette.background,
                        appearance.success_color.get_text().unwrap_or(palette.text),
                    );
                    let default_danger = palette::Danger::generate(
                        palette.danger,
                        palette.background,
                        appearance.danger_color.get_text().unwrap_or(palette.text),
                    );

                    palette::Extended {
                        background: palette::Background {
                            base: default_bg.base,
                            weak: appearance
                                .background_color
                                .get_weak_pair(palette.text)
                                .unwrap_or(default_bg.weak),
                            strong: appearance
                                .background_color
                                .get_strong_pair(palette.text)
                                .unwrap_or(default_bg.strong),
                        },
                        primary: palette::Primary {
                            base: default_primary.base,
                            weak: appearance
                                .primary_color
                                .get_weak_pair(palette.text)
                                .unwrap_or(default_primary.weak),
                            strong: appearance
                                .primary_color
                                .get_strong_pair(palette.text)
                                .unwrap_or(default_primary.strong),
                        },
                        secondary: palette::Secondary {
                            base: default_secondary.base,
                            weak: appearance
                                .secondary_color
                                .get_weak_pair(palette.text)
                                .unwrap_or(default_secondary.weak),
                            strong: appearance
                                .secondary_color
                                .get_strong_pair(palette.text)
                                .unwrap_or(default_secondary.strong),
                        },
                        success: palette::Success {
                            base: default_success.base,
                            weak: appearance
                                .success_color
                                .get_weak_pair(palette.text)
                                .unwrap_or(default_success.weak),
                            strong: appearance
                                .success_color
                                .get_strong_pair(palette.text)
                                .unwrap_or(default_success.strong),
                        },
                        danger: palette::Danger {
                            base: default_danger.base,
                            weak: appearance
                                .danger_color
                                .get_weak_pair(palette.text)
                                .unwrap_or(default_danger.weak),
                            strong: appearance
                                .danger_color
                                .get_strong_pair(palette.text)
                                .unwrap_or(default_danger.strong),
                        },
                        is_dark: true,
                    }
                },
            ),
        }
    }

    pub fn iced_theme(&self) -> &Theme {
        &self.iced_theme
    }
}

impl DefaultStyle for AshellTheme {
    fn default_style(&self) -> iced::program::Appearance {
        default(&self.iced_theme)
    }
}

/// Note: the transparent argument, when true, makes the base color bg
/// transparent but still has a hover bg color. Not to be confused with opacity,
/// which affects opacity at all times.
pub fn module_button_style(
    style: AppearanceStyle,
    opacity: f32,
    transparent: bool,
) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let mut base = button::Style {
            background: match style {
                AppearanceStyle::Solid | AppearanceStyle::Gradient => None,
                AppearanceStyle::Islands => {
                    if transparent {
                        None
                    } else {
                        Some(theme.palette().background.scale_alpha(opacity).into())
                    }
                }
            },
            border: Border {
                width: 0.0,
                radius: 12.0.into(),
                color: Color::TRANSPARENT,
            },
            text_color: theme.palette().text,
            ..button::Style::default()
        };
        match status {
            Status::Active => base,
            Status::Hovered => {
                base.background = Some(
                    theme
                        .extended_palette()
                        .background
                        .weak
                        .color
                        .scale_alpha(opacity)
                        .into(),
                );
                base
            }
            _ => base,
        }
    }
}

pub fn ghost_button_style(opacity: f32) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let mut base = button::Style {
            background: None,
            border: Border {
                width: 0.0,
                radius: 4.0.into(),
                color: Color::TRANSPARENT,
            },
            text_color: theme.palette().text,
            ..button::Style::default()
        };
        match status {
            Status::Active => base,
            Status::Hovered => {
                base.background = Some(
                    theme
                        .extended_palette()
                        .background
                        .weak
                        .color
                        .scale_alpha(opacity)
                        .into(),
                );
                base
            }
            _ => base,
        }
    }
}

pub fn outline_button_style(opacity: f32) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let mut base = button::Style {
            background: None,
            border: Border {
                width: 2.0,
                radius: 32.into(),
                color: theme.extended_palette().background.weak.color,
            },
            text_color: theme.palette().text,
            ..button::Style::default()
        };
        match status {
            Status::Active => base,
            Status::Hovered => {
                base.background = Some(
                    theme
                        .extended_palette()
                        .background
                        .weak
                        .color
                        .scale_alpha(opacity)
                        .into(),
                );
                base
            }
            _ => base,
        }
    }
}

pub fn confirm_button_style(opacity: f32) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let mut base = button::Style {
            background: Some(
                theme
                    .extended_palette()
                    .background
                    .weak
                    .color
                    .scale_alpha(opacity)
                    .into(),
            ),
            border: Border {
                width: 2.0,
                radius: 32.0.into(),
                color: Color::TRANSPARENT,
            },
            text_color: theme.palette().text,
            ..button::Style::default()
        };
        match status {
            Status::Active => base,
            Status::Hovered => {
                base.background = Some(
                    theme
                        .extended_palette()
                        .background
                        .strong
                        .color
                        .scale_alpha(opacity)
                        .into(),
                );
                base
            }
            _ => base,
        }
    }
}

pub fn settings_button_style(opacity: f32) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme, status| {
        let mut base = button::Style {
            background: Some(
                theme
                    .extended_palette()
                    .background
                    .weak
                    .color
                    .scale_alpha(opacity)
                    .into(),
            ),
            border: Border {
                width: 0.0,
                radius: 32.0.into(),
                color: Color::TRANSPARENT,
            },
            text_color: theme.palette().text,
            ..button::Style::default()
        };
        match status {
            Status::Active => base,
            Status::Hovered => {
                base.background = Some(
                    theme
                        .extended_palette()
                        .background
                        .strong
                        .color
                        .scale_alpha(opacity)
                        .into(),
                );
                base
            }
            _ => base,
        }
    }
}

pub fn workspace_button_style(
    is_empty: bool,
    colors: Option<Option<AppearanceColor>>,
) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme: &Theme, status: Status| {
        let (bg_color, fg_color) = colors
            .map(|c| {
                c.map_or(
                    (
                        theme.extended_palette().primary.base.color,
                        theme.extended_palette().primary.base.text,
                    ),
                    |c| {
                        let color = palette::Primary::generate(
                            c.get_base(),
                            theme.palette().background,
                            c.get_text().unwrap_or(theme.palette().text),
                        );
                        (color.base.color, color.base.text)
                    },
                )
            })
            .unwrap_or((
                theme.extended_palette().background.weak.color,
                theme.palette().text,
            ));
        let mut base = button::Style {
            background: Some(Background::Color(if is_empty {
                theme.extended_palette().background.weak.color
            } else {
                bg_color
            })),
            border: Border {
                width: if is_empty { 1.0 } else { 0.0 },
                color: bg_color,
                radius: 16.0.into(),
            },
            text_color: if is_empty {
                theme.extended_palette().background.weak.text
            } else {
                fg_color
            },
            ..button::Style::default()
        };
        match status {
            Status::Active => base,
            Status::Hovered => {
                let (bg_color, fg_color) = colors
                    .map(|c| {
                        c.map_or(
                            (
                                theme.extended_palette().primary.strong.color,
                                theme.extended_palette().primary.strong.text,
                            ),
                            |c| {
                                let color = palette::Primary::generate(
                                    c.get_base(),
                                    theme.palette().background,
                                    c.get_text().unwrap_or(theme.palette().text),
                                );
                                (color.strong.color, color.strong.text)
                            },
                        )
                    })
                    .unwrap_or((
                        theme.extended_palette().background.strong.color,
                        theme.palette().text,
                    ));

                base.background = Some(Background::Color(if is_empty {
                    theme.extended_palette().background.strong.color
                } else {
                    bg_color
                }));
                base.text_color = if is_empty {
                    theme.extended_palette().background.weak.text
                } else {
                    fg_color
                };
                base
            }
            _ => base,
        }
    }
}

pub fn quick_settings_button_style(
    is_active: bool,
    opacity: f32,
) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme: &Theme, status: Status| {
        let mut base = button::Style {
            background: Some(
                if is_active {
                    theme.palette().primary
                } else {
                    theme.extended_palette().background.weak.color
                }
                .scale_alpha(opacity)
                .into(),
            ),
            border: Border {
                width: 0.0,
                radius: 32.0.into(),
                color: Color::TRANSPARENT,
            },
            text_color: if is_active {
                theme.extended_palette().primary.base.text
            } else {
                theme.palette().text
            },
            ..button::Style::default()
        };
        match status {
            Status::Active => base,
            Status::Hovered => {
                let peach = theme.extended_palette().primary.weak.color;
                base.background = Some(
                    if is_active {
                        peach
                    } else {
                        theme.extended_palette().background.strong.color
                    }
                    .scale_alpha(opacity)
                    .into(),
                );
                base
            }
            _ => base,
        }
    }
}

pub fn quick_settings_submenu_button_style(
    is_active: bool,
    opacity: f32,
) -> impl Fn(&Theme, Status) -> button::Style {
    move |theme: &Theme, status: Status| {
        let mut base = button::Style {
            background: None,
            border: Border {
                width: 0.0,
                radius: 16.0.into(),
                color: Color::TRANSPARENT,
            },
            text_color: if is_active {
                theme.extended_palette().primary.base.text
            } else {
                theme.palette().text
            },
            ..button::Style::default()
        };
        match status {
            Status::Active => base,
            Status::Hovered => {
                base.background = Some(
                    theme
                        .extended_palette()
                        .background
                        .weak
                        .color
                        .scale_alpha(opacity)
                        .into(),
                );
                base.text_color = theme.palette().text;
                base
            }
            _ => base,
        }
    }
}

pub fn text_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let mut base = text_input::Style {
        background: theme.palette().background.into(),
        border: Border {
            width: 2.0,
            radius: 32.0.into(),
            color: theme.extended_palette().background.weak.color,
        },
        icon: theme.palette().text,
        placeholder: theme.palette().text,
        value: theme.palette().text,
        selection: theme.palette().primary,
    };
    match status {
        text_input::Status::Active => base,
        text_input::Status::Focused | text_input::Status::Hovered => {
            base.border.color = theme.extended_palette().background.strong.color;
            base
        }
        text_input::Status::Disabled => {
            base.background = theme.extended_palette().background.weak.color.into();
            base.border.color = Color::TRANSPARENT;
            base
        }
    }
}

pub fn backdrop_color(backdrop: f32) -> Color {
    Color::from_rgba(0.0, 0.0, 0.0, backdrop)
}

pub fn darken_color(color: Color, darkening_alpha: f32) -> Color {
    let new_r = color.r * (1.0 - darkening_alpha);
    let new_g = color.g * (1.0 - darkening_alpha);
    let new_b = color.b * (1.0 - darkening_alpha);
    let new_a = color.a + (1.0 - color.a) * darkening_alpha;

    Color::from([new_r, new_g, new_b, new_a])
}
