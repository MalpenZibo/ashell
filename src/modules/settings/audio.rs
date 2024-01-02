use iced::widget::Text;

use crate::{components::icons::icon, utils::audio::{Sink, Sinks, Source, Sources}, style::YELLOW};

pub fn sink_indicator<'a>(data: &Vec<Sink>) -> Text<'a, iced::Renderer> {
    let icon_type = data.get_icon();

    icon(icon_type)
}

pub fn source_indicator<'a>(data: &Vec<Source>) -> Option<Text<'a, iced::Renderer>> {
    data.get_icon().map(|icon_type| icon(icon_type).style(YELLOW))
}
