use iced::{theme::Palette, widget::button, Border, Color, Theme};

pub const BASE: Color = Color::from_rgb(0.117_647_06, 0.117_647_06, 0.180_392_16);
pub const MANTLE: Color = Color::from_rgb(0.094_117_65, 0.094_117_66, 0.145_098_05);
pub const CRUST: Color = Color::from_rgb(0.066_666_67, 0.066_666_68, 0.105_882_353);
pub const SURFACE_0: Color = Color::from_rgb(0.192_156_87, 0.196_078_43, 0.266_666_68);
pub const SURFACE_1: Color = Color::from_rgb(0.270_588_25, 0.278_431_43, 0.352_941_26);
pub const TEXT: Color = Color::from_rgb(0.803_921_6, 0.839_215_7, 0.956_862_75);
pub const PEACH: Color = Color::from_rgb(0.980_392_16, 0.701_960_84, 0.529_411_85);
pub const LAVENDER: Color = Color::from_rgb(0.705_882_4, 0.745_098_05, 0.996_078_43);
pub const MAUVE: Color = Color::from_rgb(0.796_078_44, 0.650_980_4, 0.968_627_45);
pub const RED: Color = Color::from_rgb(0.952_941_2, 0.545_098_07, 0.658_823_55);
pub const YELLOW: Color = Color::from_rgb(0.976_470_6, 0.886_274_5, 0.686_274_5);
pub const GREEN: Color = Color::from_rgb(0.650_980_4, 0.890_196_1, 0.631_372_6);

pub fn ashell_theme() -> Theme {
    Theme::custom(
        "local".to_string(),
        Palette {
            background: BASE,
            text: TEXT,
            primary: PEACH,
            success: GREEN,
            danger: RED,
        },
    )
}

pub fn header_pills(theme: &Theme) -> iced::widget::container::Appearance {
    let palette = theme.palette();
    iced::widget::container::Appearance {
        background: Some(palette.background.into()),
        border: Border {
            width: 0.0,
            radius: 12.0.into(),
            color: iced::Color::TRANSPARENT,
        },
        text_color: Some(palette.text),
        ..Default::default()
    }
}

pub fn left_header_pills(theme: &Theme) -> iced::widget::container::Appearance {
    let palette = theme.palette();
    iced::widget::container::Appearance {
        background: Some(palette.background.into()),
        border: Border {
            width: 0.0,
            radius: [12.0, 0.0, 0.0, 12.0].into(),
            color: iced::Color::TRANSPARENT,
        },
        text_color: Some(palette.text),
        ..Default::default()
    }
}

pub enum HeaderButtonStyle {
    Full,
    None,
    Left,
    Right,
}

impl button::StyleSheet for HeaderButtonStyle {
    type Style = iced::theme::Theme;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(style.palette().background.into()),
            border: Border {
                width: 0.0,
                radius: match self {
                    HeaderButtonStyle::Full => 12.0.into(),
                    HeaderButtonStyle::Left => [12.0, 0.0, 0.0, 12.0].into(),
                    HeaderButtonStyle::Right => [0.0, 12.0, 12.0, 0.0].into(),
                    HeaderButtonStyle::None => 0.0.into(),

                },
                color: iced::Color::TRANSPARENT,
            },
            text_color: style.palette().text,
            ..button::Appearance::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(SURFACE_0.into()),
            ..self.active(style)
        }
    }
}

pub struct GhostButtonStyle;

impl button::StyleSheet for GhostButtonStyle {
    type Style = iced::theme::Theme;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: None,
            border: Border {
                width: 0.0,
                radius: 4.0.into(),
                color: iced::Color::TRANSPARENT,
            },
            text_color: style.palette().text,
            ..button::Appearance::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(iced::Background::Color(SURFACE_0)),
            ..self.active(style)
        }
    }
}

pub struct SettingsButtonStyle;

impl button::StyleSheet for SettingsButtonStyle {
    type Style = iced::theme::Theme;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(iced::Background::Color(SURFACE_0)),
            border: Border {
                width: 0.0,
                radius: 32.0.into(),
                color: iced::Color::TRANSPARENT,
            },
            text_color: style.palette().text,
            ..button::Appearance::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(SURFACE_1.into()),
            ..self.active(style)
        }
    }
}

pub struct SliderStyle;

// impl iced::widget::slider::StyleSheet for SliderStyle {
//     type Style = iced::theme::Theme;
//
//     fn active(&self, style: &Self::Style) -> iced::widget::slider::Appearance {
//         let palette = style.extended_palette();
//
//         let handle = iced::widget::slider::Handle {
//             shape: iced::widget::slider::HandleShape::Circle { radius: 8. } ,
//             color: Color::WHITE,
//             border_color: Color::WHITE,
//             border_width: 1.0,
//         };
//
//         iced::widget::slider::Appearance {
//             rail: iced::widget::slider::Rail {
//                 colors: iced::widget::slider::RailBackground::Pair(
//                     palette.primary.base.color,
//                     palette.secondary.base.color,
//                 ),
//                 width: 4.0,
//                 border_radius: 2.0.into(),
//             },
//             handle: iced::widget::slider::Handle {
//                 color: palette.background.base.color,
//                 border_color: palette.primary.base.color,
//                 ..handle
//             },
//             breakpoint: iced_style pv,
//         }
//     }
//
//     fn hovered(&self, style: &Self::Style) -> iced::widget::slider::Appearance {
//         self.active(style)
//     }
//
//     fn dragging(&self, style: &Self::Style) -> iced::widget::slider::Appearance {
//         self.active(style)
//     }
// }

pub struct QuickSettingsButtonStyle(pub bool);

impl button::StyleSheet for QuickSettingsButtonStyle {
    type Style = iced::theme::Theme;

    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(if self.0 {
                iced::Background::Color(PEACH)
            } else {
                iced::Background::Color(SURFACE_0)
            }),
            border: Border {
                width: 0.0,
                radius: 32.0.into(),
                color: iced::Color::TRANSPARENT,
            },
            text_color: if self.0 { SURFACE_0 } else { TEXT },
            ..button::Appearance::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let peach = style.extended_palette().primary.weak.color;

        button::Appearance {
            background: Some(if self.0 { peach } else { SURFACE_1 }.into()),
            ..self.active(style)
        }
    }
}

pub struct QuickSettingsSubMenuButtonStyle(pub bool);

impl button::StyleSheet for QuickSettingsSubMenuButtonStyle {
    type Style = iced::theme::Theme;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: None,
            border: Border {
                width: 0.0,
                radius: 16.0.into(),
                color: iced::Color::TRANSPARENT,
            },
            text_color: if self.0 {
                SURFACE_0
            } else {
                style.palette().text
            },
            ..button::Appearance::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(iced::Background::Color(SURFACE_0)),
            text_color: style.palette().text,
            ..self.active(style)
        }
    }
}
