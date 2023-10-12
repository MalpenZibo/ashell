use gtk4::traits::ButtonExt;
use gtk4::{Label, Widget};
use leptos::{create_signal, SignalUpdate};

use crate::gtk4_wrapper::{center_box, container, Align, Component};
use crate::modules::{app_launcher, title, updates};

pub fn bar() -> Widget {
    let (value, set_value) = create_signal::<Option<Widget>>(None);

    let button = gtk4::Button::builder().label("remove center").build();
    button.connect_clicked(move |_| {
        set_value.update(|current| {
            if current.is_some() {
                *current = None;
            } else {
                *current = Some(Label::builder().label("Hello dyn").build().into());
            }
        });
    });

    center_box()
        .class(vec!["ph-1"])
        .valign(Align::Center)
        .vexpand(false)
        .left(Some(
            container()
                .valign(Align::Center)
                .vexpand(false)
                .children(vec![app_launcher(), updates()])
                .into(),
        ))
        // .center(Some(container().children(vec![title()]).into()))
        // .right(Some(button.into()))
        .into()
}
