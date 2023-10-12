use futures_signals::signal::Signal;
use typed_builder::TypedBuilder;

use crate::reactive_gtk::{AsStr, Component, Label, Node};

#[derive(TypedBuilder)]
pub struct Pills<S: AsStr, C: Signal<Item = S> + 'static, V: Signal<Item = bool> + 'static> {
    content: C,
    visible: V,
}

impl<S: AsStr, C: Signal<Item = S> + 'static, V: Signal<Item = bool> + 'static> From<Pills<S, C, V>>
    for Node
{
    fn from(pills: Pills<S, C, V>) -> Self {
        Label::default()
            .class(&["bg", "ph-4", "rounded-m"])
            .text_signal(pills.content)
            .visible_signal(pills.visible)
            .into()
    }
}
