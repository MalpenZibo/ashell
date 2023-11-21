use futures_signals::signal::{Signal, SignalExt, always, Always};
use std::{cell::RefCell, rc::Rc, borrow::Cow};

mod center_box;
mod container;
mod label;
mod node;
mod overlay;
mod scale;
mod scrolled_window;
mod separator;
mod spawn;

pub use center_box::*;
pub use container::*;
pub use label::*;
pub use node::*;
pub use overlay::*;
pub use scale::*;
pub use scrolled_window::*;
pub use separator::*;
pub use spawn::*;

pub struct Dynamic<T, S: Signal<Item = T>>(pub S);

pub trait IntoSignal<T> {
    type Output: Signal<Item = T> + 'static; 

    fn into_signal(self) -> Self::Output;
}

impl<T: 'static> IntoSignal<T> for T {
    type Output = Always<T>;

    fn into_signal(self) -> Self::Output {
        always(self)
    }
}

impl<T: 'static, S: Signal<Item = T> + 'static> IntoSignal<T> for Dynamic<T, S> {
    type Output = S;

    fn into_signal(self) -> Self::Output {
        self.0
    }
}

#[derive(Default, Debug)]
pub struct AsyncContext {
    handles: Vec<Handle<()>>,
    childrens: Vec<Rc<RefCell<AsyncContext>>>,
}

impl AsyncContext {
    fn subscribe<T, F>(&mut self, value: impl IntoSignal<T> + 'static, mut f: F)
    where
        F: FnMut(T) + 'static,
    {
        let handle = spawn({
            value.into_signal().for_each(move |value| {
                f(value);

                async {}
            })
        });

        self.handles.push(handle);
    }

    fn subscribe_with_ctx<T, F>(
        &mut self,
        value: impl IntoSignal<T> + 'static,
        mut f: F,
    ) where
        F: FnMut(T, &mut AsyncContext) + 'static,
    {
        self.childrens
            .push(Rc::new(RefCell::new(AsyncContext::default())));
        let ctx = self.childrens.last().unwrap();

        let handle = spawn({
            let ctx = ctx.clone();
            value.into_signal().for_each(move |value| {
                f(
                    value,
                    &mut ctx.try_borrow_mut().expect("Failed to borrow context"),
                );

                async {}
            })
        });

        self.handles.push(handle);
    }

    pub fn consume(&mut self, ctx: &mut AsyncContext) {
        for handle in ctx.handles.drain(..) {
            self.handles.push(handle);
        }

        for ctx in ctx.childrens.drain(..) {
            self.childrens.push(ctx);
        }
    }

    pub fn cancel(&mut self) {
        for handle in self.handles.drain(..) {
            handle.cancel();
        }

        for ctx in self.childrens.drain(..) {
            ctx.try_borrow_mut()
                .expect("Failed to borrow context")
                .cancel();
        }
    }
}

impl Drop for AsyncContext {
    fn drop(&mut self) {
        self.cancel();
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

#[macro_export]
macro_rules! nodes {
    ( $( $child:expr ),* ) => {
        vec!($( $child.into() ),*)
    };
}
