use super::TaskPool;
use std::{ops::Deref, sync::OnceLock};

static COMPUTE_TASK_POOL: OnceLock<ComputeTaskPool> = OnceLock::new();
static ASYNC_COMPUTE_TASK_POOL: OnceLock<AsyncComputeTaskPool> = OnceLock::new();
static IO_TASK_POOL: OnceLock<IoTaskPool> = OnceLock::new();

/// A newtype for a task pool for CPU-intensive work that must be completed to
/// deliver the next frame
///
/// See [`TaskPool`] documentation for details on Bevy tasks.
/// [`AsyncComputeTaskPool`] should be preferred if the work does not have to be
/// completed before the next frame.
#[derive(Debug)]
pub struct ComputeTaskPool(TaskPool);

impl ComputeTaskPool {
    /// Initializes the global [`ComputeTaskPool`] instance.
    pub fn init(f: impl FnOnce() -> TaskPool) -> &'static Self {
        COMPUTE_TASK_POOL.get_or_init(|| Self(f()))
    }

    /// Gets the global [`ComputeTaskPool`] instance.
    ///
    /// # Panics
    /// Panics if no pool has been initialized yet.
    pub fn get() -> &'static Self {
        COMPUTE_TASK_POOL.get().expect(
            "A ComputeTaskPool has not been initialized yet. Please call \
                    ComputeTaskPool::init beforehand.",
        )
    }
}

impl Deref for ComputeTaskPool {
    type Target = TaskPool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A newtype for a task pool for CPU-intensive work that may span across multiple frames
///
/// See [`TaskPool`] documentation for details on Bevy tasks. Use [`ComputeTaskPool`] if
/// the work must be complete before advancing to the next frame.
#[derive(Debug)]
pub struct AsyncComputeTaskPool(TaskPool);

impl AsyncComputeTaskPool {
    /// Initializes the global [`AsyncComputeTaskPool`] instance.
    pub fn init(f: impl FnOnce() -> TaskPool) -> &'static Self {
        ASYNC_COMPUTE_TASK_POOL.get_or_init(|| Self(f()))
    }

    /// Gets the global [`AsyncComputeTaskPool`] instance.
    ///
    /// # Panics
    /// Panics if no pool has been initialized yet.
    pub fn get() -> &'static Self {
        ASYNC_COMPUTE_TASK_POOL.get().expect(
            "A AsyncComputeTaskPool has not been initialized yet. Please call \
                    AsyncComputeTaskPool::init beforehand.",
        )
    }
}

impl Deref for AsyncComputeTaskPool {
    type Target = TaskPool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A newtype for a task pool for IO-intensive work (i.e. tasks that spend very little time in a
/// "woken" state)
#[derive(Debug)]
pub struct IoTaskPool(TaskPool);

impl IoTaskPool {
    /// Initializes the global [`IoTaskPool`] instance.
    pub fn init(f: impl FnOnce() -> TaskPool) -> &'static Self {
        IO_TASK_POOL.get_or_init(|| Self(f()))
    }

    /// Gets the global [`IoTaskPool`] instance.
    ///
    /// # Panics
    /// Panics if no pool has been initialized yet.
    pub fn get() -> &'static Self {
        IO_TASK_POOL.get().expect(
            "A IoTaskPool has not been initialized yet. Please call \
                    IoTaskPool::init beforehand.",
        )
    }
}

impl Deref for IoTaskPool {
    type Target = TaskPool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A function used by `bevy_core` to tick the global tasks pools on the main thread.
/// This will run a maximum of 100 local tasks per executor per call to this function.
///
/// # Warning
///
/// This function *must* be called on the main thread, or the task pools will not be updated appropriately.
#[cfg(not(target_arch = "wasm32"))]
pub fn tick_global_task_pools_on_main_thread() {
    COMPUTE_TASK_POOL
        .get()
        .unwrap()
        .with_local_executor(|compute_local_executor| {
            ASYNC_COMPUTE_TASK_POOL
                .get()
                .unwrap()
                .with_local_executor(|async_local_executor| {
                    IO_TASK_POOL
                        .get()
                        .unwrap()
                        .with_local_executor(|io_local_executor| {
                            for _ in 0..100 {
                                compute_local_executor.try_tick();
                                async_local_executor.try_tick();
                                io_local_executor.try_tick();
                            }
                        });
                });
        });
}
