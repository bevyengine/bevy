use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use futures_lite::FutureExt;
use tokio::task::JoinHandle;

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
#[must_use = "Tasks are canceled when dropped, use `.detach()` to run them in the background."]
pub struct Task<T>(Option<JoinHandle<T>>);

impl<T> Task<T> {
    /// Creates a new task from a given `async_executor::Task`
    pub fn new(task: JoinHandle<T>) -> Self {
        Self(Some(task))
    }

    /// Detaches the task to let it keep running in the background. See
    /// `async_executor::Task::detach`
    pub fn detach(mut self) {
        drop(self.0.take());
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
    pub async fn cancel(mut self) -> Option<T> {
        self.0.take()?
            .await
            .ok()
    }

    /// Returns `true` if the current task is finished.
    ///
    ///
    /// Unlike poll, it doesn't resolve the final value, it just checks if the task has finished.
    /// Note that in a multithreaded environment, this task can be finished immediately after calling this function.
    pub fn is_finished(&self) -> bool {
        self.0.as_ref().map(|handle| handle.is_finished()).unwrap_or(true)
    }
}

impl<T> Future for Task<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(handle) = self.0.as_mut() {
            match handle.poll(cx) {
                Poll::Ready(Ok(result)) => Poll::Ready(result),
                Poll::Ready(Err(err)) => panic!("Task has failed: {}", err),
                Poll::Pending => Poll::Pending,
            }
        } else {
            unreachable!("Polling dropped task");
        }
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        if let Some(handle) = self.0.take() {
            handle.abort();
        }
    }
}