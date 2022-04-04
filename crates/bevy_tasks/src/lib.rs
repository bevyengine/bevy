#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

mod slice;
pub use slice::{ParallelSlice, ParallelSliceMut};

mod task;
pub use task::Task;

#[cfg(not(target_arch = "wasm32"))]
mod task_pool;
#[cfg(not(target_arch = "wasm32"))]
pub use task_pool::{Scope, TaskPool, TaskPoolBuilder};

#[cfg(target_arch = "wasm32")]
mod single_threaded_task_pool;
#[cfg(target_arch = "wasm32")]
pub use single_threaded_task_pool::{Scope, TaskPool, TaskPoolBuilder};

mod usages;
pub use usages::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool};

mod countdown_event;
pub use countdown_event::CountdownEvent;

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

pub use num_cpus::get as logical_core_count;
pub use num_cpus::get_physical as physical_core_count;
