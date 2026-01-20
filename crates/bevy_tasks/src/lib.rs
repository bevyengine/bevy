#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
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
        #[cfg(feature = "async_executor")] => {
            /// Indicates `async_executor` is used as the future execution backend.
            async_executor
        }

        #[cfg(all(not(target_arch = "wasm32"), feature = "multi_threaded"))] => {
            /// Indicates multithreading support.
            multi_threaded
        }

        #[cfg(target_arch = "wasm32")] => {
            /// Indicates the current target requires additional `Send` bounds.
            conditional_send
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

use alloc::boxed::Box;

/// An owned and dynamically typed Future used when you can't statically type your result or need to add some indirection.
pub type BoxedFuture<'a, T> = core::pin::Pin<Box<dyn ConditionalSendFuture<Output = T> + 'a>>;

// Modules
mod executor;
pub mod futures;
mod iter;
mod slice;
mod task;
mod usages;

cfg::async_executor! {
    if {} else {
        mod edge_executor;
    }
}

// Exports
pub use iter::ParallelIterator;
pub use slice::{ParallelSlice, ParallelSliceMut};
pub use task::Task;
pub use usages::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool};

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
        mod thread_executor;

        pub use task_pool::{Scope, TaskPool, TaskPoolBuilder};
        pub use thread_executor::{ThreadExecutor, ThreadExecutorTicker};
    } else {
        mod single_threaded_task_pool;

        pub use single_threaded_task_pool::{Scope, TaskPool, TaskPoolBuilder, ThreadExecutor};
    }
}

pub use bevy_platform::future::block_on;

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

/// Represents different priority levels that can be assigned to a thread.
///
/// These priority levels are hints to the operating system’s scheduler. The actual behavior can vary based on the OS,
/// system load, and other factors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ThreadPriority {
    /// Background priority: For tasks that should only run when CPU is idle.
    ///
    /// # Platform Specifc Behavior
    ///
    ///  * **Linux:** Typically uses `SCHED_OTHER` policy with a high `nice` value (e.g., 19).
    Background,
    /// Lowest priority: For tasks that are not time-sensitive but more important than background.
    ///
    /// # Platform Specifc Behavior
    ///
    ///  * **Linux:** Typically uses `SCHED_OTHER` with a `nice` value (e.g., 15).
    Lowest,
    /// Below normal priority: For tasks that are less critical than normal operations.
    ///
    /// # Platform Specifc Behavior
    ///
    ///  * **Linux:** Typically uses `SCHED_OTHER` with a `nice` value (e.g., 10).
    BelowNormal,
    /// Normal priority: The default priority for most threads.
    ///
    /// # Platform Specifc Behavior
    ///
    ///  * **Linux:** Typically uses SCHED_OTHER with a `nice` value of 0.
    ///    spikes under heavy load.
    #[default]
    Normal,
    /// Above normal priority: For tasks that are more important than normal but not critical.
    ///
    /// # Platform Specifc Behavior
    ///
    ///  *  **Linux:** Typically uses `SCHED_OTHER` with a negative `nice` value (e.g., -5).
    AboveNormal,
    /// Highest priority: For critical tasks that are deadline-sensitive.
    ///
    /// # Platform Specifc Behavior
    ///
    ///  * **General:** Often maps to a real-time scheduling policy.
    ///  * **Linux:** Typically maps to `SCHED_RR` (Round Robin) with a high real-time priority.
    ///    Requires `CAP_SYS_NICE` capability or root privileges.
    Highest,
    /// Realtime priority: For extremely sensitive tasks requiring minimum latency.
    ///
    /// **Use with extreme caution.** This level gives threads the highest possible precedence and can potentially
    /// starve other system processes if not managed carefully.
    ///
    /// # Platform Specifc Behavior
    ///
    /// * **General:** Maps to the highest available real-time scheduling priority.
    /// * **Linux:* Typically maps to `SCHED_RR` with a very high (often maximum) real-time priority.
    ///   Requires CAP_SYS_NICE capability or root privileges.
    Realtime,
}

/// Errors from [`set_thread_priority`].
pub enum ThreadPriorityError {
    /// The thread priority is not supported on this platform.
    UnsupportedPlatform,
    /// The current execution context does not have the permissoins to use this thread priority.
    PermissionDenied,
    /// An unknown, platform-specific error occured.
    Unknown,
}

/// Sets the priority of the current thread.
///
/// This affectss how regularly the OS scheduler will preemptively interrupt the current thread to allow other threads
/// and processes to use the CPU.
///
/// The interpretation of priority levels can vary between operating systems. Refer to the `ThreadPriority` enum for available levels.
///
/// # Platform Specific Behavior
/// This will always return `ThreadPriority::UnsupportedPlatform` in web builds.
pub fn set_thread_priority(thread_priority: ThreadPriority) -> Result<(), ThreadPriorityError> {
    crate::cfg::web! {
        if {
            Err(ThreadPriorityError::UnsupportedPlatform)
        } else {
            let gdt_priority = match thread_priority {
                ThreadPriority::Background => gdt_cpus::ThreadPriority::Background,
                ThreadPriority::Lowest => gdt_cpus::ThreadPriority::Lowest,
                ThreadPriority::BelowNormal => gdt_cpus::ThreadPriority::BelowNormal,
                ThreadPriority::Normal => gdt_cpus::ThreadPriority::Normal,
                ThreadPriority::AboveNormal => gdt_cpus::ThreadPriority::AboveNormal,
                ThreadPriority::Highest => gdt_cpus::ThreadPriority::Highest,
                ThreadPriority::Realtime => gdt_cpus::ThreadPriority::TimeCritical,
            };

            Err(match gdt_cpus::set_thread_priority(gdt_priority) {
                Ok(()) => return Ok(()),
                Err(gdt_cpus::Error::Unsupported(_)) => ThreadPriorityError::UnsupportedPlatform,
                Err(gdt_cpus::Error::PermissionDenied(_)) => ThreadPriorityError::PermissionDenied,
                _ => ThreadPriorityError::Unknown,
            })
        }
    }
}
