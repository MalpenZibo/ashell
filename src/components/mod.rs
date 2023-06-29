use std::{
    borrow::Cow,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use crate::spawner::Handle;
use futures::stream::{AbortHandle, Abortable};
use futures_signals::{
    signal::{Signal, SignalExt},
    signal_vec::{SignalVec, SignalVecExt, VecDiff},
    CancelableFutureHandle,
};
use gtk::{
    prelude::IsA,
    traits::{BoxExt, OrientableExt, WidgetExt},
    DropTarget, Widget,
};
use typed_builder::TypedBuilder;

// Al Zibo del futuro
// l'idea è quella di avere un qualcosa dove mettere i vari box label etc...
// quel qualcosa deve gestirsi gli handler ai signal per ucciderli quando si troppa l'elemento
// e deve anche gestire i figli. Buona fortuna

#[derive(Clone)]
pub struct T(Handle<()>);

#[derive(Clone)]
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

impl From<gtk::Button> for Node {
    fn from(value: gtk::Button) -> Self {
        Node {
            component: value.into(),
            handlers: Vec::with_capacity(0),
        }
    }
}

#[derive(Default, Clone)]
pub struct Box {
    element: gtk::Box,
    handlers: Vec<Handle<()>>,
}

impl Box {
    pub fn spacing<S: Signal<Item = i32> + 'static>(mut self, spacing: Value<i32, S>) -> Self {
        match spacing {
            Value::Simple(spacing) => self.element.set_spacing(spacing),
            Value::Signal(spacing) => {
                let element = self.element.clone();

                let handler = crate::spawner::spawn(spacing.for_each(move |spacing| {
                    element.set_spacing(spacing);

                    async {}
                }));

                self.handlers.push(handler);
            }
        };

        self
    }

    pub fn children<S: SignalVec<Item = Node> + 'static>(
        mut self,
        children: VecValue<Node, S>,
    ) -> Self {
        match children {
            VecValue::Simple(children) => {
                for child in children {
                    self.handlers.extend_from_slice(&child.handlers);
                    self.element.append(&child.component);
                }
            }
            VecValue::Signal(children) => {
                println!("here?");
                struct State {
                    element: gtk::Box,
                    children: Vec<Node>,
                }

                let mut state = State {
                    element: self.element.clone(),
                    children: Vec::default(),
                };

                let h = crate::spawner::spawn(children.for_each(move |change| {
                    #[allow(clippy::single_match)]
                    match change {
                        VecDiff::Replace { values } => {
                            println!("here??????");
                            state.children.clear();

                            state.children = values;

                            for child in state.children.iter() {
                                println!("forse");
                                state.element.append(&child.component);
                            }

                            // for child in self.children.drain(..) {
                            //     element.remove(&child.component);
                            // }
                            // self.children.clone_from(&values);

                            // for child in self.children.iter() {
                            //     self.element.append(&child.component);
                            // }
                        }
                        VecDiff::RemoveAt { index } => {
                            if let Some(child) = state.children.get(index) {
                                state.element.remove(&child.component);
                            }
                            state.children.remove(index);
                        }
                        _ => {} // VecDiff::InsertAt { index, value } => {
                                //     let mut i = 0;
                                //     let mut some_child = element.first_child();
                                //     let mut found = None;
                                //     while let Some(child) = some_child {
                                //         if i == index {
                                //             found = Some(child);
                                //             break;
                                //         }
                                //         i += 1;
                                //         some_child = child.next_sibling();
                                //     }

                                //     element.insert_before(&value, found.as_ref());
                                // }
                                // VecDiff::UpdateAt { index, value } => {
                                //     let mut i = 0;
                                //     let mut some_child = element.first_child();
                                //     let mut found = None;
                                //     while let Some(child) = some_child {
                                //         if i == index {
                                //             found = Some(child);
                                //             break;
                                //         }
                                //         i += 1;
                                //         some_child = child.next_sibling();
                                //     }

                                //     element.insert_before(&value, found.as_ref());
                                //     if let Some(found) = found {
                                //         element.remove(&found);
                                //     }
                                // }
                                // VecDiff::RemoveAt { index } => {
                                //     let mut i = 0;
                                //     let mut some_child = element.first_child();
                                //     let mut found = None;
                                //     while let Some(child) = some_child {
                                //         if i == index {
                                //             found = Some(child);
                                //             break;
                                //         }
                                //         i += 1;
                                //         some_child = child.next_sibling();
                                //     }

                                //     if let Some(found) = found {
                                //         element.remove(&found);
                                //     }
                                // }
                                // VecDiff::Move {
                                //     old_index,
                                //     new_index,
                                // } => {
                                //     let mut i = 0;
                                //     let mut some_child = element.first_child();
                                //     let mut found = None;
                                //     while let Some(child) = some_child {
                                //         if i == old_index {
                                //             found = Some(child);
                                //             break;
                                //         }
                                //         i += 1;
                                //         some_child = child.next_sibling();
                                //     }

                                //     let mut i = 0;
                                //     let mut some_child = element.first_child();
                                //     let mut target = None;
                                //     while let Some(child) = some_child {
                                //         if i == new_index {
                                //             target = Some(child);
                                //             break;
                                //         }
                                //         i += 1;
                                //         some_child = child.next_sibling();
                                //     }

                                //     if let Some(found) = found {
                                //         if let Some(target) = target {
                                //             element.remove(&found);
                                //             element.insert_before(&found, Some(&target));
                                //         }
                                //     }
                                // }
                                // VecDiff::Push { value } => element.append(&value),
                                // VecDiff::Pop {} => {
                                //     if let Some(last_child) = element.last_child() {
                                //         element.remove(&last_child);
                                //     }
                                // }
                                // VecDiff::Clear {} => {
                                //     let mut some_child = element.first_child();
                                //     while let Some(child) = some_child {
                                //         element.remove(&child);

                                //         some_child = child.next_sibling();
                                //     }
                                // }
                    }

                    async {}
                }));

                self.handlers.push(h);

                // crate::spawner::spawn(children.for_each(move |change| {
                //     match change {
                //         VecDiff::Replace { values } => {
                //             for child in self.children.drain(..) {
                //                 element.remove(&child.component);
                //             }
                //             self.children.clone_from(&values);

                //             for child in self.children.iter() {
                //                 self.element.append(&child.component);
                //             }
                //         }
                //         _ => {} // VecDiff::InsertAt { index, value } => {
                //                 //     let mut i = 0;
                //                 //     let mut some_child = element.first_child();
                //                 //     let mut found = None;
                //                 //     while let Some(child) = some_child {
                //                 //         if i == index {
                //                 //             found = Some(child);
                //                 //             break;
                //                 //         }
                //                 //         i += 1;
                //                 //         some_child = child.next_sibling();
                //                 //     }

                //                 //     element.insert_before(&value, found.as_ref());
                //                 // }
                //                 // VecDiff::UpdateAt { index, value } => {
                //                 //     let mut i = 0;
                //                 //     let mut some_child = element.first_child();
                //                 //     let mut found = None;
                //                 //     while let Some(child) = some_child {
                //                 //         if i == index {
                //                 //             found = Some(child);
                //                 //             break;
                //                 //         }
                //                 //         i += 1;
                //                 //         some_child = child.next_sibling();
                //                 //     }

                //                 //     element.insert_before(&value, found.as_ref());
                //                 //     if let Some(found) = found {
                //                 //         element.remove(&found);
                //                 //     }
                //                 // }
                //                 // VecDiff::RemoveAt { index } => {
                //                 //     let mut i = 0;
                //                 //     let mut some_child = element.first_child();
                //                 //     let mut found = None;
                //                 //     while let Some(child) = some_child {
                //                 //         if i == index {
                //                 //             found = Some(child);
                //                 //             break;
                //                 //         }
                //                 //         i += 1;
                //                 //         some_child = child.next_sibling();
                //                 //     }

                //                 //     if let Some(found) = found {
                //                 //         element.remove(&found);
                //                 //     }
                //                 // }
                //                 // VecDiff::Move {
                //                 //     old_index,
                //                 //     new_index,
                //                 // } => {
                //                 //     let mut i = 0;
                //                 //     let mut some_child = element.first_child();
                //                 //     let mut found = None;
                //                 //     while let Some(child) = some_child {
                //                 //         if i == old_index {
                //                 //             found = Some(child);
                //                 //             break;
                //                 //         }
                //                 //         i += 1;
                //                 //         some_child = child.next_sibling();
                //                 //     }

                //                 //     let mut i = 0;
                //                 //     let mut some_child = element.first_child();
                //                 //     let mut target = None;
                //                 //     while let Some(child) = some_child {
                //                 //         if i == new_index {
                //                 //             target = Some(child);
                //                 //             break;
                //                 //         }
                //                 //         i += 1;
                //                 //         some_child = child.next_sibling();
                //                 //     }

                //                 //     if let Some(found) = found {
                //                 //         if let Some(target) = target {
                //                 //             element.remove(&found);
                //                 //             element.insert_before(&found, Some(&target));
                //                 //         }
                //                 //     }
                //                 // }
                //                 // VecDiff::Push { value } => element.append(&value),
                //                 // VecDiff::Pop {} => {
                //                 //     if let Some(last_child) = element.last_child() {
                //                 //         element.remove(&last_child);
                //                 //     }
                //                 // }
                //                 // VecDiff::Clear {} => {
                //                 //     let mut some_child = element.first_child();
                //                 //     while let Some(child) = some_child {
                //                 //         element.remove(&child);

                //                 //         some_child = child.next_sibling();
                //                 //     }
                //                 // }
                //     }

                //     async {}
                // }));
            }
        }

        self
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

pub enum VecValue<T, S: SignalVec<Item = T>> {
    Simple(Vec<T>),
    Signal(S),
}

pub enum Value<T, S: Signal<Item = T>> {
    Simple(T),
    Signal(S),
}

pub struct FakeSignal<T> {
    value: T,
}

impl<T> Signal for FakeSignal<T> {
    type Item = T;

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        Poll::Ready(None)
    }
}

