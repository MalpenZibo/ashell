use hex_color::HexColor;
use iced::{Color, Theme};

use crate::config::{Appearance, AppearanceColor};

pub struct PrebuiltTheme {
    pub theme: Theme,
}
impl PrebuiltTheme {
    pub fn parse(theme_name: &String) -> Option<Self> {
        let theme = match theme_name
            .to_lowercase()
            .replace(|c| [' ', '-', '_'].contains(&c), "")
            .as_str()
        {
            "light" => Some(Theme::Light),
            "dark" => Some(Theme::Dark),
            "dracula" => Some(Theme::Dracula),
            "nord" => Some(Theme::Nord),
            "solarizedlight" => Some(Theme::SolarizedLight),
            "solarizeddark" => Some(Theme::SolarizedDark),
            "gruvboxlight" => Some(Theme::GruvboxLight),
            "gruvboxdark" => Some(Theme::GruvboxDark),
            "catppuccinlatte" => Some(Theme::CatppuccinLatte),
            "catppuccinfrappe" => Some(Theme::CatppuccinFrappe),
            "catppuccinmacchiato" => Some(Theme::CatppuccinMacchiato),
            "catppuccinmocha" | "catppuccin" => Some(Theme::CatppuccinMocha),
            "tokyonight" => Some(Theme::TokyoNight),
            "tokyonightstorm" => Some(Theme::TokyoNightStorm),
            "tokyonightlight" => Some(Theme::TokyoNightLight),
            "kanagawawave" => Some(Theme::KanagawaWave),
            "kanagawadragon" => Some(Theme::KanagawaDragon),
            "kanagawalotus" => Some(Theme::KanagawaLotus),
            "moonfly" => Some(Theme::Moonfly),
            "nightfly" => Some(Theme::Nightfly),
            "oxocarbon" => Some(Theme::Oxocarbon),
            "ferra" => Some(Theme::Ferra),
            //Can add more using the custom theme in iced
            _ => None,
        };
        if let Some(t) = theme {
            return Some(PrebuiltTheme { theme: t });
        }
        return None;
    }
    pub fn apply(&self, app: &mut Appearance) {
        let extended = self.theme.extended_palette();
        let background = extended.background;
        app.background_color = crate::config::BackgroundAppearanceColor::Complete {
            base: color_to_hex_color(background.base.color),
            weakest: Some(color_to_hex_color(background.weakest.color)),
            weaker: Some(color_to_hex_color(background.weaker.color)),
            weak: Some(color_to_hex_color(background.weak.color)),
            neutral: Some(color_to_hex_color(background.neutral.color)),
            strong: Some(color_to_hex_color(background.strong.color)),
            stronger: Some(color_to_hex_color(background.stronger.color)),
            strongest: Some(color_to_hex_color(background.strongest.color)),
            text: Some(color_to_hex_color(background.base.text)),
        };
        let primary = extended.primary;
        app.primary_color = AppearanceColor::Complete {
            base: color_to_hex_color(primary.base.color),
            strong: Some(color_to_hex_color(primary.strong.color)),
            weak: Some(color_to_hex_color(primary.weak.color)),
            text: Some(color_to_hex_color(primary.base.color)),
        };
        let success = extended.success;
        app.success_color = AppearanceColor::Complete {
            base: color_to_hex_color(success.base.color),
            strong: Some(color_to_hex_color(success.strong.color)),
            weak: Some(color_to_hex_color(success.weak.color)),
            text: Some(color_to_hex_color(success.base.color)),
        };
        let warning = extended.warning;
        app.warning_color = AppearanceColor::Complete {
            base: color_to_hex_color(warning.base.color),
            strong: Some(color_to_hex_color(warning.strong.color)),
            weak: Some(color_to_hex_color(warning.weak.color)),
            text: Some(color_to_hex_color(warning.base.color)),
        };
        let danger = extended.danger;
        app.danger_color = AppearanceColor::Complete {
            base: color_to_hex_color(danger.base.color),
            strong: Some(color_to_hex_color(danger.strong.color)),
            weak: Some(color_to_hex_color(danger.weak.color)),
            text: Some(color_to_hex_color(danger.base.color)),
        };
        app.text_color = AppearanceColor::Complete {
            base: color_to_hex_color(primary.base.color),
            strong: Some(color_to_hex_color(primary.strong.color)),
            weak: Some(color_to_hex_color(primary.weak.color)),
            text: None,
        };
    }
}
pub fn color_to_hex_color(color: Color) -> HexColor {
    let r = (color.r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (color.g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (color.b.clamp(0.0, 1.0) * 255.0).round() as u8;
    let a = (color.a.clamp(0.0, 1.0) * 255.0).round() as u8;
    HexColor { r, g, b, a }
}
