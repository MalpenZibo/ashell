use gtk4::Widget;
use leptos::MaybeSignal;
use typed_builder::TypedBuilder;

use crate::gtk4_wrapper::{label, Component};

#[derive(TypedBuilder)]
pub struct Pills {
    #[builder(setter(into))]
    text: MaybeSignal<String>,
    #[builder(default, setter(into))]
    visible: MaybeSignal<bool>,
    #[builder(default, setter(strip_option))]
    on_click: Option<Box<dyn Fn()>>,
}

impl From<Pills> for Widget {
    fn from(pills: Pills) -> Self {
        let classes = if pills.on_click.is_some() {
            vec!["bg", "pv-1", "ph-2", "rounded-m", "interactive"]
        } else {
            vec!["bg", "pv-1", "ph-2", "rounded-m"]
        };
        let label = label()
            .class(classes)
            .vexpand(false)
            .text(pills.text)
            .visible(pills.visible);

        if let Some(on_click) = pills.on_click {
            label.on_click(on_click).into()
        } else {
            label.into()
        }
    }
}
