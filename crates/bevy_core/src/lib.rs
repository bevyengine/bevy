#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! This crate provides core functionality for Bevy Engine.

extern crate alloc;

mod task_pool_options;

pub use task_pool_options::*;

/// The core prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{TaskPoolOptions, TaskPoolPlugin};
}

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use core::marker::PhantomData;

#[cfg(not(target_arch = "wasm32"))]
use bevy_tasks::tick_global_task_pools_on_main_thread;

/// Setup of default task pools: [`AsyncComputeTaskPool`](bevy_tasks::AsyncComputeTaskPool),
/// [`ComputeTaskPool`](bevy_tasks::ComputeTaskPool), [`IoTaskPool`](bevy_tasks::IoTaskPool).
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_tasks::prelude::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool};

    #[test]
    fn runs_spawn_local_tasks() {
        let mut app = App::new();
        app.add_plugins(TaskPoolPlugin::default());

        let (async_tx, async_rx) = crossbeam_channel::unbounded();
        AsyncComputeTaskPool::get()
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
        IoTaskPool::get()
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
