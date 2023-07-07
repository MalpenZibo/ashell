use futures_signals::{
    signal::{Signal, SignalExt},
    signal_vec::{SignalVec, SignalVecExt, VecDiff},
};
use gtk::traits::{BoxExt, GestureExt, OrientableExt, WidgetExt};

use crate::reactive_gtk::ChildrenState;

use super::{
    spawner::{spawn, Handle},
    Component, Node, Orientation,
};

#[derive(Default, Clone)]
pub struct CenterBox {
    element: gtk::CenterBox,
    handlers: Vec<Handle<()>>,
}

impl CenterBox {
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

    pub fn children(
        mut self,
        (left, center, right): (Option<Node>, Option<Node>, Option<Node>),
    ) -> Self {
        if let Some(mut left) = left {
            self.element.set_start_widget(Some(&left.component));
            self.handlers.extend_from_slice(&left.handlers);
            left.handlers.clear();
        }

        if let Some(mut center) = center {
            self.element.set_center_widget(Some(&center.component));
            self.handlers.extend_from_slice(&center.handlers);
            center.handlers.clear();
        }

        if let Some(mut right) = right {
            self.element.set_end_widget(Some(&right.component));
            self.handlers.extend_from_slice(&right.handlers);
            right.handlers.clear();
        }

        self
    }

    pub fn children_signal<
        S: Signal<Item = (Option<Node>, Option<Node>, Option<Node>)> + 'static,
    >(
        mut self,
        children: S,
    ) -> Self {
        let element = self.element.clone();
        let mut children_state: (Option<Node>, Option<Node>, Option<Node>) = (None, None, None);

        let h = spawn(children.for_each(move |(left, center, right)| {
            element.set_start_widget(left.as_ref().map(|w| &w.component));
            element.set_center_widget(center.as_ref().map(|w| &w.component));
            element.set_end_widget(right.as_ref().map(|w| &w.component));

            children_state.0 = left;
            children_state.1 = center;
            children_state.2 = right;

            async {}
        }));

        self.handlers.push(h);

        self
    }

    pub fn on_click(self, onclick: impl Fn() + 'static) -> Self {
        let gesture = gtk::GestureClick::new();
        gesture.connect_released(move |gesture, _, _, _| {
            gesture.set_state(gtk::EventSequenceState::Claimed);

            onclick();
        });
        self.element.add_controller(gesture);

        self
    }
}

impl Component for CenterBox {
    fn get_widget(&self) -> gtk::Widget {
        self.element.clone().into()
    }

    fn get_handlers(&mut self) -> &mut Vec<Handle<()>> {
        &mut self.handlers
    }
}

impl From<CenterBox> for Node {
    fn from(value: CenterBox) -> Self {
        Node {
            component: value.element.into(),
            handlers: value.handlers,
        }
    }
}
