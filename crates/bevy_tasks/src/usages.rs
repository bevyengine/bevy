//! Definitions for a few common task pools that we want. Generally the determining factor for what
//! kind of work should go in each pool is latency requirements.
//!
//! For CPU-intensive work (tasks that generally spin until completion) we have a standard
//! [`ComputeTaskPool`]  and an [`AsyncComputeTaskPool`]. Work that does not need to be completed to
//! present the next frame should go to the [`AsyncComputeTaskPool`]
//!
//! For IO-intensive work (tasks that spend very little time in a "woken" state) we have an IO
//! task pool. The tasks here are expected to complete very quickly. Generally they should just
//! await receiving data from somewhere (i.e. disk) and signal other systems when the data is ready
//! for consumption. (likely via channels)

use super::TaskPool;
use once_cell::sync::OnceCell;
use std::ops::Deref;

static COMPUTE_TASK_POOL: OnceCell<ComputeTaskPool> = OnceCell::new();
static ASYNC_COMPUTE_TASK_POOL: OnceCell<AsyncComputeTaskPool> = OnceCell::new();
static IO_TASK_POOL: OnceCell<IoTaskPool> = OnceCell::new();

/// A newtype for a task pool for CPU-intensive work that must be completed to deliver the next
/// frame
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
