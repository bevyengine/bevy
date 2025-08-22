#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]
#![no_std]

/// Configuration information for this crate.
pub mod cfg {
    pub(crate) use bevy_platform::cfg::*;

    pub use bevy_platform::cfg::{alloc, std, web};

    define_alias! {
        #[cfg(feature = "bevy_executor")] => {
            /// Indicates `bevy_executor` is used as the future execution backend.
            bevy_executor
        }

        #[cfg(all(not(target_arch = "wasm32"), feature = "multi_threaded"))] => {
            /// Indicates multithreading support.
            multi_threaded
        }

        #[cfg(target_arch = "wasm32")] => {
            /// Indicates the current target requires additional `Send` bounds.
            conditional_send
        }

        #[cfg(feature = "async-io")] => {
            /// Indicates `async-io` will be used for the implementation of `block_on`.
            async_io
        }

        #[cfg(feature = "futures-lite")] => {
            /// Indicates `futures-lite` will be used for the implementation of `block_on`.
            futures_lite
        }
    }
}

cfg::std! {
    extern crate std;
}

extern crate alloc;

cfg::conditional_send! {
    if {
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

/// Use [`ConditionalSendFuture`] for a future with an optional Send trait bound, as on certain platforms (eg. Wasm),
/// futures aren't Send.
pub trait ConditionalSendFuture: Future + ConditionalSend {}

impl<T: Future + ConditionalSend> ConditionalSendFuture for T {}

use core::marker::PhantomData;

use alloc::boxed::Box;

/// An owned and dynamically typed Future used when you can't statically type your result or need to add some indirection.
pub type BoxedFuture<'a, T> = core::pin::Pin<Box<dyn ConditionalSendFuture<Output = T> + 'a>>;

// Modules
pub mod futures;
mod iter;
mod slice;
mod task;
mod usages;

cfg::bevy_executor! {
    if {
        mod bevy_executor;
    } else {
        mod edge_executor;
    }
}

// Exports
pub use iter::ParallelIterator;
pub use slice::{ParallelSlice, ParallelSliceMut};
pub use task::Task;

pub use futures_lite;
pub use futures_lite::future::poll_once;

cfg::web! {
    if {} else {
        pub use usages::tick_global_task_pools_on_main_thread;
    }
}

cfg::multi_threaded! {
    if {
        mod task_pool;

        pub use task_pool::{Scope, TaskPool, TaskPoolBuilder, ThreadSpawner};
    } else {
        mod single_threaded_task_pool;

        pub use single_threaded_task_pool::{Scope, TaskPool, TaskPoolBuilder, ThreadSpawner};
    }
}

cfg::switch! {
    cfg::async_io => {
        pub use async_io::block_on;
    }
    cfg::futures_lite => {
        pub use futures_lite::future::block_on;
    }
    _ => {
        /// Blocks on the supplied `future`.
        /// This implementation will busy-wait until it is completed.
        /// Consider enabling the `async-io` or `futures-lite` features.
        pub fn block_on<T>(future: impl Future<Output = T>) -> T {
            use core::task::{Poll, Context};

            // Pin the future on the stack.
            let mut future = core::pin::pin!(future);

            // We don't care about the waker as we're just going to poll as fast as possible.
            let waker = futures::noop_waker();
            let cx = &mut Context::from_waker(&waker);

            // Keep polling until the future is ready.
            loop {
                match future.as_mut().poll(cx) {
                    Poll::Ready(output) => return output,
                    Poll::Pending => core::hint::spin_loop(),
                }
            }
        }
    }
}

/// The tasks prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        block_on,
        iter::ParallelIterator,
        slice::{ParallelSlice, ParallelSliceMut},
    };
}

/// Gets the logical CPU core count available to the current process.
///
/// This is identical to `std::thread::available_parallelism`, except
/// it will return a default value of 1 if it internally errors out.
///
/// This will always return at least 1.
pub fn available_parallelism() -> usize {
    cfg::switch! {{
        cfg::std => {
            std::thread::available_parallelism()
                .map(core::num::NonZero::<usize>::get)
                .unwrap_or(1)
        }
        _ => {
            1
        }
    }}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum TaskPriority {
    BlockingIO,
    BlockingCompute,
    AsyncIO,
    #[default]
    Compute,
    RunNow,
}

impl TaskPriority {
    const MAX: usize = TaskPriority::RunNow as u8 as usize + 1;

    #[inline]
    fn to_index(self) -> usize {
        self as u8 as usize
    }
}

#[derive(Debug, Default)]
pub(crate) struct Metadata {
    pub priority: TaskPriority,
    pub is_send: bool,
}

pub struct TaskBuilder<'a, T> {
    pub(crate) task_pool: &'a TaskPool,
    pub(crate) priority: TaskPriority,
    marker_: PhantomData<*const T>,
}

impl<'a, T> TaskBuilder<'a, T> {
    pub(crate) fn new(task_pool: &'a TaskPool) -> Self {
        Self {
            task_pool,
            priority: TaskPriority::default(),
            marker_: PhantomData,
        }
    }

    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    pub(crate) fn build_metadata(self) -> Metadata {
        Metadata {
            priority: self.priority,
            is_send: false,
        }
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub enum ScopeTaskTarget {
    #[default]
    Any,
    /// Spawns a scoped future onto the thread the scope is run on. The scope *must* outlive
    /// the provided future. The results of the future will be returned as a part of
    /// [`TaskPool::scope`]'s return value.  Users should generally prefer to use
    /// [`Scope::spawn`] instead, unless the provided future needs to run on the scope's thread.
    ///
    /// For more information, see [`TaskPool::scope`].
    Scope,

    /// Spawns a scoped future onto the thread of the external thread executor.
    /// This is typically the main thread. The scope *must* outlive
    /// the provided future. The results of the future will be returned as a part of
    /// [`TaskPool::scope`]'s return value.  Users should generally prefer to use
    /// [`Scope::spawn`] instead, unless the provided future needs to run on the external thread.
    ///
    /// For more information, see [`TaskPool::scope`].
    External,
}

pub struct ScopeTaskBuilder<'a, 'scope, 'env: 'scope, T> {
    scope: &'a Scope<'scope, 'env, T>,
    priority: TaskPriority,
    target: ScopeTaskTarget,
}

impl<'a, 'scope, 'env, T> ScopeTaskBuilder<'a, 'scope, 'env, T> {
    pub(crate) fn new(scope: &'a Scope<'scope, 'env, T>) -> Self {
        Self {
            scope,
            priority: TaskPriority::default(),
            target: ScopeTaskTarget::default(),
        }
    }

    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_target(mut self, target: ScopeTaskTarget) -> Self {
        self.target = target;
        self
    }

    pub(crate) fn build_metadata(self) -> Metadata {
        Metadata {
            priority: self.priority,
            is_send: false,
        }
    }
}
