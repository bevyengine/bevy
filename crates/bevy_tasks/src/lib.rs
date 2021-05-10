pub use countdown_event::CountdownEvent;
pub use iter::ParallelIterator;
#[cfg(not(target_arch = "wasm32"))]
pub use priority_executor::*;
#[cfg(target_arch = "wasm32")]
pub use single_threaded_task_pool::{Scope, TaskPool, TaskPoolBuilder};
pub use slice::{ParallelSlice, ParallelSliceMut};
pub use task::Task;
#[cfg(not(target_arch = "wasm32"))]
pub use task_pool::{Scope, TaskPool, TaskPoolBuilder};
pub use usages::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool};

mod slice;
mod task;
#[cfg(not(target_arch = "wasm32"))]
mod task_pool;
#[cfg(not(target_arch = "wasm32"))]
mod priority_executor;

#[cfg(target_arch = "wasm32")]
mod single_threaded_task_pool;
mod usages;
mod countdown_event;
mod iter;

pub mod prelude {
    pub use crate::{
        iter::ParallelIterator,
        slice::{ParallelSlice, ParallelSliceMut},
        usages::ComputeTaskPool,
    };
}

pub fn logical_core_count() -> usize {
    num_cpus::get()
}

pub fn physical_core_count() -> usize {
    num_cpus::get_physical()
}
