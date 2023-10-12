use gtk4::Widget;
use leptos::{create_effect, MaybeSignal, SignalGet};

use super::Component;

#[derive(Default, Clone)]
pub struct Label(gtk4::Label);

impl Component for Label {
    fn get_widget(&self) -> gtk4::Widget {
        self.0.clone().into()
    }
}

pub fn label() -> Label {
    Label::default()
}

impl Label {
    pub fn text(self, text: impl Into<MaybeSignal<String>> + 'static) -> Self {
        create_effect({
            let label = self.0.clone();
            let text = text.into();

            move |_| {
                label.set_text(text.get().as_str());
            }
        });

        self
    }
}

impl From<Label> for Widget {
    fn from(label: Label) -> Self {
        label.0.into()
    }
}
