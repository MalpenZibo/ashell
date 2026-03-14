use iced::futures::{
    Stream,
    task::{Context, Poll},
};
use pin_project_lite::pin_project;
use std::{pin::Pin, time::Duration};
use tokio::time::{self, Sleep};

pin_project! {
    pub struct Throttle<S: Stream> {
        #[pin]
        inner: S,
        duration: Duration,
        sleep: Option<Pin<Box<Sleep>>>,
        pending: Option<S::Item>,
    }
}

impl<S: Stream> Throttle<S> {
    pub fn new(inner: S, duration: Duration) -> Self {
        Self {
            inner,
            duration,
            sleep: None,
            pending: None,
        }
    }
}

impl<S> Stream for Throttle<S>
where
    S: Stream,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        // Always drain the inner stream, keeping only the latest value
        loop {
            match this.inner.as_mut().poll_next(cx) {
                Poll::Ready(Some(item)) => *this.pending = Some(item),
                Poll::Ready(None) => {
                    return Poll::Ready(this.pending.take());
                }
                Poll::Pending => break,
            }
        }

        // If we're in the throttling period, poll the sleep
        if let Some(sleep) = &mut this.sleep {
            match sleep.as_mut().poll(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(_) => *this.sleep = None, // Throttle period over
            }
        }

        // Emit pending value if we have one
        if let Some(item) = this.pending.take() {
            *this.sleep = Some(Box::pin(time::sleep(*this.duration)));
            return Poll::Ready(Some(item));
        }

        Poll::Pending
    }
}

pub trait ThrottleExt: Stream + Sized {
    fn throttle(self, duration: Duration) -> Throttle<Self> {
        Throttle::new(self, duration)
    }
}

impl<T: Stream> ThrottleExt for T {}
