#![warn(missing_docs)]
//! This crate provides core functionality for Bevy Engine.

mod name;
mod task_pool_options;

pub use bytemuck::{bytes_of, cast_slice, Pod, Zeroable};
pub use name::*;
pub use task_pool_options::*;

pub mod prelude {
    //! The Bevy Core Prelude.
    #[doc(hidden)]
    pub use crate::{DefaultTaskPoolOptions, Name};
}

use bevy_app::prelude::*;
use bevy_ecs::entity::Entity;
use bevy_reflect::TypeRegistryArc;

/// Adds core functionality to Apps.
#[derive(Default)]
pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        // Setup the default bevy task pools
        app.world
            .get_resource::<DefaultTaskPoolOptions>()
            .cloned()
            .unwrap_or_default()
            .create_default_pools();

        app.register_type::<Entity>().register_type::<Name>();

        let registry = app.world.resource::<TypeRegistryArc>();
        let mut registry = registry.write();
        rust_types::register_types(&mut registry);
        math_types::register_types(&mut registry);
    }
}

mod rust_types {
    use bevy_reflect::erased_serde::Serialize;
    use bevy_reflect::register_all;
    use bevy_utils::HashSet;
    use std::ops::Range;

    register_all! {
        traits: [Serialize],
        types: [
            String,
            Option<String>,
            Range<f32>,
            HashSet<String>
        ]
    }
}

mod math_types {
    use bevy_reflect::erased_serde::Serialize;
    use bevy_reflect::register_all;

    register_all! {
        traits: [Serialize],
        types: [
            bevy_math::IVec2,
            bevy_math::IVec3,
            bevy_math::IVec4,
            bevy_math::UVec2,
            bevy_math::UVec3,
            bevy_math::UVec4,
            bevy_math::Vec2,
            bevy_math::Vec3,
            bevy_math::Vec4,
            bevy_math::Mat3,
            bevy_math::Mat4,
            bevy_math::Quat,
        ]
    }
}
