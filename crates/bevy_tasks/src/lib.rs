#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

mod countdown_event;
mod iter;
#[cfg(target_arch = "wasm32")]
mod single_threaded_task_pool;
mod slice;
mod task;
#[cfg(not(target_arch = "wasm32"))]
mod task_pool;
mod usages;

/// The `bevy_tasks` prelude.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        iter::ParallelIterator,
        slice::{ParallelSlice, ParallelSliceMut},
        usages::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool},
    };
}

pub use countdown_event::CountdownEvent;
pub use iter::ParallelIterator;
pub use num_cpus::get as logical_core_count;
pub use num_cpus::get_physical as physical_core_count;
#[cfg(target_arch = "wasm32")]
pub use single_threaded_task_pool::{Scope, TaskPool, TaskPoolBuilder};
pub use slice::{ParallelSlice, ParallelSliceMut};
pub use task::Task;
#[cfg(not(target_arch = "wasm32"))]
pub use task_pool::{Scope, TaskPool, TaskPoolBuilder};
pub use usages::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool};
