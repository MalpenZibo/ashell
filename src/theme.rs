use crate::{
    components::button::{ButtonHierarchy, ButtonKind},
    config::{
        Appearance, AppearanceColor, BackgroundLevel, BarBackground, BoxSpacing, MenuAppearance,
        Position,
    },
};
use iced::{
    Background, Border, Color, Theme,
    theme::{Palette, palette},
    widget::{
        button::{self, Status},
        text_input::{self},
    },
};

#[allow(unused)]
#[derive(Debug, Copy, Clone)]
pub struct Space {
    pub xxs: f32,
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub xxl: f32,
}

impl Default for Space {
    fn default() -> Self {
        Self {
            xxs: 4.0,
            xs: 8.0,
            sm: 12.0,
            md: 16.0,
            lg: 24.0,
            xl: 32.0,
            xxl: 48.0,
        }
    }
}

#[allow(unused)]
#[derive(Debug, Clone, Copy)]
pub struct Radius {
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
}

impl Default for Radius {
    fn default() -> Self {
        Self {
            sm: 4.0,
            md: 8.0,
            lg: 16.0,
            xl: 32.0,
        }
    }
}

#[allow(unused)]
#[derive(Debug, Copy, Clone)]
pub struct FontSize {
    pub xxs: f32,
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub xxl: f32,
}

