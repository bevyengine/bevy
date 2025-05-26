use alloc::boxed::Box;
use core::{
    any::Any,
    future::{Future, IntoFuture},
    panic::{AssertUnwindSafe, UnwindSafe},
    pin::Pin,
    task::{Context, Poll},
};

use futures_channel::oneshot;

/// Wraps an asynchronous task, a spawned future.
///
/// Tasks are also futures themselves and yield the output of the spawned future.
#[derive(Debug)]
pub struct Task<T>(oneshot::Receiver<Result<T, Panic>>);

impl<T: 'static> Task<T> {
    pub(crate) fn wrap_future(future: impl Future<Output = T> + 'static) -> Self {
        let (sender, receiver) = oneshot::channel();
        wasm_bindgen_futures::spawn_local(async move {
            // Catch any panics that occur when polling the future so they can
            // be propagated back to the task handle.
            let value = CatchUnwind(AssertUnwindSafe(future)).await;
            let _ = sender.send(value);
        });
        Self(receiver.into_future())
    }

    /// When building for Wasm, this method has no effect.
    /// This is only included for feature parity with other platforms.
    pub fn detach(self) {}

    /// Requests a task to be cancelled and returns a future that suspends until it completes.
    /// Returns the output of the future if it has already completed.
    ///
    /// # Implementation
    ///
    /// When building for Wasm, it is not possible to cancel tasks, which means this is the same
    /// as just awaiting the task. This method is only included for feature parity with other platforms.
    pub async fn cancel(self) -> Option<T> {
        match self.0.await {
            Ok(Ok(value)) => Some(value),
            Err(_) => None,
            Ok(Err(panic)) => {
                // drop this to prevent the panic payload from resuming the panic on drop.
                // this also leaks the box but I'm not sure how to avoid that
                core::mem::forget(panic);
                None
            }
        }
    }
}

impl<T> Future for Task<T> {
    type Output = T;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.0).poll(cx) {
            Poll::Ready(Ok(Ok(value))) => Poll::Ready(value),
            // NOTE: Propagating the panic here sorta has parity with the async_executor behavior.
            // For those tasks, polling them after a panic returns a `None` which gets `unwrap`ed, so
            // using `resume_unwind` here is essentially keeping the same behavior while adding more information.
            #[cfg(feature = "std")]
            Poll::Ready(Ok(Err(panic))) => std::panic::resume_unwind(panic),
            #[cfg(not(feature = "std"))]
            Poll::Ready(Ok(Err(_panic))) => {
                unreachable!("catching a panic is only possible with std")
            }
            Poll::Ready(Err(_)) => panic!("Polled a task after it was cancelled"),
            Poll::Pending => Poll::Pending,
        }
    }
}

type Panic = Box<dyn Any + Send + 'static>;

#[pin_project::pin_project]
struct CatchUnwind<F: UnwindSafe>(#[pin] F);

impl<F: Future + UnwindSafe> Future for CatchUnwind<F> {
    type Output = Result<F::Output, Panic>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let f = AssertUnwindSafe(|| self.project().0.poll(cx));

        #[cfg(feature = "std")]
        let result = std::panic::catch_unwind(f)?;

        #[cfg(not(feature = "std"))]
        let result = f();

        result.map(Ok)
    }
}
