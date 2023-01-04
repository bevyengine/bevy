use std::{
    marker::PhantomData,
    sync::Arc,
    thread::{self, ThreadId},
};

use async_executor::{Executor, Task};
use futures_lite::Future;

/// An executor that can only be ticked on the thread it was instantiated on.
#[derive(Debug)]
pub struct ThreadExecutor {
    executor: Arc<Executor<'static>>,
    thread_id: ThreadId,
}

impl Default for ThreadExecutor {
    fn default() -> Self {
        Self {
            executor: Arc::new(Executor::new()),
            thread_id: thread::current().id(),
        }
    }
}

impl ThreadExecutor {
    /// createa a new `[ThreadExecutor]`
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the `[ThreadSpawner]` for the thread executor.
    /// Use this to spawn tasks that run on the thread this was instatiated on.
    pub fn spawner(&self) -> ThreadSpawner<'static> {
        ThreadSpawner(self.executor.clone())
    }

    /// Gets the `[ThreadTicker]` for this executor.
    /// Use this to tick the executor.
    /// It only returns the ticker if it's on the thread the executor was created on
    /// and returns `None` otherwise.
    pub fn ticker(&self) -> Option<ThreadTicker> {
        if thread::current().id() == self.thread_id {
            return Some(ThreadTicker {
                executor: self.executor.clone(),
                _marker: PhantomData::default(),
            });
        }
        None
    }
}

/// Used to spawn on the [`ThreadExecutor`]
#[derive(Debug)]
pub struct ThreadSpawner<'a>(Arc<Executor<'a>>);
impl<'a> ThreadSpawner<'a> {
    /// Spawn a task on the main thread
    pub fn spawn<T: Send + 'a>(&self, future: impl Future<Output = T> + Send + 'a) -> Task<T> {
        self.0.spawn(future)
    }
}

/// Used to tick the [`ThreadExecutor`]
#[derive(Debug)]
pub struct ThreadTicker {
    executor: Arc<Executor<'static>>,
    // make type not send or sync
    _marker: PhantomData<*const ()>,
}
impl ThreadTicker {
    /// Tick the main thread executor.
    /// This needs to be called manually on the thread if it is not being used with
    /// a `[TaskPool::scope]`.
    pub fn tick(&self) -> impl Future<Output = ()> + '_ {
        self.executor.tick()
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
