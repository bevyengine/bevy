mod bytes;
mod float_ord;
mod label;
mod name;
mod task_pool_options;
mod time;

use std::ops::Range;

use bevy_ecs::IntoSystem;
use bevy_reflect::RegisterTypeBuilder;
pub use bytes::*;
pub use float_ord::*;
pub use label::*;
pub use name::*;
pub use task_pool_options::DefaultTaskPoolOptions;
pub use time::*;

pub mod prelude {
    pub use crate::{DefaultTaskPoolOptions, EntityLabels, Labels, Name, Time, Timer};
}

use bevy_app::{prelude::*, startup_stage};

/// Adds core functionality to Apps.
#[derive(Default)]
pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut AppBuilder) {
        // Setup the default bevy task pools
        app.resources_mut()
            .get_cloned::<DefaultTaskPoolOptions>()
            .unwrap_or_else(DefaultTaskPoolOptions::default)
            .create_default_pools(app.resources_mut());

        app.init_resource::<Time>()
            .init_resource::<EntityLabels>()
            .init_resource::<FixedTimesteps>()
            .register_type::<Option<String>>()
            .register_type::<Name>()
            .register_type::<Labels>()
            .register_type::<Range<f32>>()
            .register_type::<Timer>()
            .add_system_to_stage(stage::FIRST, time_system.system())
            .add_startup_system_to_stage(startup_stage::POST_STARTUP, entity_labels_system.system())
            .add_system_to_stage(stage::POST_UPDATE, entity_labels_system.system());
    }
}
