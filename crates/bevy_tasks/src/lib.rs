#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

mod local_executor;
mod slice;
pub use slice::{ParallelSlice, ParallelSliceMut};

mod task;
pub use task::{Task, TaskGroup};

#[cfg(not(target_arch = "wasm32"))]
mod executor;
#[cfg(not(target_arch = "wasm32"))]
mod task_pool;
#[cfg(not(target_arch = "wasm32"))]
pub use task_pool::{Scope, TaskPool, TaskPoolBuilder};

#[cfg(target_arch = "wasm32")]
mod single_threaded_task_pool;
#[cfg(target_arch = "wasm32")]
pub use single_threaded_task_pool::{Scope, TaskPool, TaskPoolBuilder};

mod task_pool_builder;

mod iter;
pub use iter::ParallelIterator;

#[allow(missing_docs)]
pub mod prelude {
    #[cfg(target_arch = "wasm32")]
    pub use crate::single_threaded_task_pool::TaskPool;
    #[cfg(not(target_arch = "wasm32"))]
    pub use crate::task_pool::TaskPool;
    #[doc(hidden)]
    pub use crate::{
        iter::ParallelIterator,
        slice::{ParallelSlice, ParallelSliceMut},
        TaskGroup,
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
