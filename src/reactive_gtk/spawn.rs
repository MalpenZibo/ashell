//! An executor to run futures in the idle cycles of a Glib/Gtk application.
//!
//! This crate provides a function that programs a future to be run using the
//! _idle_ time of the main loop, as provided by [`glib::idle_add`](../glib/source/fn.idle_add.html).
//!
//! It works by registering an idle function to poll the future once, and whenever the future is
//! awoken, it will register another idle function to be continued, and so on.
//!
//! The main advantage of this approach is that the future is run in the main loop.
//! As such, it does not need to be `Send`, it does not require any kind of synchronization
//! to access shared state, and it can even manipulate GUI objects directly.
//!
//! Naturally, you still need to be careful, because if you set up several idle futures at the same
//! time they will be run intermixed.
//!
//! # What can it be used for?
//!
//! You can use this crate to implement async I/O that handles the GUI without the need of
//! synchronization. For example you can download assets or updates from the internet, or provide
//! an API for automating your GUI.
//!
//! You can also use async functions to implement background tasks that run when the program is
//! idle. They can be interrupted at any time and they can update the GUI directly. A similar
//! technique can be used to easily build a progress bar for a long-running process. Just add
//! something like this call here and there:
//! ```
//! async_std::task::yield_now().await;
//! ```
//!
//! # Working with other async frameworks
//!
//! You can run almost any future as an idle future, but if it uses functionality from any async
//! framework (`async_std`, `smol`, `tokio`...) it will most likely need to be initialized first,
//! or else your futures will not advance. This is because these frameworks usually work by creating
//! a background thread that does the actual polling and wakes up the other futures. If this
//! background thread is not created, nothing will be polled and your futures will stall forever.
//!
//! You may think that interacting with other fraworks is going to be difficult. But actually it is
//! super easy, barely an inconvenience:
//!
//! ## `async_std`
//!
//! Spawning any async task will bootstrap the runtime, so just this from your `main` is enough:
//! ```
//! async_std::task::spawn(async {});
//! ```
//!
//! ## `smol`
//!
//! There must be at least one thread runing an async job, and it will do that and poll all the
//! futures. So the code would be something like this, that will spawn a never-ending future:
//! ```
//! std::thread::spawn(|| smol::run(futures::future::pending::<()>()));
//! ```
//! If you also use `async_std`, since it uses `smol` under the hood, you just need one of these.
//!
//! ## `tokio`
//!
//! In `tokio` futures need to be spawn from a tokio reactor. It is enough if you run your main loop from there:
//! ```
//!     let mut rt = tokio::runtime::Builder::new()
//!       .threaded_scheduler()
//!       .enable_all()
//!       .build()
//!       .unwrap();
//!    rt.block_on(async { gtk4::main() });
//!```
//!
//!Or you can decorate your `main` function with:
//!```
//!#[tokio::main]
//!async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!    //...
//!}
//!```

