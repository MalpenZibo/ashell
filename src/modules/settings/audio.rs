use iced::widget::Text;

use crate::{components::icons::icon, utils::audio::{Sink, Sinks}};

pub fn sink_indicator<'a>(data: &Vec<Sink>) -> Text<'a, iced::Renderer> {
    let icon_type = data.get_icon();

    icon(icon_type)
}
