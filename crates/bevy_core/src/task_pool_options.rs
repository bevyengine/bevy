use bevy_tasks::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool, TaskPoolBuilder};
use bevy_utils::tracing::trace;

/// Defines a simple way to determine how many threads to use given the number of remaining cores
/// and number of total cores
#[derive(Clone, Debug)]
pub struct TaskPoolThreadAssignmentPolicy {
    /// Force using at least this many threads
    pub min_threads: usize,
    /// Under no circumstance use more than this many threads for this pool
    pub max_threads: usize,
    /// Target using this percentage of total cores, clamped by `min_threads` and `max_threads`. It is
    /// permitted to use 1.0 to try to use all remaining threads
    pub percent: f32,
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
/// set up [`TaskPoolPlugin`](super::TaskPoolPlugin)
#[derive(Clone, Debug)]
pub struct TaskPoolOptions {
    /// If the number of physical cores is less than `min_total_threads`, force using
    /// `min_total_threads`
    pub min_total_threads: usize,
    /// If the number of physical cores is greater than `max_total_threads`, force using
    /// `max_total_threads`
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
            max_total_threads: usize::MAX,

            // Use 25% of cores for IO, at least 1, no more than 4
            io: TaskPoolThreadAssignmentPolicy {
                min_threads: 1,
                max_threads: 4,
                percent: 0.25,
            },

            // Use 25% of cores for async compute, at least 1, no more than 4
            async_compute: TaskPoolThreadAssignmentPolicy {
                min_threads: 1,
                max_threads: 4,
                percent: 0.25,
            },

            // Use all remaining cores for compute (at least 1)
            compute: TaskPoolThreadAssignmentPolicy {
                min_threads: 1,
                max_threads: usize::MAX,
                percent: 1.0, // This 1.0 here means "whatever is left over"
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

        {
            // Determine the number of IO threads we will use
            let io_threads = self
                .io
                .get_number_of_threads(remaining_threads, total_threads);

            trace!("IO Threads: {}", io_threads);
            remaining_threads = remaining_threads.saturating_sub(io_threads);

            IoTaskPool::get_or_init(|| {
                TaskPoolBuilder::default()
                    .num_threads(io_threads)
                    .thread_name("IO Task Pool".to_string())
                    .build()
            });
        }

        {
            // Determine the number of async compute threads we will use
            let async_compute_threads = self
                .async_compute
                .get_number_of_threads(remaining_threads, total_threads);

            trace!("Async Compute Threads: {}", async_compute_threads);
            remaining_threads = remaining_threads.saturating_sub(async_compute_threads);

            AsyncComputeTaskPool::get_or_init(|| {
                TaskPoolBuilder::default()
                    .num_threads(async_compute_threads)
                    .thread_name("Async Compute Task Pool".to_string())
                    .build()
            });
        }

        {
            // Determine the number of compute threads we will use
            // This is intentionally last so that an end user can specify 1.0 as the percent
            let compute_threads = self
                .compute
                .get_number_of_threads(remaining_threads, total_threads);

            trace!("Compute Threads: {}", compute_threads);

            ComputeTaskPool::get_or_init(|| {
                TaskPoolBuilder::default()
                    .num_threads(compute_threads)
                    .thread_name("Compute Task Pool".to_string())
                    .build()
            });
        }
    }
}
