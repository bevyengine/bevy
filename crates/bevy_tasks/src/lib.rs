#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

extern crate alloc;

mod slice;
pub use slice::{ParallelSlice, ParallelSliceMut};

#[cfg_attr(target_arch = "wasm32", path = "wasm_task.rs")]
mod task;

pub use task::Task;

#[cfg(all(not(target_arch = "wasm32"), feature = "multi_threaded"))]
mod task_pool;

#[cfg(all(not(target_arch = "wasm32"), feature = "multi_threaded"))]
pub use task_pool::{Scope, TaskPool, TaskPoolBuilder};

#[cfg(any(target_arch = "wasm32", not(feature = "multi_threaded")))]
mod single_threaded_task_pool;

#[cfg(any(target_arch = "wasm32", not(feature = "multi_threaded")))]
pub use single_threaded_task_pool::{Scope, TaskPool, TaskPoolBuilder, ThreadExecutor};

mod usages;
#[cfg(not(target_arch = "wasm32"))]
pub use usages::tick_global_task_pools_on_main_thread;
pub use usages::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool};

#[cfg(all(not(target_arch = "wasm32"), feature = "multi_threaded"))]
mod thread_executor;
#[cfg(all(not(target_arch = "wasm32"), feature = "multi_threaded"))]
pub use thread_executor::{ThreadExecutor, ThreadExecutorTicker};

#[cfg(feature = "async-io")]
pub use async_io::block_on;
#[cfg(not(feature = "async-io"))]
pub use futures_lite::future::block_on;
pub use futures_lite::future::poll_once;

mod iter;
pub use iter::ParallelIterator;

pub use futures_lite;

/// The tasks prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        block_on,
        iter::ParallelIterator,
        slice::{ParallelSlice, ParallelSliceMut},
        usages::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool},
    };
}

use core::num::NonZero;

/// Gets the logical CPU core count available to the current process.
///
/// This is identical to [`std::thread::available_parallelism`], except
/// it will return a default value of 1 if it internally errors out.
///
/// This will always return at least 1.
pub fn available_parallelism() -> usize {
    std::thread::available_parallelism()
        .map(NonZero::<usize>::get)
        .unwrap_or(1)
}
