use std::{
    marker::PhantomData,
    thread::{self, ThreadId},
};

use async_executor::{Executor, Task};
use futures_lite::Future;

/// An executor that can only be ticked on the thread it was instantiated on. But
/// can spawn `Send` tasks from other threads.
///
/// # Example
/// ```
/// # use std::sync::{Arc, atomic::{AtomicI32, Ordering}};
/// use bevy_tasks::ThreadExecutor;
///
/// let thread_executor = ThreadExecutor::new();
/// let count = Arc::new(AtomicI32::new(0));
///
/// // create some owned values that can be moved into another thread
/// let count_clone = count.clone();
///
/// std::thread::scope(|scope| {
///     scope.spawn(|| {
///         // we cannot get the ticker from another thread
///         let not_thread_ticker = thread_executor.ticker();
///         assert!(not_thread_ticker.is_none());
///         
///         // but we can spawn tasks from another thread
///         thread_executor.spawn(async move {
///             count_clone.fetch_add(1, Ordering::Relaxed);
///         }).detach();
///     });
/// });
///
/// // the tasks do not make progress unless the executor is manually ticked
/// assert_eq!(count.load(Ordering::Relaxed), 0);
///
/// // tick the ticker until task finishes
/// let thread_ticker = thread_executor.ticker().unwrap();
/// thread_ticker.try_tick();
/// assert_eq!(count.load(Ordering::Relaxed), 1);
/// ```
#[derive(Debug)]
pub struct ThreadExecutor<'task> {
    executor: Executor<'task>,
    thread_id: ThreadId,
}

impl<'task> Default for ThreadExecutor<'task> {
    fn default() -> Self {
        Self {
            executor: Executor::new(),
            thread_id: thread::current().id(),
        }
    }
}

impl<'task> ThreadExecutor<'task> {
    /// create a new [`ThreadExecutor`]
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn a task on the thread executor
    pub fn spawn<T: Send + 'task>(
        &self,
        future: impl Future<Output = T> + Send + 'task,
    ) -> Task<T> {
        self.executor.spawn(future)
    }

    /// Gets the [`ThreadExecutorTicker`] for this executor.
    /// Use this to tick the executor.
    /// It only returns the ticker if it's on the thread the executor was created on
    /// and returns `None` otherwise.
    pub fn ticker<'ticker>(&'ticker self) -> Option<ThreadExecutorTicker<'task, 'ticker>> {
        if thread::current().id() == self.thread_id {
            return Some(ThreadExecutorTicker {
                executor: self,
                _marker: PhantomData,
            });
        }
        None
    }

    /// Returns true if `self` and `other`'s executor is same
    pub fn is_same(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// Used to tick the [`ThreadExecutor`]. The executor does not
/// make progress unless it is manually ticked on the thread it was
/// created on.
#[derive(Debug)]
pub struct ThreadExecutorTicker<'task, 'ticker> {
    executor: &'ticker ThreadExecutor<'task>,
    // make type not send or sync
    _marker: PhantomData<*const ()>,
}
impl<'task, 'ticker> ThreadExecutorTicker<'task, 'ticker> {
    /// Tick the thread executor.
    pub async fn tick(&self) {
        self.executor.executor.tick().await;
    }

    /// Synchronously try to tick a task on the executor.
    /// Returns false if does not find a task to tick.
    pub fn try_tick(&self) -> bool {
        self.executor.executor.try_tick()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_ticker() {
        let executor = Arc::new(ThreadExecutor::new());
        let ticker = executor.ticker();
        assert!(ticker.is_some());

        thread::scope(|s| {
            s.spawn(|| {
                let ticker = executor.ticker();
                assert!(ticker.is_none());
            });
        });
    }
}
