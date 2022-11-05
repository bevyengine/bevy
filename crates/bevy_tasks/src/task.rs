use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// A group a task is assigned to upon being spawned.
///
/// By default, `Compute` is used for [`TaskPool::spawn`].
///
/// [`TaskPool::spawn`]: crate::TaskPool::spawn
#[derive(Clone, Copy, Debug)]
pub enum TaskGroup {
    /// CPU-bound, short-lived, latency-sensitive tasks. Does not need
    /// to yield regularly. Should not hold the thread indefinitely or at the
    /// very minimum should not hold a thread longer than the course of a frame.
    Compute,
    /// IO-bound, potentially long lasting tasks that readily yield any incoming or
    /// outbound communication Usually used for loading assets or network communication.
    ///
    /// If IO threads are sitting idle, they may run `Compute` tasks if the compute threads
    /// are at capacity.
    IO,
    /// CPU-bound, long-lived tasks. Can hold the thread for very long periods (longer than
    /// a single frame).
    ///
    /// If async compute threads are sitting idle, they may run `Compute` or `IO` tasks if the
    /// respective threads are at capacity.
    AsyncCompute,
}

impl TaskGroup {
    // This is unused on wasm32 platforms.
    #[allow(dead_code)]
    pub(crate) const MAX_PRIORITY: usize = Self::Compute.to_priority() + 1;

    // This is unused on wasm32 platforms.
    pub(crate) const fn to_priority(self) -> usize {
        match self {
            Self::AsyncCompute => 0,
            Self::IO => 1,
            Self::Compute => 2,
        }
    }
}

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
pub struct Task<T>(async_task::Task<T>);

impl<T> Task<T> {
    /// Creates a new task from a given `async_executor::Task`
    pub fn new(task: async_task::Task<T>) -> Self {
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

    /// Returns `true` if the current task is finished.
    ///
    ///
    /// Unlike poll, it doesn't resolve the final value, it just checks if the task has finished.
    /// Note that in a multithreaded environment, this task can be finished immediately after calling this function.
    pub fn is_finished(&self) -> bool {
        self.0.is_finished()
    }
}

impl<T> Future for Task<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx)
    }
}
