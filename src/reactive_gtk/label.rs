use futures_signals::signal::{Signal, SignalExt};

use super::{
    spawner::{spawn, Handle},
    Align, AsStr, Component, Node,
};

pub enum Justification {
    Left,
    Right,
    Center,
    Fill,
}

impl From<Justification> for gtk4::Justification {
    fn from(value: Justification) -> Self {
        match value {
            Justification::Left => gtk4::Justification::Left,
            Justification::Right => gtk4::Justification::Right,
            Justification::Center => gtk4::Justification::Center,
            Justification::Fill => gtk4::Justification::Fill,
        }
    }
}

pub enum XAlign {
    Left,
    Center,
    Right,
}

impl From<XAlign> for f32 {
    fn from(value: XAlign) -> Self {
        match value {
            XAlign::Left => 0.,
            XAlign::Center => 0.5,
            XAlign::Right => 1.0,
        }
    }
}

#[derive(Default, Clone)]
pub struct Label {
    element: gtk4::Label,
    pub(crate) handlers: Vec<Handle<()>>,
}

impl From<Label> for Node {
    fn from(value: Label) -> Self {
        Node {
            component: value.element.into(),
            handlers: value.handlers,
        }
    }
}

impl Component for Label {
    fn get_widget(&self) -> gtk4::Widget {
        self.element.clone().into()
    }

    fn get_handlers(&mut self) -> &mut Vec<Handle<()>> {
        &mut self.handlers
    }
}

impl Label {
    pub fn text<A: AsStr>(mut self, text: A) -> Self {
        text.with_str(|s| {
            self.element.set_text(s);
        });

        self
    }

    pub fn text_signal<A: AsStr, S: Signal<Item = A> + 'static>(mut self, text: S) -> Self {
        let element = self.element.clone();

        let h = spawn(text.for_each(move |text| {
            text.with_str(|s| {
                element.set_text(s);
            });

            async {}
        }));

        self.handlers.push(h);

        self
    }

    pub fn limit_width(mut self, limit: i32) -> Self {
        self.element.set_max_width_chars(limit);

        self
    }

    pub fn limit_width_signal<S: Signal<Item = i32> + 'static>(mut self, limit: S) -> Self {
        let element = self.element.clone();

        let h = spawn(limit.for_each(move |limit| {
            element.set_max_width_chars(limit);

            async {}
        }));

        self.handlers.push(h);

        self
    }

    pub fn justify(mut self, justify: Justification) -> Self {
        self.element.set_justify(justify.into());

        self
    }

    pub fn justify_signal<S: Signal<Item = Justification> + 'static>(mut self, justify: S) -> Self {
        let element = self.element.clone();

        let h = spawn(justify.for_each(move |justify| {
            element.set_justify(justify.into());

            async {}
        }));

        self.handlers.push(h);

        self
    }

    pub fn xalign(self, value: XAlign) -> Self {
        self.element.set_xalign(value.into());

        self
    }

    pub fn xalign_signal<S: Signal<Item = XAlign> + 'static>(mut self, value: S) -> Self {
        let element = self.element.clone();

        let h = spawn(value.for_each(move |justify| {
            element.set_xalign(justify.into());

            async {}
        }));

        self.handlers.push(h);

        self
    }
}
