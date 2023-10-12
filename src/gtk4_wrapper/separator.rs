use super::{Component, Orientation};
use gtk4::{traits::OrientableExt, Widget};
use leptos::{create_effect, MaybeSignal, SignalGet};

#[derive(Default, Clone)]
pub struct Separator(gtk4::Separator);

impl Component for Separator {
    fn get_widget(&self) -> gtk4::Widget {
        self.0.clone().into()
    }
}

pub fn separator() -> Separator {
    Separator::default()
}

impl Separator {
    pub fn orientation(self, orientation: impl Into<MaybeSignal<Orientation>>) -> Self {
        create_effect({
            let separator = self.0.clone();
            let orientation = orientation.into();

            move |_| {
                separator.set_orientation(orientation.get().into());
            }
        });

        self
    }
}

impl From<Separator> for Widget {
    fn from(separator: Separator) -> Self {
        separator.0.into()
    }
}
