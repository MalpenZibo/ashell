use std::cell::RefCell;

use crate::{
    components::button::{ButtonHierarchy, ButtonKind},
    config::{
        Appearance, AppearanceColor, AppearanceStyle, BackgroundAppearanceColor, BackgroundLevel,
        MenuAppearance, ModuleAppearance, PopupAppearance, PopupStyleKey, Position,
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

thread_local! {
    pub static THEME: RefCell<AshellTheme> =  RefCell::new(AshellTheme::default());
}

pub fn init_theme(theme: AshellTheme) {
    THEME.replace(theme);
}

pub fn use_theme<R, F: FnOnce(&AshellTheme) -> R>(f: F) -> R {
    THEME.with_borrow(f)
}

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

#[derive(Debug, Clone, Copy)]
pub struct Radius {
    pub sm: f32,
    #[allow(dead_code)]
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

#[derive(Debug, Copy, Clone)]
pub struct FontSize {
    #[allow(dead_code)]
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
    pub bar_style: AppearanceStyle,
    pub opacity: f32,
    pub menu: MenuAppearance,
    pub workspace_colors: Vec<AppearanceColor>,
    pub special_workspace_colors: Option<Vec<AppearanceColor>>,
    pub scale_factor: f64,
    // Read by animation call sites added in subsequent PRs.
    #[allow(dead_code)]
    pub animations_enabled: bool,
    /// Per-module appearance overrides.
    pub module_styles: std::collections::HashMap<crate::config::ModuleName, ModuleAppearance>,
    /// Per-popup appearance overrides.
    pub popup_styles: std::collections::HashMap<PopupStyleKey, PopupAppearance>,
}

impl Default for AshellTheme {
    fn default() -> Self {
        Self::new(
            Position::default(),
            &Appearance::default(),
            &crate::config::AnimationsConfig::default(),
        )
    }
}

impl AshellTheme {
    pub fn new(
        position: Position,
        appearance: &Appearance,
        animations: &crate::config::AnimationsConfig,
    ) -> Self {
        AshellTheme {
            animations_enabled: animations.enabled,
            module_styles: appearance.module_styles.clone(),
            popup_styles: appearance.popup_styles.clone(),
            space: Space::default(),
            radius: Radius::default(),
            font_size: FontSize::default(),
            bar_position: position,
            bar_style: appearance.style,
            opacity: appearance.opacity,
            menu: appearance.menu,
            workspace_colors: appearance.workspace_colors.clone(),
            special_workspace_colors: appearance.special_workspace_colors.clone(),
            scale_factor: appearance.scale_factor,
            iced_theme: Theme::custom_with_fn(
                "local".to_string(),
                Palette {
                    background: appearance.background_color.get_base(),
                    text: appearance.text_color.get_base(),
                    primary: appearance.primary_color.get_base(),
                    success: appearance.success_color.get_base(),
                    warning: appearance.warning_color.get_base(),
                    danger: appearance.danger_color.get_base(),
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
                        appearance.primary_color.get_text().unwrap_or(text),
                    );
                    let default_success = palette::Success::generate(
                        palette.success,
                        palette.background,
                        appearance.success_color.get_text().unwrap_or(text),
                    );
                    let default_warning = palette::Warning::generate(
                        palette.warning,
                        palette.background,
                        appearance.warning_color.get_text().unwrap_or(text),
                    );
                    let default_danger = palette::Danger::generate(
                        palette.danger,
                        palette.background,
                        appearance.danger_color.get_text().unwrap_or(text),
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
                                .get_weak_pair(text)
                                .unwrap_or(default_primary.weak),
                            strong: appearance
                                .primary_color
                                .get_strong_pair(text)
                                .unwrap_or(default_primary.strong),
                        },
                        secondary: palette::Secondary::generate(palette.background, text),
                        success: palette::Success {
                            base: default_success.base,
                            weak: appearance
                                .success_color
                                .get_weak_pair(text)
                                .unwrap_or(default_success.weak),
                            strong: appearance
                                .success_color
                                .get_strong_pair(text)
                                .unwrap_or(default_success.strong),
                        },
                        warning: palette::Warning {
                            base: default_warning.base,
                            weak: appearance
                                .warning_color
                                .get_weak_pair(text)
                                .unwrap_or(default_warning.weak),
                            strong: appearance
                                .warning_color
                                .get_strong_pair(text)
                                .unwrap_or(default_warning.strong),
                        },
                        danger: palette::Danger {
                            base: default_danger.base,
                            weak: appearance
                                .danger_color
                                .get_weak_pair(text)
                                .unwrap_or(default_danger.weak),
                            strong: appearance
                                .danger_color
                                .get_strong_pair(text)
                                .unwrap_or(default_danger.strong),
                        },
                        is_dark: true,
                    }
                },
            ),
        }
    }


    /// Get the effective opacity for a module, considering per-module overrides.
    pub fn module_opacity(&self, module_name: &crate::config::ModuleName) -> f32 {
        self.module_styles
            .get(module_name)
            .and_then(|s| s.opacity)
            .unwrap_or(self.opacity)
    }

    /// Get the effective background color for a module, considering per-module overrides.
    pub fn module_background_color(
        &self,
        module_name: &crate::config::ModuleName,
    ) -> Option<&BackgroundAppearanceColor> {
        self.module_styles
            .get(module_name)
            .and_then(|s| s.background_color.as_ref())
    }

    /// Get the effective text color for a module, considering per-module overrides.
    pub fn module_text_color(
        &self,
        module_name: &crate::config::ModuleName,
    ) -> Option<&AppearanceColor> {
        self.module_styles
            .get(module_name)
            .and_then(|s| s.text_color.as_ref())
    }

    /// Get the effective hover background color for a module.
    ///
    /// Resolution order:
    /// 1. Explicit `hover_background_color` in the module's style config.
    /// 2. `weak` level of the module's `background_color` (if it's a
    ///    `BackgroundAppearanceColor::Complete` with a `weak` field).
    /// 3. Falls back to `None` (caller should use the global palette).
    pub fn module_hover_background_color(
        &self,
        module_name: &crate::config::ModuleName,
    ) -> Option<Color> {
        let style = self.module_styles.get(module_name)?;
        // 1. Explicit hover background colour
        if let Some(ref hover_bg) = style.hover_background_color {
            return Some(hover_bg.get_base());
        }
        // 2. Derive from the module's background colour
        if let Some(ref bg) = style.background_color {
            // Try the `weak` level first (gives a slightly lighter shade)
            if let Some(weak_pair) = bg.get_pair(BackgroundLevel::Weak, Color::TRANSPARENT) {
                return Some(weak_pair.color);
            }
            // For Simple colours, brighten the base colour by ~15 %
            let base = bg.get_base();
            return Some(brighten_color(base, 0.15));
        }
        None
    }

    /// Get the effective border radius for a module, considering per-module overrides.
    pub fn module_border_radius(
        &self,
        module_name: &crate::config::ModuleName,
    ) -> Option<f32> {
        self.module_styles
            .get(module_name)
            .and_then(|s| s.border_radius)
    }

    /// Get the effective popup appearance for a popup type, considering per-popup overrides.
    /// Returns (opacity, backdrop, border_radius, background_color, width).
    pub fn popup_appearance(
        &self,
        popup_key: &PopupStyleKey,
    ) -> ResolvedPopupAppearance {
        let global = &self.menu;
        let override_style = self.popup_styles.get(popup_key);

        ResolvedPopupAppearance {
            opacity: override_style.and_then(|s| s.opacity).unwrap_or(global.opacity),
            backdrop: override_style.and_then(|s| s.backdrop).unwrap_or(global.backdrop),
            border_radius: override_style.and_then(|s| s.border_radius).unwrap_or(self.radius.lg),
            background_color: override_style.and_then(|s| s.background_color),
            width: override_style.and_then(|s| s.width.map(|w| w.size())),
        }
    }
    pub fn button_style(
        &self,
        kind: ButtonKind,
        hierarchy: ButtonHierarchy,
    ) -> impl Fn(&Theme, Status) -> button::Style + use<> {
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
    ) -> impl Fn(&Theme, Status) -> button::Style + use<> {
        let radius_lg = self.radius.lg;
        let opacity = self.opacity;
        move |theme: &Theme, status: Status| {
            let mut base = button::Style {
                background: None,
                border: Border {
                    width: 0.0,
                    radius: radius_lg.into(),
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

    pub fn quick_settings_button_style(
        &self,
        is_active: bool,
    ) -> impl Fn(&Theme, Status) -> button::Style + use<> {
        let radius_xl = self.radius.xl;
        let opacity = self.opacity;
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
                    radius: radius_xl.into(),
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

    pub fn workspace_button_style(
        &self,
        is_empty: bool,
        colors: Option<Option<AppearanceColor>>,
    ) -> impl Fn(&Theme, Status) -> button::Style + use<> {
        let radius_lg = self.radius.lg;
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
                                c.get_base(),
                                theme.palette().background,
                                c.get_text().unwrap_or_else(|| theme.palette().text),
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
                    radius: radius_lg.into(),
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
                                        c.get_base(),
                                        theme.palette().background,
                                        c.get_text().unwrap_or_else(|| theme.palette().text),
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

    pub fn text_input_style(
        &self,
    ) -> impl Fn(&Theme, text_input::Status) -> text_input::Style + use<> {
        let radius_xl = self.radius.xl;
        move |theme: &Theme, status: text_input::Status| {
            let mut base = text_input::Style {
                background: theme.palette().background.into(),
                border: Border {
                    width: 2.0,
                    radius: radius_xl.into(),
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
    ///
    /// When a `module_name` is provided, per-module opacity, text color and
    /// hover background overrides are applied so the button matches the
    /// module's appearance.
    pub fn module_button_style(&self, module_name: Option<&crate::config::ModuleName>) -> impl Fn(&Theme, Status) -> button::Style + use<> {
        let radius_lg = self.radius.lg;
        let opacity = module_name
            .map(|name| self.module_opacity(name))
            .unwrap_or(self.opacity);
        let text_color_override = module_name.and_then(|name| self.module_text_color(name).map(|c| c.get_base()));
        let hover_bg_override = module_name.and_then(|name| self.module_hover_background_color(name));
        move |theme, status| {
            let text_color = text_color_override.unwrap_or_else(|| theme.palette().text);
            let mut base = button::Style {
                background: None,
                border: Border {
                    width: 0.0,
                    radius: radius_lg.into(),
                    color: Color::TRANSPARENT,
                },
                text_color,
                ..button::Style::default()
            };
            match status {
                Status::Active => base,
                Status::Hovered => {
                    let hover_color = hover_bg_override
                        .unwrap_or_else(|| theme.extended_palette().background.weak.color);
                    base.background = Some(hover_color.scale_alpha(opacity).into());
                    base
                }
                _ => base,
            }
        }
    }

    /// Notification action button style — follows the project design:
    /// rounded corners, subtle background, border that matches the
    /// notification card aesthetic, and a brighter hover state.
    ///
    /// Note: due to iced's lifetime constraints, action buttons in the
    /// notification module currently create this style inline. This method
    /// is kept for documentation/reference and future use.
    #[allow(dead_code)]
    pub fn notification_action_button_style(
        &self,
    ) -> impl Fn(&Theme, Status) -> button::Style + use<> {
        let radius = self.radius.lg;
        let opacity = self.menu.opacity;
        move |theme: &Theme, status: Status| {
            let ext = theme.extended_palette();
            match status {
                Status::Active => button::Style {
                    background: Some(ext.background.weak.color.scale_alpha(opacity).into()),
                    border: Border {
                        width: 1.0,
                        radius: radius.into(),
                        color: ext.background.strong.color.scale_alpha(0.5),
                    },
                    text_color: theme.palette().text,
                    ..button::Style::default()
                },
                Status::Hovered => button::Style {
                    background: Some(ext.background.strong.color.scale_alpha(opacity).into()),
                    border: Border {
                        width: 1.0,
                        radius: radius.into(),
                        color: ext.background.strong.color,
                    },
                    text_color: theme.palette().text,
                    ..button::Style::default()
                },
                _ => button::Style {
                    background: Some(ext.background.weak.color.scale_alpha(opacity).into()),
                    border: Border {
                        width: 1.0,
                        radius: radius.into(),
                        color: ext.background.strong.color.scale_alpha(0.5),
                    },
                    text_color: theme.palette().text,
                    ..button::Style::default()
                },
            }
        }
    }
}

/// Resolved popup appearance values, with per-popup overrides already
/// merged into the global defaults.
#[derive(Debug, Clone)]
pub struct ResolvedPopupAppearance {
    pub opacity: f32,
    pub backdrop: f32,
    pub border_radius: f32,
    pub background_color: Option<BackgroundAppearanceColor>,
    #[allow(dead_code)]
    pub width: Option<f32>,
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

/// Brighten a colour by blending it towards white.
/// `amount` is in `[0, 1]` — 0 leaves the colour unchanged, 1 makes it white.
pub fn brighten_color(color: Color, amount: f32) -> Color {
    let new_r = color.r + (1.0 - color.r) * amount;
    let new_g = color.g + (1.0 - color.g) * amount;
    let new_b = color.b + (1.0 - color.b) * amount;
    let new_a = color.a + (1.0 - color.a) * amount;

    Color::from([new_r, new_g, new_b, new_a])
}
