use super::TaskPool;
use std::{ops::Deref, sync::OnceLock};

macro_rules! taskpool {
    ($(#[$attr:meta])* ($static:ident, $type:ident)) => {
        static $static: OnceLock<$type> = OnceLock::new();

        $(#[$attr])*
        #[derive(Debug)]
        pub struct $type(TaskPool);

        impl $type {
            #[doc = concat!(" Gets the global [`", stringify!($type), "`] instance, or initializes it with `f`.")]
            pub fn get_or_init(f: impl FnOnce() -> TaskPool) -> &'static Self {
                $static.get_or_init(|| Self(f()))
            }

            #[doc = concat!(" Attempts to get the global [`", stringify!($type), "`] instance, \
                or returns `None` if it is not initialized.")]
            pub fn try_get() -> Option<&'static Self> {
                $static.get()
            }

            #[doc = concat!(" Gets the global [`", stringify!($type), "`] instance.")]
            #[doc = ""]
            #[doc = " # Panics"]
            #[doc = " Panics if the global instance has not been initialized yet."]
            pub fn get() -> &'static Self {
                $static.get().expect(
                    concat!(
                        "The ",
                        stringify!($type),
                        " has not been initialized yet. Please call ",
                        stringify!($type),
                        "::get_or_init beforehand."
                    )
                )
            }
        }

        impl Deref for $type {
            type Target = TaskPool;

            fn deref(&self) -> &Self::Target {
                &self.0
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
