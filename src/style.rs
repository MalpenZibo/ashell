use crate::config::{Appearance, AppearanceColor};
use iced::{
    Background, Border, Color, Theme,
    border::Radius,
    theme::{Palette, palette},
    widget::{
        button::{self, Status},
        container,
        text_input::{self},
    },
};

pub fn ashell_theme(appearance: &Appearance) -> Theme {
    Theme::custom_with_fn(
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
    )
}

pub fn module_label(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style {
        background: Some(palette.background.into()),
        border: Border {
            width: 0.0,
            radius: 12.0.into(),
            color: Color::TRANSPARENT,
        },
        text_color: Some(palette.text),
        ..Default::default()
    }
}

pub fn module_first_label(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style {
        background: Some(palette.background.into()),
        border: Border {
            width: 0.0,
            radius: Radius::default().left(12),
            color: Color::TRANSPARENT,
        },
        text_color: Some(palette.text),
        ..Default::default()
    }
}

pub fn module_middle_label(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style {
        background: Some(palette.background.into()),
        border: Border {
            width: 0.0,
            radius: Radius::default(),
            color: Color::TRANSPARENT,
        },
        text_color: Some(palette.text),
        ..Default::default()
    }
}

pub fn module_last_label(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style {
        background: Some(palette.background.into()),
        border: Border {
            width: 0.0,
            radius: Radius::default().right(12),
            color: Color::TRANSPARENT,
        },
        text_color: Some(palette.text),
        ..Default::default()
    }
}

pub enum ModuleButtonStyle {
    Full,
    First,
    Middle,
    Last,
}

impl ModuleButtonStyle {
    pub fn into_style<'a>(self) -> button::StyleFn<'a, Theme> {
        Box::new(move |theme, status| {
            let mut base = button::Style {
                background: Some(theme.palette().background.into()),
                border: Border {
                    width: 0.0,
                    radius: match self {
                        ModuleButtonStyle::Full => 12.0.into(),
                        ModuleButtonStyle::First => Radius::default().left(12),
                        ModuleButtonStyle::Middle => Radius::default(),
                        ModuleButtonStyle::Last => Radius::default().right(12),
                    },
                    color: Color::TRANSPARENT,
                },
                text_color: theme.palette().text,
                ..button::Style::default()
            };
            match status {
                Status::Active => base,
                Status::Hovered => {
                    base.background = Some(theme.extended_palette().background.weak.color.into());
                    base
                }
                _ => base,
            }
        })
    }
}

pub struct GhostButtonStyle;

impl GhostButtonStyle {
    pub fn into_style<'a>(self) -> button::StyleFn<'a, Theme> {
        Box::new(move |theme, status| {
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
                    base.background = Some(theme.extended_palette().background.weak.color.into());
                    base
                }
                _ => base,
            }
        })
    }
}

pub struct OutlineButtonStyle;

impl OutlineButtonStyle {
    pub fn into_style<'a>(self) -> button::StyleFn<'a, Theme> {
        Box::new(move |theme, status| {
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
                    base.background = Some(theme.extended_palette().background.weak.color.into());
                    base
                }
                _ => base,
            }
        })
    }
}

pub struct ConfirmButtonStyle;

impl ConfirmButtonStyle {
    pub fn into_style<'a>(self) -> button::StyleFn<'a, Theme> {
        Box::new(move |theme, status| {
            let mut base = button::Style {
                background: Some(theme.extended_palette().background.weak.color.into()),
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
                    base.background = Some(theme.extended_palette().background.strong.color.into());
                    base
                }
                _ => base,
            }
        })
    }
}

pub struct SettingsButtonStyle;

impl SettingsButtonStyle {
    pub fn into_style<'a>(self) -> button::StyleFn<'a, Theme> {
        Box::new(move |theme, status| {
            let mut base = button::Style {
                background: Some(theme.extended_palette().background.weak.color.into()),
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
                    base.background = Some(theme.extended_palette().background.strong.color.into());
                    base
                }
                _ => base,
            }
        })
    }
}

pub struct WorkspaceButtonStyle(pub bool, pub Option<Option<AppearanceColor>>);

impl WorkspaceButtonStyle {
    pub fn into_style<'a>(self) -> button::StyleFn<'a, Theme> {
        Box::new(move |theme, status| {
            let (bg_color, fg_color) = self
                .1
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
                background: Some(Background::Color(if self.0 {
                    theme.extended_palette().background.weak.color
                } else {
                    bg_color
                })),
                border: Border {
                    width: if self.0 { 1.0 } else { 0.0 },
                    color: bg_color,
                    radius: 16.0.into(),
                },
                text_color: if self.0 {
                    theme.extended_palette().background.weak.text
                } else {
                    fg_color
                },
                ..button::Style::default()
            };
            match status {
                Status::Active => base,
                Status::Hovered => {
                    let (bg_color, fg_color) = self
                        .1
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

                    base.background = Some(Background::Color(if self.0 {
                        theme.extended_palette().background.strong.color
                    } else {
                        bg_color
                    }));
                    base.text_color = if self.0 {
                        theme.extended_palette().background.weak.text
                    } else {
                        fg_color
                    };
                    base
                }
                _ => base,
            }
        })
    }
}

pub struct QuickSettingsButtonStyle(pub bool);

impl QuickSettingsButtonStyle {
    pub fn into_style<'a>(self) -> button::StyleFn<'a, Theme> {
        Box::new(move |theme, status| {
            let mut base = button::Style {
                background: Some(if self.0 {
                    theme.palette().primary.into()
                } else {
                    theme.extended_palette().background.weak.color.into()
                }),
                border: Border {
                    width: 0.0,
                    radius: 32.0.into(),
                    color: Color::TRANSPARENT,
                },
                text_color: if self.0 {
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
                        if self.0 {
                            peach
                        } else {
                            theme.extended_palette().background.strong.color
                        }
                        .into(),
                    );
                    base
                }
                _ => base,
            }
        })
    }
}

pub struct QuickSettingsSubMenuButtonStyle(pub bool);

impl QuickSettingsSubMenuButtonStyle {
    pub fn into_style<'a>(self) -> button::StyleFn<'a, Theme> {
        Box::new(move |theme, status| {
            let mut base = button::Style {
                background: None,
                border: Border {
                    width: 0.0,
                    radius: 16.0.into(),
                    color: Color::TRANSPARENT,
                },
                text_color: if self.0 {
                    theme.extended_palette().primary.base.text
                } else {
                    theme.palette().text
                },
                ..button::Style::default()
            };
            match status {
                Status::Active => base,
                Status::Hovered => {
                    base.background = Some(theme.extended_palette().background.weak.color.into());
                    base.text_color = theme.palette().text;
                    base
                }
                _ => base,
            }
        })
    }
}

pub struct TextInputStyle;

impl TextInputStyle {
    pub fn into_style<'a>(self) -> text_input::StyleFn<'a, Theme> {
        Box::new(move |theme, status| {
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
        })
    }
}
