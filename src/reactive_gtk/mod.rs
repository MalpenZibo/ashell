use std::borrow::Cow;

use futures_signals::signal::{Signal, SignalExt};
use gtk::{
    traits::{GestureExt, WidgetExt},
    Widget,
};

use self::spawner::{spawn, Handle};

mod app;
mod r#box;
mod button;
mod centerbox;
mod label;
mod overlay;
mod scrolled_window;
mod separator;
pub mod spawner;

pub use app::*;
pub use button::*;
pub use centerbox::*;
pub use label::*;
pub use overlay::*;
pub use r#box::*;
pub use scrolled_window::*;
pub use separator::*;

#[derive(Clone)]
pub struct T(Handle<()>);

#[derive(Debug, Clone)]
pub struct Node {
    pub component: Widget,
    pub handlers: Vec<Handle<()>>,
}

impl Drop for Node {
    fn drop(&mut self) {
        for handler in self.handlers.drain(..) {
            handler.cancel();
        }
    }
}

pub enum Align {
    Fill,
    Baseline,
    Start,
    Center,
    End,
}

impl From<Align> for gtk::Align {
    fn from(value: Align) -> Self {
        match value {
            Align::Fill => gtk::Align::Fill,
            Align::Baseline => gtk::Align::Baseline,
            Align::Start => gtk::Align::Start,
            Align::Center => gtk::Align::Center,
            Align::End => gtk::Align::End,
        }
    }
}

pub trait Component: Sized {
    fn get_widget(&self) -> gtk::Widget;

    fn get_handlers(&mut self) -> &mut Vec<Handle<()>>;

    fn class(self, value: &[&str]) -> Self {
        let widget = self.get_widget();

        widget.set_css_classes(value);

        self
    }

    fn class_signal<A: AsStr, S: Signal<Item = Vec<A>> + 'static>(mut self, value: S) -> Self {
        let widget = self.get_widget();

        let handler = spawn(value.for_each(move |value| {
            let classes = value
                .into_iter()
                .map(|s| s.with_str(|s| s.to_owned()))
                .collect::<Vec<String>>();
            widget.set_css_classes(
                classes
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<&str>>()
                    .as_slice(),
            );

            async {}
        }));

        let handlers = self.get_handlers();
        handlers.push(handler);

        self
    }

    fn vexpand(self, value: bool) -> Self {
        let widget = self.get_widget();

        widget.set_vexpand(value);

        self
    }

    fn vexpand_signal<S: Signal<Item = bool> + 'static>(mut self, value: S) -> Self {
        let widget = self.get_widget();

        let handler = spawn(value.for_each(move |value| {
            widget.set_vexpand(value);

            async {}
        }));

        let handlers = self.get_handlers();
        handlers.push(handler);

        self
    }

    fn hexpand(self, value: bool) -> Self {
        let widget = self.get_widget();

        widget.set_hexpand(value);

        self
    }

    fn hexpand_signal<S: Signal<Item = bool> + 'static>(mut self, value: S) -> Self {
        let widget = self.get_widget();

        let handler = spawn(value.for_each(move |value| {
            widget.set_hexpand(value);

            async {}
        }));

        let handlers = self.get_handlers();
        handlers.push(handler);

        self
    }

    fn valign(self, value: Align) -> Self {
        let widget = self.get_widget();

        widget.set_valign(value.into());

        self
    }

    fn valign_signal<S: Signal<Item = Align> + 'static>(mut self, value: S) -> Self {
        let widget = self.get_widget();

        let handler = spawn(value.for_each(move |value| {
            widget.set_valign(value.into());

            async {}
        }));

        let handlers = self.get_handlers();
        handlers.push(handler);

        self
    }

    fn halign(self, value: Align) -> Self {
        let widget = self.get_widget();

        widget.set_halign(value.into());

        self
    }

    fn halign_signal<S: Signal<Item = Align> + 'static>(mut self, value: S) -> Self {
        let widget = self.get_widget();

        let handler = spawn(value.for_each(move |value| {
            widget.set_halign(value.into());

            async {}
        }));

        let handlers = self.get_handlers();
        handlers.push(handler);

        self
    }

    fn active(self, value: bool) -> Self {
        let widget = self.get_widget();

        widget.set_sensitive(value);

        self
    }

    fn active_signal<S: Signal<Item = bool> + 'static>(mut self, value: S) -> Self {
        let widget = self.get_widget();

        let handler = spawn(value.for_each(move |value| {
            widget.set_sensitive(value);

            async {}
        }));

        let handlers = self.get_handlers();
        handlers.push(handler);

        self
    }

    fn visible(self, value: bool) -> Self {
        let widget = self.get_widget();

        widget.set_visible(value);

        self
    }

    fn visible_signal<S: Signal<Item = bool> + 'static>(mut self, value: S) -> Self {
        let widget = self.get_widget();

        let handler = spawn(value.for_each(move |value| {
            widget.set_visible(value);

            async {}
        }));

        let handlers = self.get_handlers();
        handlers.push(handler);

        self
    }

    fn size(self, value: (i32, i32)) -> Self {
        let widget = self.get_widget();

        widget.set_size_request(value.0, value.1);

        self
    }

    fn size_signal<S: Signal<Item = (i32, i32)> + 'static>(mut self, value: S) -> Self {
        let widget = self.get_widget();

        let handler = spawn(value.for_each(move |value| {
            widget.set_size_request(value.0, value.1);

            async {}
        }));

        let handlers = self.get_handlers();
        handlers.push(handler);

        self
    }

    fn on_click(self, onclick: impl Fn() + 'static) -> Self {
        let gesture = gtk::GestureClick::new();

        gesture.connect_released(move |gesture, _, _, _| {
            gesture.set_state(gtk::EventSequenceState::Claimed);

            onclick();
        });

        let widget = self.get_widget();

        widget.add_controller(gesture);

        self
    }
}