use futures::prelude::*;
use std::{
    cell::Cell,
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::Arc,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

//The raw pointer in the RawWaker will be a pointer to an Arc-allocated GIdleTask
struct GIdleTask {
    future: Cell<Option<Pin<Box<dyn Future<Output = ()> + 'static>>>>,
}

#[inline]
unsafe fn increment_arc_count(ptr: *const ()) {
    let rc = Arc::from_raw(ptr as *const GIdleTask);
    std::mem::forget(rc.clone());
    std::mem::forget(rc);
}
#[inline]
unsafe fn decrement_arc_count(ptr: *const ()) {
    Arc::from_raw(ptr as *const GIdleTask);
}

unsafe fn gwaker_clone(ptr: *const ()) -> RawWaker {
    increment_arc_count(ptr);
    RawWaker::new(ptr, &GWAKER_VTABLE)
}
unsafe fn gwaker_wake(ptr: *const ()) {
    //poll_idle consumes one reference count, as wake requires, so nothing to do
    poll_idle(ptr);
}
unsafe fn gwaker_wake_by_ref(ptr: *const ()) {
    //poll_idle consumes one reference count, so we have to increment it here one
    increment_arc_count(ptr);
    poll_idle(ptr);
}
unsafe fn gwaker_drop(ptr: *const ()) {
    decrement_arc_count(ptr);
}

static GWAKER_VTABLE: RawWakerVTable =
    RawWakerVTable::new(gwaker_clone, gwaker_wake, gwaker_wake_by_ref, gwaker_drop);

//Actually the inner future is not Send, but GIdleTask is private to this module, so if
//we are careful we can move it between threads, as long as we only use the future in the
//main thread.
unsafe impl Send for GIdleTask {}
unsafe impl Sync for GIdleTask {}

//poll_idle() can be called from an arbitrary thread, because Waker is Send,
//but once we are in the glib::source::idle_add() callback we are in the main loop.
//When it ends, Waker::drop decrements the counter for the Arc<GIdleTask>.
fn poll_idle(ptr: *const ()) {
    let task = unsafe { &*(ptr as *const GIdleTask) };
    glib::source::idle_add(move || {
        let raw = RawWaker::new(task as *const GIdleTask as *const (), &GWAKER_VTABLE);
        let waker = unsafe { Waker::from_raw(raw) };

        //It is unlikely but the call to poll could call gtk4::main_iteration() or similar and
        //reenter the this idle call. We avoid reentering the future by taking it from the
        //GIdleTask and storing it later if it returns pending.
        //This has the additional advantage that the future is automatically fused.
        let mut op_future = task.future.take();

        //If the future has finished, drop it, a home-made fuse.
        if let Some(future) = op_future.as_mut() {
            let mut ctx = Context::from_waker(&waker);
            match future.as_mut().poll(&mut ctx) {
                Poll::Ready(()) => {}
                Poll::Pending => {
                    task.future.set(op_future);
                }
            }
        }
        glib::ControlFlow::Break
    });
}

/// Spawns an idle future.
///
/// This function registers the given future to be run in the idle time of the main Glib loop.
///
/// It must be called from the main Glib loop or it will panic. Since the future will be run in the
/// same thread, it does not need to be `Send`.
///
/// The future can return any value, but it is discarded.
///
/// Currently there is no way to cancel a future. If you need that you can use `futures::future::AbortHandle`.
///
pub fn spawn<T, F>(future: F) -> Handle<T>
where
    T: Copy + 'static,
    F: Future<Output = T> + 'static,
{
    //Check that we are in the main loop
    assert!(
        glib::MainContext::default().is_owner(),
        "idle_spawn can only be called from the glib main loop"
    );

    let res: Rc<Cell<Option<T>>> = Rc::new(Cell::new(None));
    let task = Arc::new(GIdleTask {
        future: Cell::new(Some(Box::pin(future.map({
            let res = res.clone();
            move |t| res.set(Some(t))
        })))),
    });
    let wtask = Arc::downgrade(&task);
    let ptr = Arc::into_raw(task) as *const ();

    poll_idle(ptr);
    Handle { task: wtask, res }
}

/// A handle to a running idle future.
///
/// You can use this handle to cancel the future and to retrieve the return
/// value of a future that has finished.
///
/// If the Handle is dropped, the future will keep on running, and there will
/// be no way to cancel it or get the return value.
#[derive(Clone)]
pub struct Handle<T: Copy> {
    task: std::sync::Weak<GIdleTask>,
    res: Rc<Cell<Option<T>>>,
}

impl<T: Copy> std::fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handle")
            .field("task", &self.task)
            .finish()
    }
}

impl<T: Copy> Handle<T> {
    /// If the future has finished, it returns `Some(t)`.
    /// If it has not finished, the future is dropped and it returns `None`.
    pub fn cancel(self) -> Option<T> {
        if let Some(task) = self.task.upgrade() {
            task.future.take();
        }
        self.res.take()
    }
    /// Returns `true` if the future has finished, `false` otherwise.
    /// If it returns `true` then you can be sure that [`cancel`](#method.cancel) will return `Some(t)`.
    pub fn has_finished(&self) -> bool {
        match self.res.take() {
            None => false,
            Some(x) => {
                self.res.set(Some(x));
                true
            }
        }
    }
    /// Converts this handle into a future that is satisfied when the original job finishes.
    /// It returns the value returned by the original future.
    pub async fn future(self) -> T {
        if let Some(task) = self.task.upgrade() {
            if let Some(fut) = task.future.take() {
                fut.await;
            }
        }
        //This unwrap() cannot fail: if the future has finished, then self.res must be Some,
        //because the last action of the future is assigning to it.
        //And the future cannot be cancelled, because both Handle::cancel() and Handle::future()
        //consume self, thus only one of them can be used.
        self.res.take().unwrap()
    }
}


