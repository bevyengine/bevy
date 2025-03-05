use crate::{App, Plugin};

use alloc::string::ToString;
use bevy_platform_support::sync::Arc;
use bevy_tasks::{ComputeTaskPool, TaskPoolBuilder};
use core::{fmt::Debug, marker::PhantomData};
use log::trace;

#[cfg(not(target_arch = "wasm32"))]
use {crate::Last, bevy_ecs::prelude::NonSend};

#[cfg(not(target_arch = "wasm32"))]
use bevy_tasks::tick_global_task_pools_on_main_thread;

/// Setup of default task pools: [`AsyncComputeTaskPool`], [`ComputeTaskPool`], [`IoTaskPool`].
#[derive(Default)]
pub struct TaskPoolPlugin {
    /// Options for the [`TaskPool`](bevy_tasks::TaskPool) created at application start.
    pub task_pool_options: TaskPoolOptions,
}

impl Plugin for TaskPoolPlugin {
    fn build(&self, _app: &mut App) {
        // Setup the default bevy task pools
        self.task_pool_options.create_default_pools();

        #[cfg(not(target_arch = "wasm32"))]
        _app.add_systems(Last, tick_global_task_pools);
    }
}
/// A dummy type that is [`!Send`](Send), to force systems to run on the main thread.
pub struct NonSendMarker(PhantomData<*mut ()>);

/// A system used to check and advanced our task pools.
///
/// Calls [`tick_global_task_pools_on_main_thread`],
/// and uses [`NonSendMarker`] to ensure that this system runs on the main thread
#[cfg(not(target_arch = "wasm32"))]
fn tick_global_task_pools(_main_thread_marker: Option<NonSend<NonSendMarker>>) {
    tick_global_task_pools_on_main_thread();
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
}

impl Debug for TaskPoolOptions {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TaskPoolOptions")
            .field("min_total_threads", &self.min_total_threads)
            .field("max_total_threads", &self.max_total_threads)
            .field("on_thread_spawn", &self.on_thread_spawn.is_some())
            .field("on_thread_destroy", &self.on_thread_destroy.is_some())
            .finish()
    }
}

impl Default for TaskPoolOptions {
    fn default() -> Self {
        TaskPoolOptions {
            // By default, use however many cores are available on the system
            min_total_threads: 1,
            max_total_threads: usize::MAX,
            on_thread_spawn: None,
            on_thread_destroy: None,
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

        ComputeTaskPool::get_or_init(|| {
            #[cfg_attr(target_arch = "wasm32", expect(unused_mut))]
            let mut builder = TaskPoolBuilder::default()
                .num_threads(total_threads)
                .thread_name("Compute Task Pool".to_string());

            #[cfg(not(target_arch = "wasm32"))]
            {
                if let Some(f) = self.on_thread_spawn.clone() {
                    builder = builder.on_thread_spawn(move || f());
                }
                if let Some(f) = self.on_thread_destroy.clone() {
                    builder = builder.on_thread_destroy(move || f());
                }
            }

            builder.build()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_tasks::prelude::ComputeTaskPool;

    #[test]
    fn runs_spawn_local_tasks() {
        let mut app = App::new();
        app.add_plugins(TaskPoolPlugin::default());

        let (async_tx, async_rx) = crossbeam_channel::unbounded();
        ComputeTaskPool::get()
            .spawn_local(async move {
                async_tx.send(()).unwrap();
            })
            .detach();

        let (compute_tx, compute_rx) = crossbeam_channel::unbounded();
        ComputeTaskPool::get()
            .spawn_local(async move {
                compute_tx.send(()).unwrap();
            })
            .detach();

        let (io_tx, io_rx) = crossbeam_channel::unbounded();
        ComputeTaskPool::get()
            .spawn_local(async move {
                io_tx.send(()).unwrap();
            })
            .detach();

        app.run();

        async_rx.try_recv().unwrap();
        compute_rx.try_recv().unwrap();
        io_rx.try_recv().unwrap();
    }
}
