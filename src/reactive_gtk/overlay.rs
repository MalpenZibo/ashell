use futures_signals::signal_vec::{SignalVec, SignalVecExt, VecDiff};
use gtk::traits::WidgetExt;

use crate::reactive_gtk::ChildrenState;

use super::{
    spawner::{spawn, Handle},
    Component, Node,
};

#[derive(Default, Clone)]
pub struct Overlay {
    element: gtk::Overlay,
    handlers: Vec<Handle<()>>,
}

impl Overlay {
    pub fn children(mut self, children: Vec<Node>) -> Self {
        for mut child in children {
            self.handlers.extend_from_slice(&child.handlers);
            child.handlers.clear();
            self.element.add_overlay(&child.component);
        }

        self
    }

    pub fn children_signal<S: SignalVec<Item = Node> + 'static>(mut self, children: S) -> Self {
        let element = self.element.clone();
        let mut state = ChildrenState::default();

        let h = spawn(children.for_each(move |change| {
            #[allow(clippy::single_match)]
            match change {
                VecDiff::Replace { values } => {
                    state.replace(
                        values,
                        |child| element.add_overlay(child),
                        |child| element.remove_overlay(child),
                    );
                }
                VecDiff::RemoveAt { index } => {
                    state.remove_at(index, |child| element.remove_overlay(child));
                }
                VecDiff::InsertAt { index, value } => {
                    state.insert_at(index, value, |child, before_child| {
                        element.insert_before(child, Some(before_child))
                    });
                }
                VecDiff::UpdateAt { index, value } => {
                    state.update_at(index, value, |child, old_child| {
                        element.insert_before(child, Some(old_child));
                        element.remove_overlay(old_child);
                    });
                }
                VecDiff::Move {
                    old_index,
                    new_index,
                } => {
                    state.move_child(old_index, new_index, |child, before_child| {
                        element.remove_overlay(child);
                        element.insert_before(child, Some(before_child))
                    });
                }
                VecDiff::Push { value } => {
                    state.push(value, |child| element.add_overlay(child));
                }
                VecDiff::Pop {} => {
                    state.pop(|child| element.remove_overlay(child));
                }
                VecDiff::Clear {} => {
                    state.clear(|child| element.remove_overlay(child));
                }
            }

            async {}
        }));

        self.handlers.push(h);

        self
    }
}

impl Component for Overlay {
    fn get_widget(&self) -> gtk::Widget {
        self.element.clone().into()
    }

    fn get_handlers(&mut self) -> &mut Vec<Handle<()>> {
        &mut self.handlers
    }
}

impl From<Overlay> for Node {
    fn from(value: Overlay) -> Self {
        Node {
            component: value.element.into(),
            handlers: value.handlers,
        }
    }
}
