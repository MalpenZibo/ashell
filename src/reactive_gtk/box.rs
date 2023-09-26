use futures_signals::{
    signal::{Signal, SignalExt},
    signal_vec::{SignalVec, SignalVecExt, VecDiff},
};
use gtk4::traits::{BoxExt, GestureExt, OrientableExt, WidgetExt};

use crate::reactive_gtk::ChildrenState;

use super::{
    spawner::{spawn, Handle},
    Component, Node,
};

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
pub struct Box {
    element: gtk4::Box,
    handlers: Vec<Handle<()>>,
}

impl Box {
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

    pub fn spacing(self, spacing: i32) -> Self {
        self.element.set_spacing(spacing);

        self
    }

    pub fn spacing_signal<S: Signal<Item = i32> + 'static>(mut self, spacing: S) -> Self {
        let element = self.element.clone();

        let handler = spawn(spacing.for_each(move |spacing| {
            element.set_spacing(spacing);

            async {}
        }));

        self.handlers.push(handler);

        self
    }

    pub fn homogeneous(self, value: bool) -> Self {
        self.element.set_homogeneous(true);

        self
    }

    pub fn children(mut self, children: Vec<Node>) -> Self {
        for mut child in children {
            self.handlers.extend_from_slice(&child.handlers);
            child.handlers.clear();
            self.element.append(&child.component);
        }

        self
    }

    pub fn children_signal_vec<S: SignalVec<Item = Node> + 'static>(mut self, children: S) -> Self {
        let element = self.element.clone();
        let mut state = ChildrenState::default();

        let h = spawn(children.for_each(move |change| {
            match change {
                VecDiff::Replace { values } => {
                    state.replace(
                        values,
                        |child| element.append(child),
                        |child| element.remove(child),
                    );
                }
                VecDiff::RemoveAt { index } => {
                    state.remove_at(index, |child| element.remove(child));
                }
                VecDiff::InsertAt { index, value } => {
                    state.insert_at(index, value, |child, before_child| {
                        element.insert_before(child, Some(before_child))
                    });
                }
                VecDiff::UpdateAt { index, value } => {
                    state.update_at(index, value, |child, old_child| {
                        element.insert_before(child, Some(old_child));
                        element.remove(old_child);
                    });
                }
                VecDiff::Move {
                    old_index,
                    new_index,
                } => {
                    state.move_child(old_index, new_index, |child, before_child| {
                        element.remove(child);
                        element.insert_before(child, Some(before_child))
                    });
                }
                VecDiff::Push { value } => {
                    state.push(value, |child| element.append(child));
                }
                VecDiff::Pop {} => {
                    state.pop(|child| element.remove(child));
                }
                VecDiff::Clear {} => {
                    state.clear(|child| element.remove(child));
                }
            }

            async {}
        }));

        self.handlers.push(h);

        self
    }

    pub fn children_signal<S: Signal<Item = Vec<Node>> + 'static>(mut self, children: S) -> Self {
        let element = self.element.clone();
        let mut state = ChildrenState::default();

        let h = spawn(children.for_each(move |values| {
            state.replace(
                values,
                |child| element.append(child),
                |child| element.remove(child),
            );

            async {}
        }));

        self.handlers.push(h);

        self
    }
}

impl Component for Box {
    fn get_widget(&self) -> gtk4::Widget {
        self.element.clone().into()
    }

    fn get_handlers(&mut self) -> &mut Vec<Handle<()>> {
        &mut self.handlers
    }
}

impl From<Box> for Node {
    fn from(value: Box) -> Self {
        Node {
            component: value.element.into(),
            handlers: value.handlers,
        }
    }
}
