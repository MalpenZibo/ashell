use gtk4::{traits::WidgetExt, Widget};
use leptos::{create_effect, MaybeSignal, SignalGet};

use super::Component;

#[derive(Default, Clone)]
pub struct Overlay(gtk4::Overlay);

pub fn overlay() -> Overlay {
    Overlay::default()
}

impl Overlay {
    pub fn children(self, children: impl Into<MaybeSignal<Vec<Widget>>>) -> Self {
        create_effect({
            let element = self.0.clone();
            let children = children.into();

            move |_| {
                while let Some(row) = element.last_child() {
                    element.remove_overlay(&row);
                }

                for child in children.get() {
                    element.add_overlay(&child);
                    element.set_measure_overlay(&child, true);
                }
            }
        });

        self
    }
}

impl Component for Overlay {
    fn get_widget(&self) -> gtk4::Widget {
        self.0.clone().into()
    }
}

impl From<Overlay> for Widget {
    fn from(w: Overlay) -> Self {
        w.0.into()
    }
}
