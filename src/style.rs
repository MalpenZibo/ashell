use iced::{theme::Palette, widget::button, BorderRadius, Color, Theme};

pub const BASE: Color = Color::from_rgb(0.117_647_06, 0.117_647_06, 0.180_392_16);
pub const MANTLE: Color = Color::from_rgb(0.094117647, 0.094117647, 0.145098039);
pub const CRUST: Color = Color::from_rgb(0.066_666_67, 0.066_666_68, 0.105_882_353);
pub const SURFACE_0: Color = Color::from_rgb(0.192_156_87, 0.196_078_43, 0.266_666_68);
pub const SURFACE_1: Color = Color::from_rgb(0.270_588_25,0.278_431_43,0.352_941_26);
pub const TEXT: Color = Color::from_rgb(0.803_921_6, 0.839_215_7, 0.956_862_75);
pub const PEACH: Color = Color::from_rgb(0.980_392_16, 0.701_960_84, 0.529_411_85);
pub const LAVENDER: Color = Color::from_rgb(0.705_882_4, 0.745_098_05, 0.996_078_43);
pub const MAUVE: Color = Color::from_rgb(0.796_078_44, 0.650_980_4, 0.968_627_45);
pub const RED: Color = Color::from_rgb(0.952_941_2, 0.545_098_07, 0.658_823_55);
pub const YELLOW: Color = Color::from_rgb(0.976_470_6, 0.886_274_5, 0.686_274_5);
pub const GREEN: Color = Color::from_rgb(0.650_980_4, 0.890_196_1, 0.631_372_6);

pub fn ashell_theme() -> Theme {
    Theme::custom(Palette {
        background: BASE,
        text: TEXT,
        primary: PEACH,
        success: GREEN,
        danger: RED,
    })
}

pub fn header_pills(theme: &Theme) -> iced::widget::container::Appearance {
    let palette = theme.palette();
    iced::widget::container::Appearance {
        background: Some(palette.background.into()),
        border_radius: BorderRadius::from(12.0),
        border_width: 0.0,
        border_color: iced::Color::TRANSPARENT,
        text_color: Some(palette.text),
        ..Default::default()
    }
}

pub fn left_header_pills(theme: &Theme) -> iced::widget::container::Appearance {
    let palette = theme.palette();
    iced::widget::container::Appearance {
        background: Some(palette.background.into()),
        border_radius: BorderRadius::from([12.0, 0.0, 0.0, 12.0]),
        border_width: 0.0,
        border_color: iced::Color::TRANSPARENT,
        text_color: Some(palette.text),
        ..Default::default()
    }
}

pub enum HeaderButtonStyle {
    Full,
    Left,
    Right,
}

impl button::StyleSheet for HeaderButtonStyle {
    type Style = iced::theme::Theme;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(style.palette().background.into()),
            border_radius: match self {
                HeaderButtonStyle::Full => BorderRadius::from(12.0),
                HeaderButtonStyle::Left => BorderRadius::from([12.0, 0.0, 0.0, 12.0]),
                HeaderButtonStyle::Right => BorderRadius::from([0.0, 12.0, 12.0, 0.0]),
            },
            border_width: 0.0,
            border_color: iced::Color::TRANSPARENT,
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
            border_radius: BorderRadius::from(4.0),
            border_width: 0.0,
            border_color: iced::Color::TRANSPARENT,
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
            border_radius: BorderRadius::from(32.0),
            border_width: 0.0,
            border_color: iced::Color::TRANSPARENT,
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
