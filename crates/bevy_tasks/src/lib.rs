#![warn(missing_docs)]
#![allow(clippy::type_complexity)]
#![doc = include_str!("../README.md")]

mod slice;
pub use slice::{ParallelSlice, ParallelSliceMut};

mod task;
pub use task::Task;

#[cfg(any(not(target_arch = "wasm32"), not(feature = "browser")))]
mod task_pool;
#[cfg(any(not(target_arch = "wasm32"), not(feature = "browser")))]
pub use task_pool::{Scope, TaskPool, TaskPoolBuilder};

#[cfg(all(target_arch = "wasm32", feature = "browser"))]
mod single_threaded_task_pool;
#[cfg(all(target_arch = "wasm32", feature = "browser"))]
pub use single_threaded_task_pool::{Scope, TaskPool, TaskPoolBuilder, ThreadExecutor};

mod usages;
#[cfg(not(target_arch = "wasm32"))]
pub use usages::tick_global_task_pools_on_main_thread;
pub use usages::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool};

#[cfg(any(not(target_arch = "wasm32"), not(feature = "browser")))]
mod thread_executor;
#[cfg(any(not(target_arch = "wasm32"), not(feature = "browser")))]
pub use thread_executor::{ThreadExecutor, ThreadExecutorTicker};

mod iter;
pub use iter::ParallelIterator;

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        iter::ParallelIterator,
        slice::{ParallelSlice, ParallelSliceMut},
        usages::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool},
    };
}

use std::num::NonZeroUsize;

/// Gets the logical CPU core count available to the current process.
///
/// This is identical to [`std::thread::available_parallelism`], except
/// it will return a default value of 1 if it internally errors out.
///
/// This will always return at least 1.
pub fn available_parallelism() -> usize {
    std::thread::available_parallelism()
        .map(NonZeroUsize::get)
        .unwrap_or(1)
}
