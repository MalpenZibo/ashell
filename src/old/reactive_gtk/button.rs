use futures_signals::signal::{Signal, SignalExt};
use gtk4::traits::ButtonExt;

use super::{
    spawner::{spawn, Handle},
    Component, Node,
};

#[derive(Default, Clone)]
pub struct Button {
    element: gtk4::Button,
    handlers: Vec<Handle<()>>,
}

impl Button {
    pub fn on_click(self, onclick: impl Fn() + 'static) -> Self {
        self.element.connect_clicked(move |_| {
            onclick();
        });

        self
    }

    pub fn child<C: Into<Node>>(mut self, child: C) -> Self {
        let mut child = child.into();
        self.element.set_child(Some(&child.component));

        self.handlers.extend_from_slice(&child.handlers);
        child.handlers.clear();

        self
    }

    pub fn child_signal<C: Into<Node>, S: Signal<Item = C> + 'static>(mut self, child: S) -> Self {
        let element = self.element.clone();

        let handler = spawn(child.for_each(move |child| {
            let child = child.into();
            element.set_child(Some(&child.component));

            async {}
        }));

        self.handlers.push(handler);

        self
    }
}

impl Component for Button {
    fn get_widget(&self) -> gtk4::Widget {
        self.element.clone().into()
    }

    fn get_handlers(&mut self) -> &mut Vec<Handle<()>> {
        &mut self.handlers
    }
}

impl From<Button> for Node {
    fn from(value: Button) -> Self {
        Node {
            component: value.element.into(),
            handlers: value.handlers,
        }
    }
}
