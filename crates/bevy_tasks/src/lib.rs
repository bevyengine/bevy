mod slice;
pub use slice::{ParallelSlice, ParallelSliceMut};

mod task;
pub use task::Task;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm_threads"))]
mod task_pool;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm_threads"))]
pub use task_pool::{Scope, TaskPool, TaskPoolBuilder};

#[cfg(all(target_arch = "wasm32", not(feature = "wasm_threads")))]
mod single_threaded_task_pool;
#[cfg(all(target_arch = "wasm32", not(feature = "wasm_threads")))]
pub use single_threaded_task_pool::{Scope, TaskPool, TaskPoolBuilder};

mod usages;
pub use usages::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool};

mod countdown_event;
pub use countdown_event::CountdownEvent;

mod iter;
pub use iter::ParallelIterator;

pub mod prelude {
    pub use crate::{
        iter::ParallelIterator,
        slice::{ParallelSlice, ParallelSliceMut},
        usages::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool},
    };
}

pub fn logical_core_count() -> usize {
    num_cpus::get()
}

pub fn physical_core_count() -> usize {
    num_cpus::get_physical()
}
