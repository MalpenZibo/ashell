use gtk4::traits::{GestureExt, WidgetExt};
use leptos::{create_effect, MaybeSignal, SignalGet};

mod app;
mod r#box;
mod centerbox;
mod label;
mod overlay;
mod spawner;

pub use app::*;
pub use centerbox::*;
pub use label::*;
pub use overlay::*;
pub use r#box::*;
pub use spawner::*;

#[derive(Copy, Clone)]
pub enum Align {
    Fill,
    Baseline,
    Start,
    Center,
    End,
}

impl From<Align> for gtk4::Align {
    fn from(value: Align) -> Self {
        match value {
            Align::Fill => gtk4::Align::Fill,
            Align::Baseline => gtk4::Align::Baseline,
            Align::Start => gtk4::Align::Start,
            Align::Center => gtk4::Align::Center,
            Align::End => gtk4::Align::End,
        }
    }
}

pub trait Component: Sized {
    fn get_widget(&self) -> gtk4::Widget;

    fn class(self, value: impl Into<MaybeSignal<Vec<&'static str>>> + 'static) -> Self {
        create_effect({
            let widget = self.get_widget();

            let value = value.into();
            move |_| {
                widget.set_css_classes(value.get().as_slice());
            }
        });

        self
    }

    fn vexpand(self, value: impl Into<MaybeSignal<bool>> + Copy + 'static) -> Self {
        create_effect({
            let widget = self.get_widget();

            move |_| {
                widget.set_vexpand(value.into().get());
            }
        });

        self
    }

    fn hexpand(self, value: impl Into<MaybeSignal<bool>> + Copy + 'static) -> Self {
        create_effect({
            let widget = self.get_widget();

            move |_| {
                widget.set_hexpand(value.into().get());
            }
        });

        self
    }

    fn valign(self, value: impl Into<MaybeSignal<Align>> + Copy + 'static) -> Self {
        create_effect({
            let widget = self.get_widget();

            move |_| {
                widget.set_valign(value.into().get().into());
            }
        });

        self
    }

    fn halign(self, value: impl Into<MaybeSignal<Align>> + Copy + 'static) -> Self {
        create_effect({
            let widget = self.get_widget();

            move |_| {
                widget.set_halign(value.into().get().into());
            }
        });

        self
    }

    fn active(self, value: impl Into<MaybeSignal<bool>> + Copy + 'static) -> Self {
        create_effect({
            let widget = self.get_widget();

            move |_| {
                widget.set_sensitive(value.into().get());
            }
        });

        self
    }

    fn visible(self, value: impl Into<MaybeSignal<bool>> + Copy + 'static) -> Self {
        create_effect({
            let widget = self.get_widget();

            move |_| {
                widget.set_visible(value.into().get());
            }
        });

        self
    }

    fn size(self, value: impl Into<MaybeSignal<(i32, i32)>> + Copy + 'static) -> Self {
        create_effect({
            let widget = self.get_widget();

            move |_| {
                let (width, height): (i32, i32) = value.into().get();
                widget.set_size_request(width, height)
            }
        });

        self
    }

    fn on_click(self, onclick: impl Fn() + 'static) -> Self {
        let gesture = gtk4::GestureClick::new();

        gesture.connect_released(move |gesture, _, _, _| {
            gesture.set_state(gtk4::EventSequenceState::Claimed);

            onclick();
        });

        let widget = self.get_widget();

        widget.add_controller(gesture);

        self
    }
}
