#![warn(missing_docs)]
//! This crate provides core functionality for Bevy Engine.

mod name;

pub use bytemuck::{bytes_of, cast_slice, Pod, Zeroable};
pub use name::*;

pub mod prelude {
    //! The Bevy Core Prelude.
    #[doc(hidden)]
    pub use crate::Name;
}

use bevy_app::prelude::*;
use bevy_ecs::entity::Entity;
use bevy_tasks::TaskPool;
use bevy_utils::HashSet;
use std::ops::Range;

/// Adds core functionality to Apps.
#[derive(Default)]
pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        // Setup the default bevy task pool if not already set up
        app.world.init_resource::<TaskPool>();

        app.register_type::<Entity>().register_type::<Name>();

        register_rust_types(app);
        register_math_types(app);
    }
}

fn register_rust_types(app: &mut App) {
    app.register_type::<Range<f32>>()
        .register_type::<String>()
        .register_type::<HashSet<String>>()
        .register_type::<Option<String>>();
}

fn register_math_types(app: &mut App) {
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
