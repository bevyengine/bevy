mod slice;
pub use slice::{ParallelSlice, ParallelSliceMut};

mod task;
pub use task::Task;

mod task_pool;
pub use task_pool::{Scope, TaskPool, TaskPoolBuilder};

mod usages;
pub use usages::{AsyncComputeTaskPool, ComputeTaskPool, IOTaskPool};

mod countdown_event;
pub use countdown_event::CountdownEvent;

mod iter;
pub use iter::ParallelIterator;

pub mod prelude {
    pub use crate::{
        iter::ParallelIterator,
        slice::{ParallelSlice, ParallelSliceMut},
        usages::{AsyncComputeTaskPool, ComputeTaskPool, IOTaskPool},
    };
}

pub fn logical_core_count() -> usize {
    num_cpus::get()
}

pub fn physical_core_count() -> usize {
    num_cpus::get_physical()
}
