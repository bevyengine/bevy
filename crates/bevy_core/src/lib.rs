#![warn(missing_docs)]
#![allow(clippy::type_complexity)]
//! This crate provides core functionality for Bevy Engine.

mod name;
#[cfg(feature = "serialize")]
mod serde;
mod task_pool_options;

use bevy_ecs::system::{ResMut, Resource};
pub use bytemuck::{bytes_of, cast_slice, Pod, Zeroable};
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
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};
use bevy_utils::{Duration, HashSet, Instant, Uuid};
use std::borrow::Cow;
use std::ffi::OsString;
use std::marker::PhantomData;
use std::ops::Range;
use std::path::{Path, PathBuf};

#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(target_arch = "wasm32"))]
use bevy_tasks::tick_global_task_pools_on_main_thread;

/// Registration of default types to the [`TypeRegistry`](bevy_reflect::TypeRegistry) resource.
#[derive(Default)]
pub struct TypeRegistrationPlugin;

impl Plugin for TypeRegistrationPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Entity>().register_type::<Name>();

        register_rust_types(app);
        register_math_types(app);
    }
}

fn register_rust_types(app: &mut App) {
    app.register_type::<Range<f32>>()
        .register_type_data::<Range<f32>, ReflectSerialize>()
        .register_type_data::<Range<f32>, ReflectDeserialize>()
        .register_type::<String>()
        .register_type::<PathBuf>()
        .register_type::<OsString>()
        .register_type::<HashSet<String>>()
        .register_type::<Option<String>>()
        .register_type::<Option<bool>>()
        .register_type::<Option<f64>>()
        .register_type::<Cow<'static, str>>()
        .register_type::<Cow<'static, Path>>()
        .register_type::<Duration>()
        .register_type::<Instant>()
        .register_type::<Uuid>();
}

fn register_math_types(app: &mut App) {
    app.register_type::<bevy_math::IVec2>()
        .register_type::<bevy_math::IVec3>()
        .register_type::<bevy_math::IVec4>()
        .register_type::<bevy_math::UVec2>()
        .register_type::<bevy_math::UVec3>()
        .register_type::<bevy_math::UVec4>()
        .register_type::<bevy_math::DVec2>()
        .register_type::<Option<bevy_math::DVec2>>()
        .register_type::<bevy_math::DVec3>()
        .register_type::<bevy_math::DVec4>()
        .register_type::<bevy_math::BVec2>()
        .register_type::<bevy_math::BVec3>()
        .register_type::<bevy_math::BVec3A>()
        .register_type::<bevy_math::BVec4>()
        .register_type::<bevy_math::BVec4A>()
        .register_type::<bevy_math::Vec2>()
        .register_type::<bevy_math::Vec3>()
        .register_type::<bevy_math::Vec3A>()
        .register_type::<bevy_math::Vec4>()
        .register_type::<bevy_math::DAffine2>()
        .register_type::<bevy_math::DAffine3>()
        .register_type::<bevy_math::Affine2>()
        .register_type::<bevy_math::Affine3A>()
        .register_type::<bevy_math::DMat2>()
        .register_type::<bevy_math::DMat3>()
        .register_type::<bevy_math::DMat4>()
        .register_type::<bevy_math::Mat2>()
        .register_type::<bevy_math::Mat3>()
        .register_type::<bevy_math::Mat3A>()
        .register_type::<bevy_math::Mat4>()
        .register_type::<bevy_math::DQuat>()
        .register_type::<bevy_math::Quat>()
        .register_type::<bevy_math::Rect>();
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
#[derive(Default, Resource, Clone, Copy)]
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

fn update_frame_count(mut frame_count: ResMut<FrameCount>) {
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

        let frame_count = app.world.resource::<FrameCount>();
        assert_eq!(1, frame_count.0);
    }
}
