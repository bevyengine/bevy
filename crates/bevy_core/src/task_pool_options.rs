use bevy_ecs::world::World;
use bevy_tasks::{ComputeTaskPool, TaskPoolBuilder};
use bevy_utils::tracing::trace;

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
}

/// Helper for configuring and creating the default task pools. For end-users who want full control,
/// insert the default task pools into the resource map manually. If the pools are already inserted,
/// this helper will do nothing.
#[derive(Clone)]
pub struct DefaultTaskPoolOptions {
    /// If the number of physical cores is less than min_total_threads, force using
    /// min_total_threads
    pub min_total_threads: usize,
    /// If the number of physical cores is grater than max_total_threads, force using
    /// max_total_threads
    pub max_total_threads: usize,

    /// Used to determine number of IO threads to allocate
    pub io: TaskPoolThreadAssignmentPolicy,
    /// Used to determine number of async compute threads to allocate
    pub async_compute: TaskPoolThreadAssignmentPolicy,
    /// Used to determine number of compute threads to allocate
    pub compute: TaskPoolThreadAssignmentPolicy,
}

impl Default for DefaultTaskPoolOptions {
    fn default() -> Self {
        DefaultTaskPoolOptions {
            // By default, use however many cores are available on the system
            min_total_threads: 1,
            max_total_threads: std::usize::MAX,

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
                max_threads: std::usize::MAX,
                percent: 1.0, // This 1.0 here means "whatever is left over"
            },
        }
    }
}

impl DefaultTaskPoolOptions {
    /// Create a configuration that forces using the given number of threads.
    pub fn with_num_threads(thread_count: usize) -> Self {
        DefaultTaskPoolOptions {
            min_total_threads: thread_count,
            max_total_threads: thread_count,
            ..Default::default()
        }
    }

    /// Inserts the default thread pools into the given resource map based on the configured values
    pub fn create_default_pools(&self, world: &mut World) {
        let total_threads =
            bevy_tasks::logical_core_count().clamp(self.min_total_threads, self.max_total_threads);
        trace!("Assigning {} cores to default task pools", total_threads);

        if !world.contains_resource::<ComputeTaskPool>() {
            world.insert_resource(ComputeTaskPool(
                TaskPoolBuilder::default()
                    .num_threads(total_threads)
                    .thread_name("Compute Task Pool".to_string())
                    .build(),
            ));
        }
    }
}
