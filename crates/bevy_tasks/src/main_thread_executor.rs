use std::{marker::PhantomData, sync::Arc};

use async_executor::{Executor, Task};
use futures_lite::Future;
use is_main_thread::is_main_thread;
use once_cell::sync::OnceCell;

static MAIN_THREAD_EXECUTOR: OnceCell<MainThreadExecutor> = OnceCell::new();

/// Use to access the global main thread executor. Be aware that the main thread executor
/// only makes progress when it is ticked. This normally happens in `[TaskPool::scope]`.
#[derive(Debug)]
pub struct MainThreadExecutor(
    // this is only pub crate for testing purposes, do not contruct otherwise
    pub(crate) Arc<Executor<'static>>,
);

impl MainThreadExecutor {
    /// Initializes the global `[MainThreadExecutor]` instance.
    pub fn init() -> &'static Self {
        MAIN_THREAD_EXECUTOR.get_or_init(|| Self(Arc::new(Executor::new())))
    }

    /// Gets the global [`MainThreadExecutor`] instance.
    ///
    /// # Panics
    /// Panics if no executor has been initialized yet.
    pub fn get() -> &'static Self {
        MAIN_THREAD_EXECUTOR.get().expect(
            "A MainThreadExecutor has not been initialize yet. Please call \
                MainThreadExecutor::init beforehand",
        )
    }

    /// Gets the `[MainThreadSpawner]` for the global main thread executor.
    /// Use this to spawn tasks on the main thread.
    pub fn spawner(&self) -> MainThreadSpawner<'static> {
        MainThreadSpawner(self.0.clone())
    }

    /// Gets the `[MainThreadTicker]` for the global main thread executor.
    /// Use this to tick the main thread executor.
    /// Returns None if called on not the main thread.
    pub fn ticker(&self) -> Option<MainThreadTicker> {
        // always return ticker when testing to allow tests to run off main thread
        dbg!("hjj");
        #[cfg(test)]
        if true {
            dbg!("blah");
            return Some(MainThreadTicker {
                executor: self.0.clone(),
                _marker: PhantomData::default(),
            });
        }

        if let Some(is_main) = is_main_thread() {
            if is_main {
                return Some(MainThreadTicker {
                    executor: self.0.clone(),
                    _marker: PhantomData::default(),
                });
            }
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
