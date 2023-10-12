use gtk4::{
    traits::{BoxExt, OrientableExt, WidgetExt},
    Widget,
};
use leptos::{create_effect, MaybeSignal, SignalGet};

use super::Component;

#[derive(Copy, Clone)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

impl From<Orientation> for gtk4::Orientation {
    fn from(value: Orientation) -> Self {
        match value {
            Orientation::Horizontal => gtk4::Orientation::Horizontal,
            Orientation::Vertical => gtk4::Orientation::Vertical,
        }
    }
}

#[derive(Default, Clone)]
pub struct Container(gtk4::Box);

pub fn container() -> Container {
    Container::default()
}

impl Container {
    pub fn orientation(self, orientation: impl Into<MaybeSignal<Orientation>>) -> Self {
        create_effect({
            let element = self.0.clone();
            let orientation = orientation.into();

            move |_| {
                element.set_orientation(orientation.get().into());
            }
        });

        self
    }

    pub fn spacing(self, spacing: impl Into<MaybeSignal<i32>>) -> Self {
        create_effect({
            let element = self.0.clone();
            let spacing = spacing.into();

            move |_| {
                element.set_spacing(spacing.get());
            }
        });

        self
    }

    pub fn homogeneous(self, value: impl Into<MaybeSignal<bool>>) -> Self {
        create_effect({
            let element = self.0.clone();
            let value = value.into();

            move |_| {
                element.set_homogeneous(value.get());
            }
        });

        self
    }

    pub fn children(self, children: impl Into<MaybeSignal<Vec<Widget>>>) -> Self {
        create_effect({
            let element = self.0.clone();
            let children = children.into();

            move |_| {
                while let Some(row) = element.last_child() {
                    element.remove(&row);
                }

                for child in children.get() {
                    element.append(&child);
                }
            }
        });

        self
    }
}

impl Component for Container {
    fn get_widget(&self) -> gtk4::Widget {
        self.0.clone().into()
    }
}

impl From<Container> for Widget {
    fn from(w: Container) -> Self {
        w.0.into()
    }
}
