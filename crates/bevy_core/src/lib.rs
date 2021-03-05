mod bytes;
mod float_ord;
mod label;
mod name;
mod task_pool_options;
mod time;

pub use bytes::*;
pub use float_ord::*;
pub use label::*;
pub use name::*;
pub use task_pool_options::DefaultTaskPoolOptions;
pub use time::*;

pub mod prelude {
    pub use crate::{DefaultTaskPoolOptions, EntityLabels, Labels, Name, Time, Timer};
}

use bevy_app::prelude::*;
use bevy_ecs::{entity::Entity, system::IntoSystem};
use std::ops::Range;

/// Adds core functionality to Apps.
#[derive(Default)]
pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut AppBuilder) {
        // Setup the default bevy task pools
        app.world_mut()
            .get_resource::<DefaultTaskPoolOptions>()
            .cloned()
            .unwrap_or_else(DefaultTaskPoolOptions::default)
            .create_default_pools(app.world_mut());

        app.init_resource::<Time>()
            .init_resource::<EntityLabels>()
            .init_resource::<FixedTimesteps>()
            .register_type::<Entity>()
            .register_type::<Name>()
            .register_type::<Labels>()
            .register_type::<Range<f32>>()
            .register_type::<Timer>()
            .add_system_to_stage(CoreStage::First, time_system.system())
            .add_startup_system_to_stage(StartupStage::PostStartup, entity_labels_system.system())
            .add_system_to_stage(CoreStage::PostUpdate, entity_labels_system.system());

        register_rust_types(app);
        register_math_types(app);
    }
}

fn register_rust_types(app: &mut AppBuilder) {
    app.register_type::<bool>()
        .register_type::<u8>()
        .register_type::<u16>()
        .register_type::<u32>()
        .register_type::<u64>()
        .register_type::<u128>()
        .register_type::<usize>()
        .register_type::<i8>()
        .register_type::<i16>()
        .register_type::<i32>()
        .register_type::<i64>()
        .register_type::<i128>()
        .register_type::<isize>()
        .register_type::<f32>()
        .register_type::<f64>()
        .register_type::<String>()
        .register_type::<Option<String>>();
}

fn register_math_types(app: &mut AppBuilder) {
    app.register_type::<bevy_math::IVec2>()
        .register_type::<bevy_math::IVec3>()
        .register_type::<bevy_math::IVec4>()
        .register_type::<bevy_math::UVec2>()
        .register_type::<bevy_math::UVec3>()
        .register_type::<bevy_math::UVec4>()
        .register_type::<bevy_math::Vec2>()
        .register_type::<bevy_math::Vec3>()
        .register_type::<bevy_math::Vec4>()
        .register_type::<bevy_math::Mat3>()
        .register_type::<bevy_math::Mat4>()
        .register_type::<bevy_math::Quat>();
}
