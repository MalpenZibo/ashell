use super::icons::IconKind;
use crate::theme::ThemeColors;
use guido::prelude::*;

#[derive(Copy, Clone, Default, PartialEq)]
pub enum ButtonHierarchy {
    Primary,
    #[default]
    Secondary,
    #[allow(dead_code)]
    Danger,
    Custom {
        bg: Color,
        hover: Color,
        fg: Color,
    },
}

impl ButtonHierarchy {
    pub fn solid_bg(&self, theme: &ThemeColors) -> Color {
        match self {
            Self::Primary => theme.primary,
            Self::Secondary => theme.background.lighter(0.1),
            Self::Danger => theme.danger,
            Self::Custom { bg, .. } => *bg,
        }
    }

    pub fn hover_bg(&self, theme: &ThemeColors) -> Color {
        match self {
            Self::Primary => theme.primary.lighter(0.1),
            Self::Secondary => theme.background.lighter(0.2),
            Self::Danger => theme.danger.lighter(0.1),
            Self::Custom { hover, .. } => *hover,
        }
    }

    pub fn fg(&self, theme: &ThemeColors) -> Color {
        match self {
            Self::Primary => theme.background,
            Self::Secondary => theme.text,
            Self::Danger => theme.background,
            Self::Custom { fg, .. } => *fg,
        }
    }
}

#[derive(Copy, Clone, Default, PartialEq)]
pub enum ButtonSize {
    Small,
    #[default]
    Normal,
    Large,
}

#[derive(Copy, Clone)]
pub enum ButtonKind {
    Solid,
    Transparent,
}

/// Reusable button with hierarchy-based colors and optional icon
#[component]
pub fn button(
    #[prop(default = "None")] icon: Option<IconKind>,
    #[prop(slot)] content: (),
    #[prop(default = "ButtonKind::Solid")] kind: ButtonKind,
    #[prop(default = "ButtonSize::Normal")] size: ButtonSize,
    #[prop(default = "ButtonHierarchy::Secondary")] hierarchy: ButtonHierarchy,
    #[prop(default = "false")] fill_width: bool,
    #[prop(callback)] on_click: (),
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let hovered = create_signal(false);
    let size = size.get();
    let fill_width = fill_width.get();

    let (h, pad, radius, font) = match size {
        ButtonSize::Small => (24, 4, 8, 10),
        ButtonSize::Normal => (32, 8, 12, 10),
        ButtonSize::Large => (50, 10, 16, 16),
    };

    let mut c = container()
        .height(h)
        .padding([0, pad])
        .corner_radius(radius)
        .squircle()
        .on_click_option(on_click.clone())
        .on_hover(move |h| hovered.set(h))
        .pressed_state(|s| s.ripple())
        .background(move || {
            let k = kind.get();
            let hier = hierarchy.get();
            match k {
                ButtonKind::Transparent => {
                    if hovered.get() {
                        Color::rgba(1.0, 1.0, 1.0, 0.1)
                    } else {
                        Color::TRANSPARENT
                    }
                }
                ButtonKind::Solid => {
                    if hovered.get() {
                        hier.hover_bg(&theme)
                    } else {
                        hier.solid_bg(&theme)
                    }
                }
            }
        });

    c = match size {
        ButtonSize::Large => c.width(fill()),
        _ if fill_width => c.width(fill()),
        ButtonSize::Small => c.width(at_least(24)),
        ButtonSize::Normal => c.width(at_least(32)),
    };

    c = match size {
        ButtonSize::Large => c.layout(
            Flex::row()
                .spacing(8)
                .cross_alignment(CrossAlignment::Center),
        ),
        _ if fill_width => c.layout(
            Flex::row()
                .spacing(8)
                .cross_alignment(CrossAlignment::Center),
        ),
        _ => c.layout(
            Flex::row()
                .main_alignment(MainAlignment::Center)
                .cross_alignment(CrossAlignment::Center),
        ),
    };

    c = c.maybe_child(icon.get().map(|_| {
        super::icons::icon()
            .kind(move || icon.get().unwrap_or_default())
            .font_size(font)
            .mono(true)
            .color(move || hierarchy.get().fg(&theme))
    }));

    c.maybe_child(content)
}

#[component]
pub fn icon_button(
    #[prop] icon: IconKind,
    #[prop(default = "ButtonKind::Solid")] kind: ButtonKind,
    #[prop(default = "ButtonSize::Normal")] size: ButtonSize,
    #[prop(default = "ButtonHierarchy::Secondary")] hierarchy: ButtonHierarchy,
    #[prop(callback)] on_click: (),
) -> impl Widget {
    let theme = expect_context::<ThemeColors>();
    let size = size.get();

    let (size, _pad, font) = match size {
        ButtonSize::Small => (24, 4, 10),
        ButtonSize::Normal => (32, 8, 10),
        ButtonSize::Large => (50, 10, 16),
    };

    container()
        .height(size)
        .width(size)
        .corner_radius(size)
        .on_click_option(on_click.clone())
        .pressed_state(|s| s.ripple())
        .hover_state(|c| c.lighter(0.1))
        .background(move || {
            let k = kind.get();
            let hier = hierarchy.get();
            match k {
                ButtonKind::Transparent => Color::TRANSPARENT,
                ButtonKind::Solid => hier.solid_bg(&theme),
            }
        })
        .layout(
            Flex::row()
                .cross_alignment(CrossAlignment::Center)
                .main_alignment(MainAlignment::Center),
        )
        .child(
            super::icons::icon()
                .kind(move || icon.get())
                .font_size(font)
                .mono(true)
                .color(move || {
                    let k = kind.get();
                    let hier = hierarchy.get();
                    match k {
                        ButtonKind::Transparent => hier.solid_bg(&theme),
                        ButtonKind::Solid => hier.fg(&theme),
                    }
                }),
        )
}
