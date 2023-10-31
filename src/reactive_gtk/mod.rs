use futures_signals::signal::{Signal, SignalExt};

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

pub enum Subscription {
    Static(AsyncContext),
    Dynamic(Handle<()>),
}

pub trait MaybeSignal<T> {
    fn subscribe<F>(self, f: F) -> Option<Handle<()>>
    where
        F: FnMut(T) + 'static;

    fn subscribe_with_ctx<F>(self, f: F) -> Subscription
    where
        F: FnMut(T, &mut AsyncContext) + 'static;
}

impl<T> MaybeSignal<T> for T {
    fn subscribe<F>(self, mut f: F) -> Option<Handle<()>>
    where
        F: FnMut(T),
    {
        f(self);

        None
    }

    fn subscribe_with_ctx<F>(self, mut f: F) -> Subscription
    where
        F: FnMut(T, &mut AsyncContext),
    {
        let mut ctx = AsyncContext::default();
        f(self, &mut ctx);

        Subscription::Static(ctx)
    }
}

impl<T, S: Signal<Item = T> + 'static> MaybeSignal<T> for Dynamic<T, S> {
    fn subscribe<F>(self, mut f: F) -> Option<Handle<()>>
    where
        F: FnMut(T) + 'static,
    {
        Some(spawn({
            self.0.for_each(move |value| {
                f(value);

                async {}
            })
        }))
    }

    fn subscribe_with_ctx<F>(self, mut f: F) -> Subscription
    where
        F: FnMut(T, &mut AsyncContext) + 'static,
    {
        Subscription::Dynamic(spawn({
            let mut ctx = AsyncContext::default();
            self.0.for_each(move |value| {
                f(value, &mut ctx);

                async {}
            })
        }))
    }
}

#[derive(Default, Debug)]
pub struct AsyncContext(Vec<Handle<()>>);

impl AsyncContext {
    pub fn forget(&mut self) {
        self.0.clear();
    }

    fn add_subscription(&mut self, handle: Handle<()>) {
        self.0.push(handle);
    }

    pub fn consume(&mut self, ctx: &mut AsyncContext) {
        for handle in ctx.0.drain(..) {
            self.0.push(handle);
        }
    }

    pub fn cancel(&mut self) {
        for handle in self.0.drain(..) {
            handle.cancel();
        }
    }
}

impl Drop for AsyncContext {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[macro_export]
macro_rules! nodes {
    ( $( $child:expr ),* ) => {
        vec!($( $child.into() ),*)
    };
}
