use std::cell::RefCell;

use crate::{
    components::button::{ButtonHierarchy, ButtonKind},
    config::{
        Appearance, AppearanceColor, BackgroundLevel, BarAppearance, BarMargin, BarRadius,
        BarSurface, MenuAppearance, Position, RadiusSize, SpaceSize, Surface,
    },
};
use iced::{
    Background, Border, Color, Theme, border,
    theme::{Palette, palette},
    widget::{
        button::{self, Status},
        progress_bar, rule, scrollable, slider,
        text_input::{self},
        toggler,
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

impl Space {
    pub fn resolve(&self, size: SpaceSize) -> f32 {
        match size {
            SpaceSize::None => 0.0,
            SpaceSize::Xxs => self.xxs,
            SpaceSize::Xs => self.xs,
            SpaceSize::Sm => self.sm,
            SpaceSize::Md => self.md,
            SpaceSize::Lg => self.lg,
            SpaceSize::Xl => self.xl,
            SpaceSize::Xxl => self.xxl,
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

impl Radius {
    pub fn resolve(&self, size: RadiusSize) -> f32 {
        match size {
            RadiusSize::None => 0.0,
            RadiusSize::Sm => self.sm,
            RadiusSize::Md => self.md,
            RadiusSize::Lg => self.lg,
            RadiusSize::Xl => self.xl,
        }
    }
}

/// Bar geometry the layer-surface layer needs: which surface mode drives the
/// bar height, plus the resolved outer margin in logical (unscaled) pixels
/// ordered `(top, right, bottom, left)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarLayout {
    pub surface: BarSurface,
    pub margin: (f32, f32, f32, f32),
}

impl BarLayout {
    pub fn from_appearance(bar: &BarAppearance) -> Self {
        Self::new(bar.surface, bar.margin)
    }

    fn new(surface: BarSurface, margin: BarMargin) -> Self {
        let space = Space::default();
        Self {
            surface,
            margin: (
                space.resolve(margin.top),
                space.resolve(margin.right),
                space.resolve(margin.bottom),
                space.resolve(margin.left),
            ),
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

/// Everything that varies from one surface to the next.
#[derive(Debug, Clone)]
pub struct SurfaceTheme {
    pub iced_theme: Theme,
    pub blur: bool,
}

#[derive(Debug, Clone)]
struct SurfaceThemes {
    bar: SurfaceTheme,
    menu: SurfaceTheme,
    osd: SurfaceTheme,
    notifications: SurfaceTheme,
}

impl SurfaceThemes {
    fn new(appearance: &Appearance) -> Self {
        let for_surface = |surface| {
            let opacity = appearance.opacity.get(surface);

            SurfaceTheme {
                iced_theme: build_iced_theme(appearance, opacity),
                blur: appearance.blur.enabled(opacity),
            }
        };

        Self {
            bar: for_surface(Surface::Bar),
            menu: for_surface(Surface::Menu),
            osd: for_surface(Surface::Osd),
            notifications: for_surface(Surface::Notifications),
        }
    }

    fn get(&self, surface: Surface) -> &SurfaceTheme {
        match surface {
            Surface::Bar => &self.bar,
            Surface::Menu => &self.menu,
            Surface::Osd => &self.osd,
            Surface::Notifications => &self.notifications,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AshellTheme {
    surfaces: SurfaceThemes,
    /// For call sites that read a colour where there is no `&Theme` to hand.
    pub palette: Palette,
    pub space: Space,
    pub radius: Radius,
    pub font_size: FontSize,
    pub bar_position: Position,
    pub bar_surface: BarSurface,
    pub bar_radius: BarRadius,
    pub bar_margin: BarMargin,
    pub menu: MenuAppearance,
    pub workspace_colors: Vec<AppearanceColor>,
    pub special_workspace_colors: Option<Vec<AppearanceColor>>,
    pub scale_factor: f64,
    // Read by animation call sites added in subsequent PRs.
    #[allow(dead_code)]
    pub animations_enabled: bool,
}

impl Default for AshellTheme {
    fn default() -> Self {
        let appearance = Appearance::default();
        base_theme_from_appearance(&appearance, Position::default(), false)
    }
}

/// Not scaled by the configured opacity: the colour underneath already carries
/// it, and scaling here would paint that background twice.
const HOVER_OVERLAY: f32 = 0.04;

/// Straight-alpha "a over b", so only one layer ends up painted.
fn over(a: Color, b: Color) -> Color {
    let alpha = a.a + b.a * (1.0 - a.a);
    if alpha <= f32::EPSILON {
        return Color::TRANSPARENT;
    }
    let channel = |ca: f32, cb: f32| (ca * a.a + cb * b.a * (1.0 - a.a)) / alpha;
    Color {
        r: channel(a.r, b.r),
        g: channel(a.g, b.g),
        b: channel(a.b, b.b),
        a: alpha,
    }
}

/// Alphas for marks drawn *on* a surface. Fixed ratios of the foreground, as
/// in Adwaita, so they read the same whatever opacity the surface runs at.
const TROUGH_ALPHA: f32 = 0.15;
const THUMB_ALPHA: f32 = 0.4;
const DIVIDER_ALPHA: f32 = 0.12;

fn trough(theme: &Theme) -> Color {
    theme.palette().text.scale_alpha(TROUGH_ALPHA)
}

pub fn slider_style(theme: &Theme, status: slider::Status) -> slider::Style {
    let mut style = slider::default(theme, status);
    style.rail.backgrounds.1 = trough(theme).into();
    style
}

pub fn scrollable_style(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let mut style = scrollable::default(theme, status);
    for rail in [&mut style.vertical_rail, &mut style.horizontal_rail] {
        rail.background = Some(Background::Color(Color::TRANSPARENT));
        rail.scroller.background = theme.palette().text.scale_alpha(THUMB_ALPHA).into();
    }
    style
}

pub fn toggler_style(theme: &Theme, status: toggler::Status) -> toggler::Style {
    let mut style = toggler::default(theme, status);
    let toggled = matches!(
        status,
        toggler::Status::Active { is_toggled: true }
            | toggler::Status::Hovered { is_toggled: true }
    );
    if !toggled {
        style.background = trough(theme).into();
    }
    style
}

pub fn rule_style(theme: &Theme) -> rule::Style {
    rule::Style {
        color: theme.palette().text.scale_alpha(DIVIDER_ALPHA),
        ..rule::default(theme)
    }
}

pub fn progress_bar_style(
    base: fn(&Theme) -> progress_bar::Style,
) -> impl Fn(&Theme) -> progress_bar::Style {
    move |theme| progress_bar::Style {
        background: trough(theme).into(),
        ..base(theme)
    }
}

/// The opacity of the surface this theme paints. Lives on the base palette's
/// background because that is the only colour there that is paint.
pub fn surface_opacity(theme: &Theme) -> f32 {
    theme.palette().background.a
}

/// `color` painted as part of the surface, so it carries the surface opacity.
/// Anything drawn *on* the surface stays opaque or picks its own fixed alpha.
pub fn as_surface(theme: &Theme, color: Color) -> Color {
    color.scale_alpha(surface_opacity(theme))
}

/// `color` with the hover overlay composited in, so hovering paints one layer
/// rather than stacking a second one and washing out the translucency.
pub fn hovered(theme: &Theme, color: Color) -> Color {
    over(theme.palette().text.scale_alpha(HOVER_OVERLAY), color)
}

fn base_palette(appearance: &Appearance) -> Palette {
    Palette {
        background: appearance.background_color.get_base(),
        text: appearance.text_color.get_base(),
        primary: appearance.primary_color.get_base(),
        success: appearance.success_color.get_base(),
        warning: appearance.warning_color.get_base(),
        danger: appearance.danger_color.get_base(),
    }
}

fn build_iced_theme(appearance: &Appearance, opacity: f32) -> Theme {
    let base = base_palette(appearance);

    Theme::custom_with_fn(
        "local".to_string(),
        Palette {
            // The one colour here that is paint; the accents are read as ink.
            background: base.background.scale_alpha(opacity),
            ..base
        },
        |palette| {
            let text = palette.text;
            let bg_text = appearance.background_color.get_text().unwrap_or(text);
            // `mix` interpolates alpha too, so deriving from the translucent
            // colour would spread assorted alphas across the variants.
            let background = Color {
                a: 1.0,
                ..palette.background
            };

            let default_bg = palette::Background::new(background, bg_text);
            let bg = |level, fallback| {
                appearance
                    .background_color
                    .get_pair(level, text)
                    .unwrap_or(fallback)
            };

            let default_primary = palette::Primary::generate(
                palette.primary,
                background,
                appearance.primary_color.get_text().unwrap_or(text),
            );
            let default_success = palette::Success::generate(
                palette.success,
                background,
                appearance.success_color.get_text().unwrap_or(text),
            );
            let default_warning = palette::Warning::generate(
                palette.warning,
                background,
                appearance.warning_color.get_text().unwrap_or(text),
            );
            let default_danger = palette::Danger::generate(
                palette.danger,
                background,
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
                secondary: palette::Secondary::generate(background, text),
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
    )
}

fn base_theme_from_appearance(
    appearance: &Appearance,
    bar_position: Position,
    animations_enabled: bool,
) -> AshellTheme {
    AshellTheme {
        space: Space::default(),
        radius: Radius::default(),
        font_size: FontSize::default(),
        bar_position,
        bar_surface: appearance.bar.surface,
        bar_radius: appearance.bar.radius,
        bar_margin: appearance.bar.margin,
        menu: appearance.menu,
        workspace_colors: appearance.workspace_colors.clone(),
        special_workspace_colors: appearance.special_workspace_colors.clone(),
        scale_factor: appearance.scale_factor,
        animations_enabled,
        palette: base_palette(appearance),
        surfaces: SurfaceThemes::new(appearance),
    }
}

impl AshellTheme {
    pub fn new(
        position: Position,
        appearance: &Appearance,
        animations: &crate::config::AnimationsConfig,
    ) -> Self {
        base_theme_from_appearance(appearance, position, animations.enabled)
    }

    pub fn surface(&self, surface: Surface) -> &SurfaceTheme {
        self.surfaces.get(surface)
    }

    pub fn bar_layout(&self) -> BarLayout {
        BarLayout::new(self.bar_surface, self.bar_margin)
    }

    pub fn bar_border_radius(&self) -> border::Radius {
        border::Radius {
            top_left: self.radius.resolve(self.bar_radius.top_left),
            top_right: self.radius.resolve(self.bar_radius.top_right),
            bottom_right: self.radius.resolve(self.bar_radius.bottom_right),
            bottom_left: self.radius.resolve(self.bar_radius.bottom_left),
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
                    as_surface(theme, ext.background.weak.color),
                    as_surface(theme, ext.background.strong.color),
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
                    background: Some(base_bg.into()),
                    border: Border {
                        width: 0.0,
                        radius: radius.into(),
                        color: Color::TRANSPARENT,
                    },
                    text_color: base_text,
                    ..button::Style::default()
                },
                (ButtonKind::Solid, Status::Hovered) => button::Style {
                    background: Some(hover_bg.into()),
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
                // Transparent at rest, so hover adds an overlay, not a background.
                (ButtonKind::Outline, Status::Hovered) => button::Style {
                    background: Some(palette.text.scale_alpha(HOVER_OVERLAY).into()),
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
                            background: Some(base_bg.scale_alpha(disabled_opacity).into()),
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
        active: f32,
    ) -> impl Fn(&Theme, Status) -> button::Style + use<> {
        let radius_lg = self.radius.lg;
        move |theme: &Theme, status: Status| {
            let mut base = button::Style {
                background: None,
                border: Border {
                    width: 0.0,
                    radius: radius_lg.into(),
                    color: Color::TRANSPARENT,
                },
                text_color: lerp_color(
                    theme.palette().text,
                    theme.extended_palette().primary.base.text,
                    active,
                ),
                ..button::Style::default()
            };
            match status {
                Status::Active => base,
                // Transparent at rest, so hover adds an overlay, not a background.
                Status::Hovered => {
                    base.background = Some(theme.palette().text.scale_alpha(HOVER_OVERLAY).into());
                    base.text_color = theme.palette().text;
                    base
                }
                _ => base,
            }
        }
    }

    pub fn quick_settings_button_style(
        &self,
        active: f32,
    ) -> impl Fn(&Theme, Status) -> button::Style + use<> {
        let radius = self.radius.xl;
        move |theme: &Theme, status: Status| {
            let inactive_bg = as_surface(theme, theme.extended_palette().background.weak.color);
            let active_bg = theme.extended_palette().primary.base.color;
            let bg = lerp_color(inactive_bg, active_bg, active);

            let mut base = button::Style {
                background: Some(bg.into()),
                border: Border {
                    width: 0.0,
                    radius: radius.into(),
                    color: Color::TRANSPARENT,
                },
                text_color: lerp_color(
                    theme.palette().text,
                    theme.extended_palette().primary.base.text,
                    active,
                ),
                ..button::Style::default()
            };
            match status {
                Status::Active => base,
                Status::Hovered => {
                    let inactive_hover =
                        as_surface(theme, theme.extended_palette().background.strong.color);
                    let active_hover = theme.extended_palette().primary.weak.color;
                    base.background = Some(lerp_color(inactive_hover, active_hover, active).into());
                    base
                }
                _ => base,
            }
        }
    }

    pub fn workspace_button_style(
        &self,
        is_empty: bool,
        is_urgent: bool,
        is_active: bool,
        colors: Option<Option<AppearanceColor>>,
    ) -> impl Fn(&Theme, Status) -> button::Style + use<> {
        let radius_lg = self.radius.lg;
        move |theme: &Theme, status: Status| {
            let fill = |color: Color| Background::Color(as_surface(theme, color));
            let mark = Background::Color;
            let primary = colors.map(|c| {
                c.map_or_else(
                    || theme.extended_palette().primary,
                    |c| {
                        palette::Primary::generate(
                            c.get_base(),
                            theme.palette().background,
                            c.get_text().unwrap_or_else(|| theme.palette().text),
                        )
                    },
                )
            });
            let resolve = |bg: fn(&palette::Background) -> palette::Pair,
                           pr: fn(&palette::Primary) -> palette::Pair| {
                match primary {
                    Some(p) => {
                        let pair = pr(&p);
                        (pair.color, pair.text)
                    }
                    None => {
                        let pair = bg(&theme.extended_palette().background);
                        (pair.color, theme.palette().text)
                    }
                }
            };
            let (bg_color, fg_color) = resolve(|b| b.weak, |p| p.base);
            let (bg_strong, fg_strong) = resolve(|b| b.strong, |p| p.strong);
            let (bg_weak, fg_weak) = resolve(|b| b.weak, |p| p.weak);
            let mut base = button::Style {
                background: Some(if is_urgent && is_empty {
                    mark(theme.extended_palette().danger.base.color)
                } else if is_empty && is_active {
                    mark(theme.extended_palette().background.strong.color)
                } else if is_empty {
                    fill(theme.extended_palette().background.weak.color)
                } else if is_active {
                    mark(bg_color)
                } else {
                    fill(bg_weak)
                }),
                border: Border {
                    width: if is_urgent || is_empty { 1.0 } else { 0.0 },
                    color: if is_urgent {
                        theme.extended_palette().danger.base.color
                    } else if is_active {
                        bg_color
                    } else {
                        bg_weak
                    },
                    radius: radius_lg.into(),
                },
                text_color: if is_urgent && is_empty {
                    theme.extended_palette().danger.base.text
                } else if is_empty && is_active {
                    theme.extended_palette().background.strong.text
                } else if is_empty {
                    theme.extended_palette().background.weak.text
                } else if is_active {
                    fg_color
                } else {
                    fg_weak
                },
                ..button::Style::default()
            };
            match status {
                Status::Active => base,
                Status::Hovered => {
                    base.background = Some(if is_urgent && is_empty {
                        mark(theme.extended_palette().danger.strong.color)
                    } else if is_empty {
                        fill(theme.extended_palette().background.strong.color)
                    } else if is_active {
                        mark(bg_color)
                    } else {
                        fill(bg_strong)
                    });
                    base.border.color = if is_urgent && is_active {
                        theme.extended_palette().danger.base.color
                    } else if is_urgent {
                        theme.extended_palette().danger.strong.color
                    } else if is_active {
                        bg_color
                    } else {
                        bg_strong
                    };
                    base.text_color = if is_urgent && is_empty {
                        theme.extended_palette().danger.strong.text
                    } else if is_empty {
                        theme.extended_palette().background.strong.text
                    } else if is_active {
                        fg_color
                    } else {
                        fg_strong
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
                    base.background =
                        as_surface(theme, theme.extended_palette().background.weak.color).into();
                    base.border.color = Color::TRANSPARENT;
                    base
                }
            }
        }
    }

    /// Module button style: transparent base with hover highlight.
    /// The module-group background is handled by `module_group`, not the button.
    pub fn module_button_style(&self) -> impl Fn(&Theme, Status) -> button::Style + use<> {
        let radius_lg = self.radius.lg;
        move |theme, status| {
            let mut base = button::Style {
                background: None,
                border: Border {
                    width: 0.0,
                    radius: radius_lg.into(),
                    color: Color::TRANSPARENT,
                },
                text_color: theme.palette().text,
                ..button::Style::default()
            };
            match status {
                Status::Active => base,
                // The group pill already carries the opacity; overlay on it.
                Status::Hovered => {
                    base.background = Some(theme.palette().text.scale_alpha(HOVER_OVERLAY).into());
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

pub fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    Color {
        r: from.r + (to.r - from.r) * t,
        g: from.g + (to.g - from.g) * t,
        b: from.b + (to.b - from.b) * t,
        a: from.a + (to.a - from.a) * t,
    }
}
