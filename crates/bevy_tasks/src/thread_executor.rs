use std::{
    marker::PhantomData,
    sync::Arc,
    thread::{self, ThreadId},
};

use async_executor::{Executor, Task};
use futures_lite::Future;

/// An executor that can only be ticked on the thread it was instantiated on. But
/// can spawn `Send` tasks from other threads.
///
/// # Example
/// ```rust
/// # use std::sync::{Arc, atomic::{AtomicI32, Ordering}};
/// use bevy_tasks::ThreadExecutor;
///
/// let thread_executor = ThreadExecutor::new();
/// let count = Arc::new(AtomicI32::new(0));
///
/// // create some owned values that can be moved into another thread
/// let thread_executor_clone = thread_executor.clone();
/// let count_clone = count.clone();
/// let thread_spawner = thread_executor.spawner();
///
/// std::thread::scope(|scope| {
///     scope.spawn(|| {
///         // we cannot get the ticker from another thread
///         let not_thread_ticker = thread_executor_clone.ticker();
///         assert!(not_thread_ticker.is_none());
///         
///         // but we can spawn tasks from another thread
///         thread_spawner.spawn(async move {
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
#[derive(Debug, Clone)]
pub struct ThreadExecutor<'a> {
    executor: Arc<Executor<'a>>,
    thread_id: ThreadId,
}

impl<'a> Default for ThreadExecutor<'a> {
    fn default() -> Self {
        Self {
            executor: Arc::new(Executor::new()),
            thread_id: thread::current().id(),
        }
    }
}

impl<'a> ThreadExecutor<'a> {
    /// create a new [`ThreadExecutor`]
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn a task on the thread executor
    pub fn spawn<T: Send + 'a>(&self, future: impl Future<Output = T> + Send + 'a) -> Task<T> {
        self.executor.spawn(future)
    }

    /// Gets the [`ThreadExecutorTicker`] for this executor.
    /// Use this to tick the executor.
    /// It only returns the ticker if it's on the thread the executor was created on
    /// and returns `None` otherwise.
    pub fn ticker(&self) -> Option<ThreadExecutorTicker<'a>> {
        if thread::current().id() == self.thread_id {
            return Some(ThreadExecutorTicker {
                executor: self.executor.clone(),
                _marker: PhantomData::default(),
            });
        }
        None
    }
}

/// Used to tick the [`ThreadExecutor`]. The executor does not
/// make progress unless it is manually ticked on the thread it was
/// created on.
#[derive(Debug)]
pub struct ThreadExecutorTicker<'a> {
    executor: Arc<Executor<'a>>,
    // make type not send or sync
    _marker: PhantomData<*const ()>,
}
impl<'a> ThreadExecutorTicker<'a> {
    /// Tick the thread executor.
    pub async fn tick(&self) {
        self.executor.tick().await;
    }

    /// Synchronously try to tick a task on the executor.
    /// Returns false if if does not find a task to tick.
    pub fn try_tick(&self) -> bool {
        self.executor.try_tick()
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

        std::thread::scope(|s| {
            s.spawn(|| {
                let ticker = executor.ticker();
                assert!(ticker.is_none());
            });
        });
    }
}
