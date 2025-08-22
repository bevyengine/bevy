use crate::{App, Plugin};

use alloc::{string::ToString, vec::Vec};
use bevy_platform::{collections::HashMap, sync::Arc};
use bevy_tasks::{TaskPool, TaskPoolBuilder, TaskPriority};
use core::fmt::Debug;
use log::trace;

cfg_if::cfg_if! {
    if #[cfg(not(all(target_arch = "wasm32", feature = "web")))] {
        use {crate::Last, bevy_tasks::tick_global_task_pools_on_main_thread};
        use bevy_ecs::system::NonSendMarker;

        /// A system used to check and advanced our task pools.
        ///
        /// Calls [`tick_global_task_pools_on_main_thread`],
        /// and uses [`NonSendMarker`] to ensure that this system runs on the main thread
        fn tick_global_task_pools(_main_thread_marker: NonSendMarker) {
            tick_global_task_pools_on_main_thread();
        }
    }
}

/// Setup of default task pools: [`AsyncTaskPool`], [`TaskPool`], [`IoTaskPool`].
#[derive(Default)]
pub struct TaskPoolPlugin {
    /// Options for the [`TaskPool`](bevy_tasks::TaskPool) created at application start.
    pub task_pool_options: TaskPoolOptions,
}

impl Plugin for TaskPoolPlugin {
    fn build(&self, _app: &mut App) {
        // Setup the default bevy task pools
        self.task_pool_options.create_default_pools();

        #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
        _app.add_systems(Last, tick_global_task_pools);
    }
}

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
        let proportion = total_threads as f32 * self.percent;
        let mut desired = proportion as usize;

        // Equivalent to round() for positive floats without libm requirement for
        // no_std compatibility
        if proportion - desired as f32 >= 0.5 {
            desired += 1;
        }

        // Limit ourselves to the number of cores available
        desired = desired.min(remaining_threads);

        // Clamp by min_threads, max_threads. (This may result in us using more threads than are
        // available, this is intended. An example case where this might happen is a device with
        // <= 2 threads.
        desired.clamp(self.min_threads, self.max_threads)
    }
}

/// Helper for configuring and creating the default task pools. For end-users who want full control,
/// set up [`TaskPoolPlugin`]
#[derive(Clone)]
pub struct TaskPoolOptions {
    /// If the number of physical cores is less than `min_total_threads`, force using
    /// `min_total_threads`
    pub min_total_threads: usize,
    /// If the number of physical cores is greater than `max_total_threads`, force using
    /// `max_total_threads`
    pub max_total_threads: usize,

    /// Callback that is invoked once for every created thread as it starts.
    /// This configuration will be ignored under wasm platform.
    pub on_thread_spawn: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
    /// Callback that is invoked once for every created thread as it terminates
    /// This configuration will be ignored under wasm platform.
    pub on_thread_destroy: Option<Arc<dyn Fn() + Send + Sync + 'static>>,

    /// Used to determine number of threads to provide to each
    pub priority_assignment_policies: HashMap<TaskPriority, TaskPoolThreadAssignmentPolicy>,
}

impl Debug for TaskPoolOptions {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TaskPoolOptions")
            .field("min_total_threads", &self.min_total_threads)
            .field("max_total_threads", &self.max_total_threads)
            .field(
                "priority_assignment_policies",
                &self.priority_assignment_policies,
            )
            .finish()
    }
}

impl Default for TaskPoolOptions {
    fn default() -> Self {
        let mut priority_assignment_policies = HashMap::new();
        // Use 25% of cores for IO, at least 1, no more than 4
        priority_assignment_policies.insert(
            TaskPriority::BlockingIO,
            TaskPoolThreadAssignmentPolicy {
                min_threads: 1,
                max_threads: 4,
                percent: 0.25,
            },
        );
        // Use 25% of cores for blocking compute, at least 1, no more than 4
        priority_assignment_policies.insert(
            TaskPriority::BlockingCompute,
            TaskPoolThreadAssignmentPolicy {
                min_threads: 1,
                max_threads: 4,
                percent: 0.25,
            },
        );
        // Use 25% of cores for async IO, at least 1, no more than 4
        priority_assignment_policies.insert(
            TaskPriority::AsyncIO,
            TaskPoolThreadAssignmentPolicy {
                min_threads: 1,
                max_threads: 4,
                percent: 0.25,
            },
        );

        TaskPoolOptions {
            // By default, use however many cores are available on the system
            min_total_threads: 1,
            max_total_threads: usize::MAX,

            on_thread_spawn: None,
            on_thread_destroy: None,

            priority_assignment_policies,
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
        trace!("Assigning {total_threads} cores to default task pools");

        let mut remaining_threads = total_threads;

        let mut builder = TaskPoolBuilder::default()
            .num_threads(total_threads)
            .thread_name("Task Pool".to_string());

        let mut ordered = self.priority_assignment_policies.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|(prio, _)| **prio);
        for (priority, policy) in ordered {
            let priority_threads = policy.get_number_of_threads(remaining_threads, total_threads);
            builder = builder.priority_limit(*priority, Some(priority_threads));

            remaining_threads = remaining_threads.saturating_sub(priority_threads);
            trace!("{:?} Threads: {priority_threads}", *priority);
        }

        #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
        let builder = {
            let mut builder = builder;
            if let Some(f) = self.on_thread_spawn.clone() {
                builder = builder.on_thread_spawn(move || f());
            }
            if let Some(f) = self.on_thread_destroy.clone() {
                builder = builder.on_thread_destroy(move || f());
            }
            builder
        };

        TaskPool::get_or_init(move || builder);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_tasks::prelude::TaskPool;

    #[test]
    fn runs_spawn_local_tasks() {
        let mut app = App::new();
        app.add_plugins(TaskPoolPlugin::default());

        let (tx, rx) = crossbeam_channel::unbounded();
        TaskPool::get_or_init(Default::default)
            .spawn_local(async move {
                tx.send(()).unwrap();
            })
            .detach();

        app.run();

        rx.try_recv().unwrap();
    }
}