impl Default for FontSize {
    fn default() -> Self {
        Self {
            xxs: 8.0,
            xs: 10.0,
            sm: 12.0,
            md: 16.0,
            lg: 20.0,
            xl: 22.0,
            xxl: 32.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AshellTheme {
    pub iced_theme: Theme,
    pub space: Space,
    pub radius: Radius,
    pub font_size: FontSize,
    pub bar_position: Position,
    pub margin: BoxSpacing,
    pub padding: BoxSpacing,
    pub background: BarBackground,
    pub opacity: f32,
    pub menu: MenuAppearance,
    pub workspace_colors: Vec<AppearanceColor>,
    pub special_workspace_colors: Option<Vec<AppearanceColor>>,
    pub scale_factor: f64,
}

const DEFAULT_TEXT_SIZE: f32 = 14.0;

/// Default line height multiplier in iced (1.3).
const DEFAULT_LINE_HEIGHT: f32 = 1.3;

const MODULE_VERTICAL_PADDING: f32 = 2.0;

impl AshellTheme {
    pub fn bar_height(&self) -> f32 {
        (DEFAULT_TEXT_SIZE * DEFAULT_LINE_HEIGHT).ceil()
            + MODULE_VERTICAL_PADDING * 2.0
            + self.padding.vertical()
    }

    pub fn new(position: Position, appearance: &Appearance) -> Self {
        AshellTheme {
            space: Space::default(),
            radius: Radius::default(),
            font_size: FontSize::default(),
            bar_position: position,
            margin: appearance.margin,
            padding: appearance.padding,
            background: appearance.background.clone(),
            opacity: appearance.opacity,
            menu: appearance.menu,
            workspace_colors: appearance.workspace_colors.clone(),
            special_workspace_colors: appearance.special_workspace_colors.clone(),
            scale_factor: appearance.scale_factor,
            iced_theme: Theme::custom_with_fn(
                "local".to_string(),
                Palette {
                    background: appearance.background_color.get_base(),
                    text: (&appearance.text_color).into(),
                    primary: (&appearance.primary_color).into(),
                    success: (&appearance.success_color).into(),
                    warning: (&appearance.warning_color).into(),
                    danger: (&appearance.danger_color).into(),
                },
                |palette| {
                    let text = palette.text;
                    let bg_text = appearance.background_color.get_text().unwrap_or(text);

                    let default_bg = palette::Background::new(palette.background, bg_text);
                    let bg = |level, fallback| {
                        appearance
                            .background_color
                            .get_pair(level, text)
                            .unwrap_or(fallback)
                    };

                    let default_primary = palette::Primary::generate(
                        palette.primary,
                        palette.background,
                        appearance.primary_color.text().unwrap_or(text),
                    );
                    let default_success = palette::Success::generate(
                        palette.success,
                        palette.background,
                        appearance.success_color.text().unwrap_or(text),
                    );
                    let default_warning = palette::Warning::generate(
                        palette.warning,
                        palette.background,
                        appearance.warning_color.text().unwrap_or(text),
                    );
                    let default_danger = palette::Danger::generate(
                        palette.danger,
                        palette.background,
                        appearance.danger_color.text().unwrap_or(text),
                    );

                    palette::Extended {
                        background: palette::Background {
                            base: default_bg.base,
                            weakest: bg(BackgroundLevel::Weakest, default_bg.weakest),
                            weaker: bg(BackgroundLevel::Weaker, default_bg.weaker),
                            weak: bg(BackgroundLevel::Weak, default_bg.weak),
                            neutral: bg(BackgroundLevel::Neutral, default_bg.neutral),
                            strong: bg(BackgroundLevel::Strong, default_bg.strong),
                            stronger: bg(BackgroundLevel::Stronger, default_bg.stronger),
                            strongest: bg(BackgroundLevel::Strongest, default_bg.strongest),
                        },
                        primary: palette::Primary {
                            base: default_primary.base,
                            weak: appearance
                                .primary_color
                                .weak_pair(text)
                                .unwrap_or(default_primary.weak),
                            strong: appearance
                                .primary_color
                                .strong_pair(text)
                                .unwrap_or(default_primary.strong),
                        },
                        secondary: palette::Secondary::generate(palette.background, text),
                        success: palette::Success {
                            base: default_success.base,
                            weak: appearance
                                .success_color
                                .weak_pair(text)
                                .unwrap_or(default_success.weak),
                            strong: appearance
                                .success_color
                                .strong_pair(text)
                                .unwrap_or(default_success.strong),
                        },
                        warning: palette::Warning {
                            base: default_warning.base,
                            weak: appearance
                                .warning_color
                                .weak_pair(text)
                                .unwrap_or(default_warning.weak),
                            strong: appearance
                                .warning_color
                                .strong_pair(text)
                                .unwrap_or(default_warning.strong),
                        },
                        danger: palette::Danger {
                            base: default_danger.base,
                            weak: appearance
                                .danger_color
                                .weak_pair(text)
                                .unwrap_or(default_danger.weak),
                            strong: appearance
                                .danger_color
                                .strong_pair(text)
                                .unwrap_or(default_danger.strong),
                        },
                        is_dark: true,
                    }
                },
            ),
        }
    }

    pub fn get_theme(&self) -> &Theme {
        &self.iced_theme
    }

    pub fn button_style(
        &self,
        kind: ButtonKind,
        hierarchy: ButtonHierarchy,
    ) -> impl Fn(&Theme, Status) -> button::Style {
        let radius = match kind {
            ButtonKind::Transparent => self.radius.sm,
            ButtonKind::Solid | ButtonKind::Outline => self.radius.xl,
        };
        let opacity = self.opacity;

        move |theme: &Theme, status: Status| {
            let palette = theme.palette();
            let ext = theme.extended_palette();

            let (base_bg, hover_bg, base_text, hover_text, border_color) = match hierarchy {
                ButtonHierarchy::Primary => (
                    palette.primary,
                    ext.primary.weak.color,
                    ext.primary.base.text,
                    ext.primary.base.text,
                    palette.primary,
                ),
                ButtonHierarchy::Secondary => (
                    ext.background.weak.color,
                    ext.background.strong.color,
                    palette.text,
                    palette.text,
                    ext.background.weak.color,
                ),
                ButtonHierarchy::Danger => (
                    palette.danger,
                    ext.danger.weak.color,
                    ext.danger.base.text,
                    ext.danger.base.text,
                    palette.danger,
                ),
            };

            match (kind, status) {
                (ButtonKind::Solid, Status::Active) => button::Style {
                    background: Some(base_bg.scale_alpha(opacity).into()),
                    border: Border {
                        width: 0.0,
                        radius: radius.into(),
                        color: Color::TRANSPARENT,
                    },
                    text_color: base_text,
                    ..button::Style::default()
                },
                (ButtonKind::Solid, Status::Hovered) => button::Style {
                    background: Some(hover_bg.scale_alpha(opacity).into()),
                    border: Border {
                        width: 0.0,
                        radius: radius.into(),
                        color: Color::TRANSPARENT,
                    },
                    text_color: hover_text,
                    ..button::Style::default()
                },

                (ButtonKind::Transparent, Status::Active) => button::Style {
                    background: None,
                    border: Border {
                        width: 0.0,
                        radius: radius.into(),
                        color: Color::TRANSPARENT,
                    },
                    text_color: palette.text,
                    ..button::Style::default()
                },
                (ButtonKind::Transparent, Status::Hovered) => button::Style {
                    background: Some(
                        theme
                            .extended_palette()
                            .background
                            .base
                            .text
                            .scale_alpha(0.04)
                            .into(),
                    ),
                    border: Border {
                        width: 0.0,
                        radius: radius.into(),
                        color: Color::TRANSPARENT,
                    },
                    text_color: match hierarchy {
                        ButtonHierarchy::Danger => palette.danger,
                        ButtonHierarchy::Primary => palette.primary,
                        ButtonHierarchy::Secondary => palette.text,
                    },
                    ..button::Style::default()
                },

                (ButtonKind::Outline, Status::Active) => button::Style {
                    background: None,
                    border: Border {
                        width: 2.0,
                        radius: radius.into(),
                        color: border_color,
                    },
                    text_color: palette.text,
                    ..button::Style::default()
                },
                (ButtonKind::Outline, Status::Hovered) => button::Style {
                    background: Some(base_bg.scale_alpha(opacity).into()),
                    border: Border {
                        width: 2.0,
                        radius: radius.into(),
                        color: border_color,
                    },
                    text_color: palette.text,
                    ..button::Style::default()
                },

                (kind, Status::Disabled) => {
                    let disabled_opacity = 0.3;
                    match kind {
                        ButtonKind::Solid => button::Style {
                            background: Some(
                                base_bg.scale_alpha(opacity * disabled_opacity).into(),
                            ),
                            border: Border {
                                width: 0.0,
                                radius: radius.into(),
                                color: Color::TRANSPARENT,
                            },
                            text_color: base_text.scale_alpha(0.5),
                            ..button::Style::default()
                        },
                        ButtonKind::Transparent => button::Style {
                            background: None,
                            border: Border {
                                width: 0.0,
                                radius: radius.into(),
                                color: Color::TRANSPARENT,
                            },
                            text_color: palette.text.scale_alpha(disabled_opacity),
                            ..button::Style::default()
                        },
                        ButtonKind::Outline => button::Style {
                            background: None,
                            border: Border {
                                width: 2.0,
                                radius: radius.into(),
                                color: border_color.scale_alpha(disabled_opacity),
                            },
                            text_color: palette.text.scale_alpha(disabled_opacity),
                            ..button::Style::default()
                        },
                    }
                }

                _ => button::Style {
                    background: None,
                    border: Border {
                        width: 0.0,
                        radius: radius.into(),
                        color: Color::TRANSPARENT,
                    },
                    text_color: palette.text,
                    ..button::Style::default()
                },
            }
        }
    }

    pub fn quick_settings_submenu_button_style(
        &self,
        is_active: bool,
    ) -> impl Fn(&Theme, Status) -> button::Style {
        move |theme: &Theme, status: Status| {
            let mut base = button::Style {
                background: None,
                border: Border {
                    width: 0.0,
                    radius: self.radius.lg.into(),
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
                            .scale_alpha(self.opacity)
                            .into(),
                    );
                    base.text_color = theme.palette().text;
                    base
                }
                _ => base,
            }
        }
    }

    pub fn quick_settings_button_style(
        &self,
        is_active: bool,
    ) -> impl Fn(&Theme, Status) -> button::Style {
        move |theme: &Theme, status: Status| {
            let mut base = button::Style {
                background: Some(
                    if is_active {
                        theme.palette().primary
                    } else {
                        theme.extended_palette().background.weak.color
                    }
                    .scale_alpha(self.opacity)
                    .into(),
                ),
                border: Border {
                    width: 0.0,
                    radius: self.radius.xl.into(),
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
                        .scale_alpha(self.opacity)
                        .into(),
                    );
                    base
                }
                _ => base,
            }
        }
    }

    pub fn workspace_button_style(
        &self,
        is_empty: bool,
        colors: Option<Option<AppearanceColor>>,
    ) -> impl Fn(&Theme, Status) -> button::Style {
        move |theme: &Theme, status: Status| {
            let (bg_color, fg_color) = colors.map_or_else(
                || {
                    (
                        theme.extended_palette().background.weak.color,
                        theme.palette().text,
                    )
                },
                |c| {
                    c.map_or_else(
                        || {
                            (
                                theme.extended_palette().primary.base.color,
                                theme.extended_palette().primary.base.text,
                            )
                        },
                        |c| {
                            let color = palette::Primary::generate(
                                Color::from(&c),
                                theme.palette().background,
                                c.text().unwrap_or_else(|| theme.palette().text),
                            );
                            (color.base.color, color.base.text)
                        },
                    )
                },
            );
            let mut base = button::Style {
                background: Some(Background::Color(if is_empty {
                    theme.extended_palette().background.weak.color
                } else {
                    bg_color
                })),
                border: Border {
                    width: if is_empty { 1.0 } else { 0.0 },
                    color: bg_color,
                    radius: self.radius.lg.into(),
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
                    let (bg_color, fg_color) = colors.map_or_else(
                        || {
                            (
                                theme.extended_palette().background.strong.color,
                                theme.palette().text,
                            )
                        },
                        |c| {
                            c.map_or_else(
                                || {
                                    (
                                        theme.extended_palette().primary.strong.color,
                                        theme.extended_palette().primary.strong.text,
                                    )
                                },
                                |c| {
                                    let color = palette::Primary::generate(
                                        Color::from(&c),
                                        theme.palette().background,
                                        c.text().unwrap_or_else(|| theme.palette().text),
                                    );
                                    (color.strong.color, color.strong.text)
                                },
                            )
                        },
                    );

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

    pub fn text_input_style(&self) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
        move |theme: &Theme, status: text_input::Status| {
            let mut base = text_input::Style {
                background: theme.palette().background.into(),
                border: Border {
                    width: 2.0,
                    radius: self.radius.xl.into(),
                    color: theme.extended_palette().background.weak.color,
                },
                icon: theme.palette().text,
                placeholder: theme.palette().text,
                value: theme.palette().text,
                selection: theme.palette().primary,
            };
            match status {
                text_input::Status::Active => base,
                text_input::Status::Focused { .. } | text_input::Status::Hovered => {
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
    }

    /// Module button style: transparent base with hover highlight.
    /// The Islands background is handled by `module_group`, not the button.
    pub fn module_button_style(&self) -> impl Fn(&Theme, Status) -> button::Style {
        move |theme, status| {
            let mut base = button::Style {
                background: None,
                border: Border {
                    width: 0.0,
                    radius: self.radius.lg.into(),
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
                            .scale_alpha(self.opacity)
                            .into(),
                    );
                    base
                }
                _ => base,
            }
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
