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

mod conditional_send {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            /// Use [`ConditionalSend`] to mark an optional Send trait bound. Useful as on certain platforms (eg. Wasm),
            /// futures aren't Send.
            pub trait ConditionalSend {}
            impl<T> ConditionalSend for T {}
        } else {
            /// Use [`ConditionalSend`] to mark an optional Send trait bound. Useful as on certain platforms (eg. Wasm),
            /// futures aren't Send.
            pub trait ConditionalSend: Send {}
            impl<T: Send> ConditionalSend for T {}
        }
    }
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

#[cfg(not(feature = "async_executor"))]
mod edge_executor;

mod executor;

mod slice;
pub use slice::{ParallelSlice, ParallelSliceMut};

#[cfg_attr(all(target_arch = "wasm32", feature = "web"), path = "wasm_task.rs")]
mod task;

pub use task::Task;

cfg_if::cfg_if! {
    if #[cfg(all(not(target_arch = "wasm32"), feature = "multi_threaded"))] {
        mod task_pool;
        mod thread_executor;

        pub use task_pool::{Scope, TaskPool, TaskPoolBuilder};
        pub use thread_executor::{ThreadExecutor, ThreadExecutorTicker};
    } else if #[cfg(any(target_arch = "wasm32", not(feature = "multi_threaded")))] {
        mod single_threaded_task_pool;

        pub use single_threaded_task_pool::{Scope, TaskPool, TaskPoolBuilder, ThreadExecutor};
    }
}

mod usages;
pub use futures_lite::future::poll_once;
pub use usages::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool};

#[cfg(not(all(target_arch = "wasm32", feature = "web")))]
pub use usages::tick_global_task_pools_on_main_thread;

#[cfg(feature = "std")]
cfg_if::cfg_if! {
    if #[cfg(feature = "async-io")] {
        pub use async_io::block_on;
    } else {
        pub use futures_lite::future::block_on;
    }
}

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

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
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
    } else {
        /// Gets the logical CPU core count available to the current process.
        ///
        /// This will always return at least 1.
        pub fn available_parallelism() -> usize {
            // Without access to std, assume a single thread is available
            1
        }
    }
}
