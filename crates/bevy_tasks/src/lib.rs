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

// The following docs are copied from `num_cpus`'s docs for the wrapped functions.
// Attributed under MIT or Apache: https://github.com/seanmonstar/num_cpus

/// Returns the number of available CPUs of the current system.
///
/// This function will get the number of logical cores. Sometimes this is different from the number
/// of physical cores (See [Simultaneous multithreading on Wikipedia][smt]).
///
/// This will always return at least `1`.
///
/// # Examples
///
/// ```rust
/// let cpus = bevy_tasks::logical_core_count();
/// if cpus > 1 {
///     println!("We are on a multicore system with {} CPUs", cpus);
/// } else {
///     println!("We are on a single core system");
/// }
/// ```
///
/// # Note
///
/// This will check [sched affinity] on Linux, showing a lower number of CPUs if the current
/// thread does not have access to all the computer's CPUs.
///
/// This will also check [cgroups], frequently used in containers to constrain CPU usage.
///
/// [smt]: https://en.wikipedia.org/wiki/Simultaneous_multithreading
/// [sched affinity]: http://www.gnu.org/software/libc/manual/html_node/CPU-Affinity.html
/// [cgroups]: https://www.kernel.org/doc/Documentation/cgroup-v1/cgroups.txt
#[inline(always)]
pub fn logical_core_count() -> usize {
    num_cpus::get()
}

/// Returns the number of physical cores of the current system.
///
/// This will always return at least `1`.
///
/// # Note
///
/// Physical count is supported only on Linux, mac OS and Windows platforms.
/// On other platforms, or if the physical count fails on supported platforms,
/// this function returns the same as [`get()`], which is the number of logical
/// CPUS.
///
/// # Examples
///
/// ```rust
/// let logical_cpus = bevy_tasks::logical_core_count();
/// let physical_cpus = bevy_tasks::physical_core_count();
/// if logical_cpus > physical_cpus {
///     println!("We have simultaneous multithreading with about {:.2} \
///               logical cores to 1 physical core.",
///               (logical_cpus as f64) / (physical_cpus as f64));
/// } else if logical_cpus == physical_cpus {
///     println!("Either we don't have simultaneous multithreading, or our \
///               system doesn't support getting the number of physical CPUs.");
/// } else {
///     println!("We have less logical CPUs than physical CPUs, maybe we only have access to \
///               some of the CPUs on our system.");
/// }
/// ```
///
/// [`logical_core_count()`]: fn.logical_core_count.html
#[inline(always)]
pub fn physical_core_count() -> usize {
    num_cpus::get_physical()
}
