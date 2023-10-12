use gtk4::Widget;
use leptos::{create_effect, MaybeSignal, SignalGet};

use super::Component;

#[derive(Default, Clone)]
pub struct CenterBox(gtk4::CenterBox);

impl Component for CenterBox {
    fn get_widget(&self) -> gtk4::Widget {
        self.0.clone().into()
    }
}

pub fn center_box() -> CenterBox {
    CenterBox::default()
}

impl CenterBox {
    pub fn left(self, left: impl Into<MaybeSignal<Option<Widget>>>) -> Self {
        create_effect({
            let center_box = self.0.clone();
            let left = left.into();

            move |_| {
                center_box.set_start_widget(left.get().as_ref());
            }
        });

        self
    }

    pub fn center(self, center: impl Into<MaybeSignal<Option<Widget>>>) -> Self {
        create_effect({
            let center_box = self.0.clone();
            let center = center.into();

            move |_| {
                center_box.set_center_widget(center.get().as_ref());
            }
        });

        self
    }

    pub fn right(self, right: impl Into<MaybeSignal<Option<Widget>>>) -> Self {
        create_effect({
            let center_box = self.0.clone();
            let right = right.into();

            move |_| {
                center_box.set_end_widget(right.get().as_ref());
            }
        });

        self
    }
}

impl From<CenterBox> for Widget {
    fn from(w: CenterBox) -> Self {
        w.0.into()
    }
}
