use futures_signals::signal::{Signal, SignalExt};
use gtk4::traits::RangeExt;

use super::{
    spawner::{spawn, Handle},
    Component, Node,
};

#[derive(Default, Clone)]
pub struct Scale {
    element: gtk4::Scale,
    handlers: Vec<Handle<()>>,
}

impl Scale {
    pub fn range(self, (min, max): (f64, f64)) -> Self {
        self.element.set_range(min, max);
        self
    }

    pub fn range_signal<S: Signal<Item = (f64, f64)> + 'static>(mut self, range: S) -> Self {
        let element = self.element.clone();

        let handler = spawn(range.for_each(move |(min, max)| {
            element.set_range(min, max);

            async {}
        }));

        self.handlers.push(handler);

        self
    }

    pub fn value(self, value: f64) -> Self {
        self.element.set_value(value);
        self
    }

    pub fn value_signal<S: Signal<Item = f64> + 'static>(mut self, value: S) -> Self {
        let element = self.element.clone();

        let handler = spawn(value.for_each(move |value| {
            element.set_value(value);

            async {}
        }));

        self.handlers.push(handler);

        self
    }

    pub fn round_digits(self, step: i32) -> Self {
        self.element.set_round_digits(step);
        self
    }

    pub fn round_digits_signal<S: Signal<Item = i32> + 'static>(mut self, step: S) -> Self {
        let element = self.element.clone();

        let handler = spawn(step.for_each(move |step| {
            element.set_round_digits(step);

            async {}
        }));

        self.handlers.push(handler);

        self
    }

    pub fn on_change(self, on_change: impl Fn(f64) + 'static) -> Self {
        self.element.connect_value_changed(move |element| {
            on_change(element.value());
        });

        self
    }
}

impl Component for Scale {
    fn get_widget(&self) -> gtk4::Widget {
        self.element.clone().into()
    }

    fn get_handlers(&mut self) -> &mut Vec<Handle<()>> {
        &mut self.handlers
    }
}

impl From<Scale> for Node {
    fn from(value: Scale) -> Self {
        Node {
            component: value.element.into(),
            handlers: value.handlers,
        }
    }
}
