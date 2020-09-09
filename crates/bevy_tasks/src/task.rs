use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// Wraps `multitask::Task`, a spawned future.
///
/// Tasks are also futures themselves and yield the output of the spawned future.
///
/// When a task is dropped, its gets canceled and won't be polled again. To cancel a task a bit
/// more gracefully and wait until it stops running, use the [`cancel()`][Task::cancel()] method.
///
/// Tasks that panic get immediately canceled. Awaiting a canceled task also causes a panic.
/// Wraps multitask::Task
pub struct Task<T>(multitask::Task<T>);

impl<T> Task<T> {
    /// Detaches the task to let it keep running in the background. See `multitask::Task::detach`
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
    /// See `multitask::Task::cancel`
    pub async fn cancel(self) -> Option<T> {
        self.0.cancel().await
    }
}

impl<T> Future for Task<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Safe because Task is pinned and contains multitask::Task by value
        unsafe { self.map_unchecked_mut(|x| &mut x.0).poll(cx) }
    }
}