#[derive(Default)]
struct ChildrenState {
    children: Vec<Node>,
}

impl ChildrenState {
    pub fn replace<F1: Fn(&Widget), F2: Fn(&Widget)>(
        &mut self,
        children: Vec<Node>,
        append: F1,
        remove: F2,
    ) {
        for child in self.children.drain(..) {
            remove(&child.component);
        }

        self.children = children;

        for child in self.children.iter() {
            append(&child.component);
        }
    }

    fn remove_at<F1: Fn(&Widget)>(&mut self, index: usize, remove: F1) {
        if let Some(child) = self.children.get(index) {
            remove(&child.component);
        }
        self.children.remove(index);
    }

    fn insert_at<F1: Fn(&Widget, &Widget)>(
        &mut self,
        index: usize,
        child: Node,
        insert_before: F1,
    ) {
        if let Some(before_child) = self.children.get(index) {
            insert_before(&child.component, &before_child.component);
            self.children.insert(index, child);
        }
    }

    fn update_at<F1: Fn(&Widget, &Widget)>(&mut self, index: usize, child: Node, replace: F1) {
        if let Some(old_child) = self.children.get(index) {
            replace(&child.component, &old_child.component);
            self.children[index] = child;
        }
    }

    fn move_child<F1: Fn(&Widget, &Widget)>(
        &mut self,
        old_index: usize,
        new_index: usize,
        move_child: F1,
    ) {
        if let Some(child) = self.children.get(old_index) {
            if let Some(before_child) = self.children.get(new_index) {
                move_child(&child.component, &before_child.component);
            }
        }
        let child = self.children.remove(old_index);
        self.children.insert(new_index, child);
    }

    fn push<F1: Fn(&Widget)>(&mut self, child: Node, append: F1) {
        append(&child.component);
        self.children.push(child);
    }

    fn pop<F1: Fn(&Widget)>(&mut self, remove: F1) {
        if let Some(child) = self.children.pop() {
            remove(&child.component);
        }
    }

    fn clear<F1: Fn(&Widget)>(&mut self, remove: F1) {
        while let Some(child) = self.children.pop() {
            remove(&child.component);
        }
    }
}

pub trait AsStr {
    fn with_str<A, F>(&self, f: F) -> A
    where
        F: FnOnce(&str) -> A;
}

impl<'a, A> AsStr for &'a A
where
    A: AsStr,
{
    #[inline]
    fn with_str<B, F>(&self, f: F) -> B
    where
        F: FnOnce(&str) -> B,
    {
        AsStr::with_str(*self, f)
    }
}

impl AsStr for String {
    #[inline]
    fn with_str<A, F>(&self, f: F) -> A
    where
        F: FnOnce(&str) -> A,
    {
        f(self)
    }
}

impl AsStr for str {
    #[inline]
    fn with_str<A, F>(&self, f: F) -> A
    where
        F: FnOnce(&str) -> A,
    {
        f(self)
    }
}

impl<'a> AsStr for &'a str {
    #[inline]
    fn with_str<A, F>(&self, f: F) -> A
    where
        F: FnOnce(&str) -> A,
    {
        f(self)
    }
}

impl<'a> AsStr for Cow<'a, str> {
    #[inline]
    fn with_str<A, F>(&self, f: F) -> A
    where
        F: FnOnce(&str) -> A,
    {
        f(self)
    }
}

#[cfg(test)]
mod tests {
    use futures_signals::signal::Mutable;

    use super::Label;

    // use super::{Label, Value};

    // use super::{label, Label, UIBuilder};

    #[test]
    fn apply() {
        let f = Mutable::new("ciao");
        let fff = f.signal();

        Label::default().text_signal(fff);
    }
}
