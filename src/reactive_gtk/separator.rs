use futures_signals::signal::{Signal, SignalExt};
use gtk::traits::OrientableExt;

use super::{
    spawner::{spawn, Handle},
    Component, Node, Orientation,
};

#[derive(Default, Clone)]
pub struct Separator {
    element: gtk::Separator,
    handlers: Vec<Handle<()>>,
}

impl Separator {
    pub fn orientation(self, orientation: Orientation) -> Self {
        self.element.set_orientation(orientation.into());

        self
    }

    pub fn orientation_signal<S: Signal<Item = Orientation> + 'static>(
        mut self,
        orientation: S,
    ) -> Self {
        let element = self.element.clone();

        let handler = spawn(orientation.for_each(move |orientation| {
            element.set_orientation(orientation.into());

            async {}
        }));

        self.handlers.push(handler);

        self
    }
}

impl Component for Separator {
    fn get_widget(&self) -> gtk::Widget {
        self.element.clone().into()
    }

    fn get_handlers(&mut self) -> &mut Vec<Handle<()>> {
        &mut self.handlers
    }
}

impl From<Separator> for Node {
    fn from(value: Separator) -> Self {
        Node {
            component: value.element.into(),
            handlers: value.handlers,
        }
    }
}
