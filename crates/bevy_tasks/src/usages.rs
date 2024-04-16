use super::TaskPoolBuilder;
use crate::StaticTaskPool;

macro_rules! taskpool {
    ($(#[$attr:meta])* ($static:ident, $type:ident)) => {
        static $static: $type = $type(StaticTaskPool::new());

        $(#[$attr])*
        #[derive(Debug)]
        pub struct $type(StaticTaskPool);

        impl $type {
            #[doc = concat!(" Gets the global [`", stringify!($type), "`] instance.")]
            pub fn get() -> &'static StaticTaskPool {
                &$static.0
            }

            /// Gets the global instance, or initializes it with the provided builder if
            /// it hasn't already been initialized.
            pub fn get_or_init(builder: TaskPoolBuilder) -> &'static StaticTaskPool {
                let pool = &$static.0;
                if pool.is_initialized() {
                    pool.init(builder);
                }
                &$static.0
            }

            /// Gets the global instance, or initializes it with the default configuration if
            /// it hasn't already been initialized.
            pub fn get_or_default() -> &'static StaticTaskPool {
                let pool = &$static.0;
                if pool.is_initialized() {
                    pool.init(Default::default());
                }
                &$static.0
            }
        }
    };
}

taskpool! {
    /// A newtype for a task pool for CPU-intensive work that must be completed to
    /// deliver the next frame
    ///
    /// See [`TaskPool`] documentation for details on Bevy tasks.
    /// [`AsyncComputeTaskPool`] should be preferred if the work does not have to be
    /// completed before the next frame.
    (COMPUTE_TASK_POOL, ComputeTaskPool)
}

taskpool! {
    /// A newtype for a task pool for CPU-intensive work that may span across multiple frames
    ///
    /// See [`TaskPool`] documentation for details on Bevy tasks.
    /// Use [`ComputeTaskPool`] if the work must be complete before advancing to the next frame.
    (ASYNC_COMPUTE_TASK_POOL, AsyncComputeTaskPool)
}

taskpool! {
    /// A newtype for a task pool for IO-intensive work (i.e. tasks that spend very little time in a
    /// "woken" state)
    ///
    /// See [`TaskPool`] documentation for details on Bevy tasks.
    (IO_TASK_POOL, IoTaskPool)
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
        .0
        .with_local_executor(|compute_local_executor| {
            ASYNC_COMPUTE_TASK_POOL
                .0
                .with_local_executor(|async_local_executor| {
                    IO_TASK_POOL.0.with_local_executor(|io_local_executor| {
                        for _ in 0..100 {
                            compute_local_executor.try_tick();
                            async_local_executor.try_tick();
                            io_local_executor.try_tick();
                        }
                    });
                });
        });
}
