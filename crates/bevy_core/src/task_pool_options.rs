use bevy_ecs::prelude::Resource;
use bevy_tasks::{
    core_affinity, AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool, TaskPoolBuilder,
};
use bevy_utils::tracing::trace;
use std::sync::{Arc, Mutex};

/// Defines a simple way to determine how many threads to use given the number of remaining cores
/// and number of total cores
#[derive(Clone)]
pub struct TaskPoolThreadAssignmentPolicy {
    /// Force using at least this many threads
    pub min_threads: usize,
    /// Under no circumstance use more than this many threads for this pool
    pub max_threads: usize,
    /// Target using this percentage of total cores, clamped by min_threads and max_threads. It is
    /// permitted to use 1.0 to try to use all remaining threads
    pub percent: f32,
    /// If set to true, this will use [processor affinity] to forcibly pin each thread to a different
    /// physical CPU core.
    ///
    /// Only has an effect on Windows, Mac OSX, Linux, Android. This is a no-op on other platforms.
    ///
    /// Defaults to true.
    ///
    /// [processor affinity]: https://en.wikipedia.org/wiki/Processor_affinity
    pub use_core_affinity: bool,
}

impl TaskPoolThreadAssignmentPolicy {
    /// Determine the number of threads to use for this task pool
    fn get_number_of_threads(&self, remaining_threads: usize, total_threads: usize) -> usize {
        assert!(self.percent >= 0.0);
        let mut desired = (total_threads as f32 * self.percent).round() as usize;

        // Limit ourselves to the number of cores available
        desired = desired.min(remaining_threads);

        // Clamp by min_threads, max_threads. (This may result in us using more threads than are
        // available, this is intended. An example case where this might happen is a device with
        // <= 2 threads.
        desired.clamp(self.min_threads, self.max_threads)
    }
}

/// Helper for configuring and creating the default task pools. For end-users who want full control,
/// set up [`CorePlugin`](super::CorePlugin)
#[derive(Clone, Resource)]
pub struct TaskPoolOptions {
    /// If the number of physical cores is less than min_total_threads, force using
    /// min_total_threads
    pub min_total_threads: usize,
    /// If the number of physical cores is greater than max_total_threads, force using
    /// max_total_threads
    pub max_total_threads: usize,

    /// Used to determine number of IO threads to allocate
    pub io: TaskPoolThreadAssignmentPolicy,
    /// Used to determine number of async compute threads to allocate
    pub async_compute: TaskPoolThreadAssignmentPolicy,
    /// Used to determine number of compute threads to allocate
    pub compute: TaskPoolThreadAssignmentPolicy,
}

impl Default for TaskPoolOptions {
    fn default() -> Self {
        TaskPoolOptions {
            // By default, use however many cores are available on the system
            min_total_threads: 1,
            max_total_threads: std::usize::MAX,

            // Use 25% of cores for IO, at least 1, no more than 4
            io: TaskPoolThreadAssignmentPolicy {
                min_threads: 1,
                max_threads: 4,
                percent: 0.25,
                use_core_affinity: true,
            },

            // Use 25% of cores for async compute, at least 1, no more than 4
            async_compute: TaskPoolThreadAssignmentPolicy {
                min_threads: 1,
                max_threads: 4,
                percent: 0.25,
                use_core_affinity: true,
            },

            // Use all remaining cores for compute (at least 1)
            compute: TaskPoolThreadAssignmentPolicy {
                min_threads: 1,
                max_threads: std::usize::MAX,
                percent: 1.0, // This 1.0 here means "whatever is left over"
                use_core_affinity: true,
            },
        }
    }
}

impl TaskPoolOptions {
    /// Create a configuration that forces using the given number of threads.
    pub fn with_num_threads(thread_count: usize) -> Self {
        TaskPoolOptions {
            min_total_threads: thread_count,
            max_total_threads: thread_count,
            ..Default::default()
        }
    }

    /// Inserts the default thread pools into the given resource map based on the configured values
    pub fn create_default_pools(&self) {
        let total_threads = bevy_tasks::available_parallelism()
            .clamp(self.min_total_threads, self.max_total_threads);
        trace!("Assigning {} cores to default task pools", total_threads);

        let mut remaining_threads = total_threads;
        let core_ids = core_affinity::get_core_ids().map(|core_ids| Arc::new(Mutex::new(core_ids)));

        {
            // Determine the number of IO threads we will use
            let io_threads = self
                .io
                .get_number_of_threads(remaining_threads, total_threads);

            trace!("IO Threads: {}", io_threads);
            remaining_threads = remaining_threads.saturating_sub(io_threads);

            IoTaskPool::init(|| {
                let mut builder = TaskPoolBuilder::new()
                    .num_threads(io_threads)
                    .thread_name("IO Task Pool".to_string());
                if let Some(core_ids) = core_ids.clone() {
                    builder = builder.core_id_fn(move || core_ids.lock().ok()?.pop());
                }
                builder.build()
            });
        }

        {
            // Determine the number of async compute threads we will use
            let async_compute_threads = self
                .async_compute
                .get_number_of_threads(remaining_threads, total_threads);

            trace!("Async Compute Threads: {}", async_compute_threads);
            remaining_threads = remaining_threads.saturating_sub(async_compute_threads);

            AsyncComputeTaskPool::init(|| {
                let mut builder = TaskPoolBuilder::new()
                    .num_threads(async_compute_threads)
                    .thread_name("Async Compute Task Pool".to_string());
                if let Some(core_ids) = core_ids.clone() {
                    builder = builder.core_id_fn(move || core_ids.lock().ok()?.pop());
                }
                builder.build()
            });
        }

        {
            // Determine the number of compute threads we will use
            // This is intentionally last so that an end user can specify 1.0 as the percent
            let compute_threads = self
                .compute
                .get_number_of_threads(remaining_threads, total_threads);

            trace!("Compute Threads: {}", compute_threads);

            ComputeTaskPool::init(|| {
                let mut builder = TaskPoolBuilder::new()
                    .num_threads(compute_threads)
                    .thread_name("Compute Task Pool".to_string());
                if let Some(core_ids) = core_ids.clone() {
                    builder = builder.core_id_fn(move || core_ids.lock().ok()?.pop());
                }
                builder.build()
            });
        }
    }
}
