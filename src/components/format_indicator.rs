use crate::{
    components::icons::IconKind, config::SettingsFormat, theme::AshellTheme, utils::IndicatorState,
};
use iced::{
    Alignment, Element, Theme,
    mouse::ScrollDelta,
    widget::{MouseArea, container, row},
};

pub struct FormatIndicator<'a, Msg> {
    theme: &'a AshellTheme,
    format: SettingsFormat,
    icon: IconKind,
    label_element: Element<'a, Msg>,
    state: IndicatorState,
    on_scroll: Option<Box<dyn Fn(ScrollDelta) -> Msg + 'a>>,
    on_right_press: Option<Msg>,
}

pub fn format_indicator<'a, Msg: 'static + Clone>(
    theme: &'a AshellTheme,
    format: SettingsFormat,
    icon: impl Into<IconKind>,
    label_element: Element<'a, Msg>,
    state: IndicatorState,
) -> FormatIndicator<'a, Msg> {
    FormatIndicator {
        theme,
        format,
        icon: icon.into(),
        label_element,
        state,
        on_scroll: None,
        on_right_press: None,
    }
}

impl<'a, Msg: 'static + Clone> FormatIndicator<'a, Msg> {
    pub fn on_scroll(mut self, handler: impl Fn(ScrollDelta) -> Msg + 'a) -> Self {
        self.on_scroll = Some(Box::new(handler));
        self
    }

    pub fn on_right_press(mut self, msg: Msg) -> Self {
        self.on_right_press = Some(msg);
        self
    }
}

impl<'a, Msg: 'static + Clone> From<FormatIndicator<'a, Msg>> for Element<'a, Msg> {
    fn from(fi: FormatIndicator<'a, Msg>) -> Self {
        let content = match fi.format {
            SettingsFormat::Icon => fi.icon.to_text().into(),
            SettingsFormat::Percentage | SettingsFormat::Time => fi.label_element,
            SettingsFormat::IconAndPercentage | SettingsFormat::IconAndTime => {
                row![fi.icon.to_text(), fi.label_element]
                    .spacing(fi.theme.space.xxs)
                    .align_y(Alignment::Center)
                    .into()
            }
        };

        let colored = match fi.state {
            IndicatorState::Normal => content,
            _ => container(content)
                .style(move |theme: &Theme| container::Style {
                    text_color: Some(match fi.state {
                        IndicatorState::Success => theme.palette().success,
                        IndicatorState::Warning => theme.palette().warning,
                        IndicatorState::Danger => theme.palette().danger,
                        IndicatorState::Normal => unreachable!(),
                    }),
                    ..Default::default()
                })
                .into(),
        };

        if fi.on_scroll.is_some() || fi.on_right_press.is_some() {
            let mut area = MouseArea::new(colored);
            if let Some(handler) = fi.on_scroll {
                area = area.on_scroll(handler);
            }
            if let Some(msg) = fi.on_right_press {
                area = area.on_right_press(msg);
            }
            area.into()
        } else {
            colored
        }
    }
}
