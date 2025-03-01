#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]
#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

#[cfg(not(any(feature = "async_executor", feature = "edge_executor")))]
compile_error!("Either of the `async_executor` or the `edge_executor` features must be enabled.");

#[cfg(not(target_arch = "wasm32"))]
mod conditional_send {
    /// Use [`ConditionalSend`] to mark an optional Send trait bound. Useful as on certain platforms (eg. Wasm),
    /// futures aren't Send.
    pub trait ConditionalSend: Send {}
    impl<T: Send> ConditionalSend for T {}
}

#[cfg(target_arch = "wasm32")]
#[expect(missing_docs, reason = "Not all docs are written yet (#3492).")]
mod conditional_send {
    pub trait ConditionalSend {}
    impl<T> ConditionalSend for T {}
}

pub use conditional_send::*;

/// Use [`ConditionalSendFuture`] for a future with an optional Send trait bound, as on certain platforms (eg. Wasm),
/// futures aren't Send.
pub trait ConditionalSendFuture: Future + ConditionalSend {}
impl<T: Future + ConditionalSend> ConditionalSendFuture for T {}

use alloc::boxed::Box;

/// An owned and dynamically typed Future used when you can't statically type your result or need to add some indirection.
pub type BoxedFuture<'a, T> = core::pin::Pin<Box<dyn ConditionalSendFuture<Output = T> + 'a>>;

pub mod futures;

#[cfg(any(feature = "async_executor", feature = "edge_executor"))]
mod executor;

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

#[cfg(all(feature = "async-io", feature = "std"))]
pub use async_io::block_on;
#[cfg(all(not(feature = "async-io"), feature = "std"))]
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
        iter::ParallelIterator,
        slice::{ParallelSlice, ParallelSliceMut},
        usages::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool},
    };

    #[cfg(feature = "std")]
    #[doc(hidden)]
    pub use crate::block_on;
}

#[cfg(feature = "std")]
use core::num::NonZero;

/// Gets the logical CPU core count available to the current process.
///
/// This is identical to [`std::thread::available_parallelism`], except
/// it will return a default value of 1 if it internally errors out.
///
/// This will always return at least 1.
#[cfg(feature = "std")]
pub fn available_parallelism() -> usize {
    std::thread::available_parallelism()
        .map(NonZero::<usize>::get)
        .unwrap_or(1)
}

/// Gets the logical CPU core count available to the current process.
///
/// This will always return at least 1.
#[cfg(not(feature = "std"))]
pub fn available_parallelism() -> usize {
    // Without access to std, assume a single thread is available
    1
}
