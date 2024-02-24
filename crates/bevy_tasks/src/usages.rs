use super::TaskPool;
use std::{ops::Deref, sync::OnceLock};

static COMPUTE_TASK_POOL: OnceLock<ComputeTaskPool> = OnceLock::new();

/// A newtype for a task pool for CPU-intensive work that must be completed to
/// deliver the next frame
///
/// See [`TaskPool`] documentation for details on Bevy tasks.
#[derive(Debug)]
pub struct ComputeTaskPool(TaskPool);

impl ComputeTaskPool {
    /// Gets the global [`ComputeTaskPool`] instance, or initializes it with `f`.
    pub fn get_or_init(f: impl FnOnce() -> TaskPool) -> &'static Self {
        COMPUTE_TASK_POOL.get_or_init(|| Self(f()))
    }

    /// Attempts to get the global [`ComputeTaskPool`] instance, 
    /// or returns `None` if it is not initialized.
    pub fn try_get() -> Option<&'static Self> {
        COMPUTE_TASK_POOL.get()
    }

    /// Gets the global [`ComputeTaskPool`] instance."
    /// 
    /// # Panics
    /// Panics if the global instance has not been initialized yet.
    pub fn get() -> &'static Self {
        COMPUTE_TASK_POOL.get().expect(
                "The ComputeTaskPool has not been initialized yet. Please call ComputeTaskPool::get_or_init beforehand"
        )
    }
}

impl Deref for ComputeTaskPool {
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
            for _ in 0..100 {
                compute_local_executor.try_tick();
            }
        });
}
