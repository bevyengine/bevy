mod bytes;
mod float_ord;
mod label;
mod task_pool_options;
mod time;

use std::ops::Range;

use bevy_reflect::RegisterTypeBuilder;
pub use bytes::*;
pub use float_ord::*;
pub use label::*;
pub use task_pool_options::DefaultTaskPoolOptions;
pub use time::*;

pub mod prelude {
    pub use crate::{DefaultTaskPoolOptions, EntityLabels, Labels, Time, Timer};
}

use bevy_app::prelude::*;

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
            .register_type::<Option<String>>()
            .register_type::<Range<f32>>()
            .register_type::<Timer>()
            .add_system_to_stage(stage::FIRST, time_system)
            .add_system_to_stage(stage::PRE_UPDATE, entity_labels_system);
    }
}
