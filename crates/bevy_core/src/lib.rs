#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! This crate provides core functionality for Bevy Engine.

mod name;
#[cfg(feature = "serialize")]
mod serde;
mod task_pool_options;

use bevy_ecs::system::Resource;
pub use name::*;
pub use task_pool_options::*;

pub mod prelude {
    //! The Bevy Core Prelude.
    #[doc(hidden)]
    pub use crate::{
        DebugName, FrameCountPlugin, Name, TaskPoolOptions, TaskPoolPlugin, TypeRegistrationPlugin,
    };
}

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use std::marker::PhantomData;

#[cfg(not(target_arch = "wasm32"))]
use bevy_tasks::tick_global_task_pools_on_main_thread;

/// Registration of default types to the [`TypeRegistry`](bevy_reflect::TypeRegistry) resource.
#[derive(Default)]
pub struct TypeRegistrationPlugin;

impl Plugin for TypeRegistrationPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Name>();
    }
}

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

/// Maintains a count of frames rendered since the start of the application.
///
/// [`FrameCount`] is incremented during [`Last`], providing predictable
/// behavior: it will be 0 during the first update, 1 during the next, and so forth.
///
/// # Overflows
///
/// [`FrameCount`] will wrap to 0 after exceeding [`u32::MAX`]. Within reasonable
/// assumptions, one may exploit wrapping arithmetic to determine the number of frames
/// that have elapsed between two observations – see [`u32::wrapping_sub()`].
#[derive(Debug, Default, Resource, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameCount(pub u32);

/// Adds frame counting functionality to Apps.
#[derive(Default)]
pub struct FrameCountPlugin;

impl Plugin for FrameCountPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameCount>();
        app.add_systems(Last, update_frame_count);
    }
}

/// A system used to increment [`FrameCount`] with wrapping addition.
///
/// See [`FrameCount`] for more details.
pub fn update_frame_count(mut frame_count: ResMut<FrameCount>) {
    frame_count.0 = frame_count.0.wrapping_add(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_tasks::prelude::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool};

    #[test]
    fn runs_spawn_local_tasks() {
        let mut app = App::new();
        app.add_plugins((TaskPoolPlugin::default(), TypeRegistrationPlugin));

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

    #[test]
    fn frame_counter_update() {
        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            TypeRegistrationPlugin,
            FrameCountPlugin,
        ));
        app.update();

        let frame_count = app.world().resource::<FrameCount>();
        assert_eq!(1, frame_count.0);
    }
}
