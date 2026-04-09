use std::ops::RangeInclusive;

use crate::{
    components::icons::{StaticIcon, icon_button, icon_mono},
    theme::AshellTheme,
    utils::remote_value,
};
use iced::{
    Alignment, Element,
    mouse::ScrollDelta,
    widget::{MouseArea, Row, slider},
};

pub struct SliderRow<'a, Msg> {
    theme: &'a AshellTheme,
    icon: StaticIcon,
    range: RangeInclusive<u32>,
    value: u32,
    on_change: Box<dyn Fn(remote_value::Message<u32>) -> Msg + 'a>,
    on_scroll: Box<dyn Fn(ScrollDelta) -> Msg + 'a>,
    on_icon_press: Option<Msg>,
    on_icon_right_press: Option<Msg>,
    trailing_toggle: Option<(bool, Msg)>,
}

pub fn slider_row<'a, Msg: 'static + Clone>(
    theme: &'a AshellTheme,
    icon: StaticIcon,
    range: RangeInclusive<u32>,
    value: u32,
    on_change: impl Fn(remote_value::Message<u32>) -> Msg + 'a,
    on_scroll: impl Fn(ScrollDelta) -> Msg + 'a,
) -> SliderRow<'a, Msg> {
    SliderRow {
        theme,
        icon,
        range,
        value,
        on_change: Box::new(on_change),
        on_scroll: Box::new(on_scroll),
        on_icon_press: None,
        on_icon_right_press: None,
        trailing_toggle: None,
    }
}

impl<'a, Msg: 'static + Clone> SliderRow<'a, Msg> {
    pub fn on_icon_press(mut self, msg: Msg) -> Self {
        self.on_icon_press = Some(msg);
        self
    }

    pub fn on_icon_right_press(mut self, msg: Msg) -> Self {
        self.on_icon_right_press = Some(msg);
        self
    }

    pub fn trailing_toggle(mut self, expanded: bool, on_press: Msg) -> Self {
        self.trailing_toggle = Some((expanded, on_press));
        self
    }
}

impl<'a, Msg: 'static + Clone> From<SliderRow<'a, Msg>> for Element<'a, Msg> {
    fn from(row: SliderRow<'a, Msg>) -> Self {
        let icon_element: Element<'a, Msg> = if let Some(msg) = row.on_icon_press {
            let btn = icon_button(row.theme, row.icon).on_press(msg);
            if let Some(right_msg) = row.on_icon_right_press {
                MouseArea::new(btn).on_right_press(right_msg).into()
            } else {
                btn.into()
            }
        } else {
            iced::widget::container(icon_mono(row.icon))
                .center_x(32.)
                .center_y(32.)
                .clip(true)
                .into()
        };

        let slider_element = MouseArea::new(
            Element::<'a, remote_value::Message<u32>>::from(
                slider(row.range, row.value, remote_value::Message::Request)
                    .on_release(remote_value::Message::Timeout),
            )
            .map(row.on_change),
        )
        .on_scroll(row.on_scroll);

        let trailing: Option<Element<'a, Msg>> = row.trailing_toggle.map(|(expanded, msg)| {
            let trailing_icon = if expanded {
                StaticIcon::Close
            } else {
                StaticIcon::RightArrow
            };
            icon_button(row.theme, trailing_icon).on_press(msg).into()
        });

        Row::with_capacity(3)
            .push(icon_element)
            .push(slider_element)
            .push(trailing)
            .align_y(Alignment::Center)
            .spacing(row.theme.space.xs)
            .into()
    }
}
