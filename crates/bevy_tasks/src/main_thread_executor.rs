use std::{
    marker::PhantomData,
    sync::Arc,
    thread::{self, ThreadId},
};

use async_executor::{Executor, Task};
use futures_lite::Future;

/// Use to access the global main thread executor. Be aware that the main thread executor
/// only makes progress when it is ticked. This normally happens in `[TaskPool::scope]`.
#[derive(Debug)]
pub struct ThreadExecutor {
    // this is only pub crate for testing purposes, do not contruct otherwise
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
    /// Initializes the global `[MainThreadExecutor]` instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the `[MainThreadSpawner]` for the global main thread executor.
    /// Use this to spawn tasks on the main thread.
    pub fn spawner(&self) -> MainThreadSpawner<'static> {
        MainThreadSpawner(self.executor.clone())
    }

    /// Gets the `[MainThreadTicker]` for this executor.
    /// Use this to tick the executor.
    /// It only returns the ticker if it's on the thread the executor was created on
    /// and returns `None` otherwise.
    pub fn ticker(&self) -> Option<MainThreadTicker> {
        if thread::current().id() == self.thread_id {
            return Some(MainThreadTicker {
                executor: self.executor.clone(),
                _marker: PhantomData::default(),
            });
        }
        None
    }
}

#[derive(Debug)]
pub struct MainThreadSpawner<'a>(Arc<Executor<'a>>);
impl<'a> MainThreadSpawner<'a> {
    /// Spawn a task on the main thread
    pub fn spawn<T: Send + 'a>(&self, future: impl Future<Output = T> + Send + 'a) -> Task<T> {
        self.0.spawn(future)
    }
}

#[derive(Debug)]
pub struct MainThreadTicker {
    executor: Arc<Executor<'static>>,
    // make type not send or sync
    _marker: PhantomData<*const ()>,
}
impl MainThreadTicker {
    /// Tick the main thread executor.
    /// This needs to be called manually on the main thread if a `[TaskPool::scope]` is not active
    pub fn tick(&self) -> impl Future<Output = ()> + '_ {
        self.executor.tick()
    }
}
