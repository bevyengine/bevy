use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

/// Wraps `async_executor::Task`, a spawned future.
///
/// Tasks are also futures themselves and yield the output of the spawned future.
///
/// When a task is dropped, its gets canceled and won't be polled again. To cancel a task a bit
/// more gracefully and wait until it stops running, use the [`cancel()`][Task::cancel()] method.
///
/// Tasks that panic get immediately canceled. Awaiting a canceled task also causes a panic.
/// Wraps `async_executor::Task`
#[derive(Debug)]
pub struct Task<T>(async_executor::Task<T>);

impl<T> Task<T> {
    /// Creates a new task from a given `async_executor::Task`
    pub fn new(task: async_executor::Task<T>) -> Self {
        Self(task)
    }

    /// Detaches the task to let it keep running in the background. See
    /// `async_executor::Task::detach`
    pub fn detach(self) {
        self.0.detach();
    }

    /// Cancels the task and waits for it to stop running.
    ///
    /// Returns the task's output if it was completed just before it got canceled, or [`None`] if
    /// it didn't complete.
    ///
    /// While it's possible to simply drop the [`Task`] to cancel it, this is a cleaner way of
    /// canceling because it also waits for the task to stop running.
    ///
    /// See `async_executor::Task::cancel`
    pub async fn cancel(self) -> Option<T> {
        self.0.cancel().await
    }

    /// Check if the task has been completed, and if so, return the value.
    pub fn check(&mut self) -> Option<T> {
        const NOOP_WAKER_VTABLE: &RawWakerVTable = &RawWakerVTable::new(
            |_| RawWaker::new(std::ptr::null::<()>(), NOOP_WAKER_VTABLE),
            |_| {},
            |_| {},
            |_| {},
        );

        // SAFE: all vtable functions are always safe to call.
        let waker =
            unsafe { Waker::from_raw(RawWaker::new(std::ptr::null::<()>(), NOOP_WAKER_VTABLE)) };

        match Pin::new(&mut self.0).poll(&mut Context::from_waker(&waker)) {
            Poll::Ready(val) => Some(val),
            Poll::Pending => None,
        }
    }
}

impl<T> Future for Task<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx)
    }
}
