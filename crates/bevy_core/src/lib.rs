#![warn(missing_docs)]
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
    pub use crate::{CorePlugin, Name, TaskPoolOptions};
}

use bevy_app::prelude::*;
use bevy_ecs::entity::Entity;
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};
use bevy_utils::{Duration, HashSet, Instant};
use std::borrow::Cow;
use std::ffi::OsString;
use std::ops::Range;
use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
use bevy_ecs::schedule::IntoSystemDescriptor;
#[cfg(not(target_arch = "wasm32"))]
use bevy_tasks::tick_global_task_pools_on_main_thread;

/// Adds core functionality to Apps.
#[derive(Default)]
pub struct CorePlugin {
    /// Options for the [`TaskPool`](bevy_tasks::TaskPool) created at application start.
    pub task_pool_options: TaskPoolOptions,
}

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        // Setup the default bevy task pools
        self.task_pool_options.create_default_pools();

        #[cfg(not(target_arch = "wasm32"))]
        app.add_system_to_stage(
            bevy_app::CoreStage::Last,
            tick_global_task_pools_on_main_thread.at_end(),
        );

        app.register_type::<Entity>().register_type::<Name>();

        register_rust_types(app);
        register_math_types(app);

        app.init_resource::<FrameCount>();
        app.add_system(update_frame_count);
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
        .register_type::<Cow<'static, str>>()
        .register_type::<Duration>()
        .register_type::<Instant>();
}

fn register_math_types(app: &mut App) {
    app.register_type::<bevy_math::IVec2>()
        .register_type::<bevy_math::IVec3>()
        .register_type::<bevy_math::IVec4>()
        .register_type::<bevy_math::UVec2>()
        .register_type::<bevy_math::UVec3>()
        .register_type::<bevy_math::UVec4>()
        .register_type::<bevy_math::DVec2>()
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
        .register_type::<bevy_math::Quat>();
}

/// Keeps a count of rendered frames since the start of the app
///
/// Wraps to 0 when it reaches the maximum u32 value
#[derive(Default, Resource, Clone, Copy)]
pub struct FrameCount(pub u32);

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
        app.add_plugin(CorePlugin::default());

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
        app.add_plugin(CorePlugin::default());
        app.update();

        let frame_count = app.world.resource::<FrameCount>();
        assert_eq!(1, frame_count.0);
    }
}