pub fn simple<T>(value: T) -> Value<T, FakeSignal<T>> {
    Value::Simple(value)
}

pub fn signal<T, S: Signal<Item = T>>(value: S) -> Value<T, S> {
    Value::Signal(value)
}

#[derive(Default, Clone)]
pub struct Label {
    label: gtk::Label,
    pub(crate) handlers: Vec<Handle<()>>,
}

impl From<Label> for Node {
    fn from(value: Label) -> Self {
        Node {
            component: value.label.into(),
            handlers: value.handlers,
        }
    }
}

impl Label {
    pub fn text<A: AsStr, S: Signal<Item = A> + 'static>(mut self, text: Value<A, S>) -> Self {
        match text {
            Value::Simple(text) => {
                text.with_str(|s| {
                    self.label.set_text(s);
                });
            }
            Value::Signal(text) => {
                let element = self.label.clone();

                let h = crate::spawner::spawn(text.for_each(move |text| {
                    text.with_str(|s| {
                        println!("Setting text to {}", s);
                        element.set_text(s);
                    });

                    async {}
                }));

                self.handlers.push(h);
            }
        };

        self
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

    use super::{Label, Value};

    // use super::{label, Label, UIBuilder};

    #[test]
    fn apply() {
        let f = Mutable::new("ciao");
        let fff = f.signal();

        Label::default().text(Value::Signal(fff));
    }
}
