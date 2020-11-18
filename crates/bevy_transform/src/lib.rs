pub mod components;
pub mod hierarchy;
pub mod transform_propagate_system;

pub mod prelude {
    pub use crate::{components::*, hierarchy::*, TransformPlugin};
}

use bevy_app::prelude::*;
use bevy_type_registry::RegisterType;
use prelude::{parent_update_system, Children, GlobalTransform, Parent, Transform};

#[derive(Default)]
pub struct TransformPlugin;

impl Plugin for TransformPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.register_component_with::<Children>(|reg| reg.map_entities())
            .register_component_with::<Parent>(|reg| reg.map_entities())
            .register_component::<Transform>()
            .register_component::<GlobalTransform>()
            // add transform systems to startup so the first update is "correct"
            .add_startup_system(parent_update_system)
            .add_startup_system(transform_propagate_system::transform_propagate_system)
            .add_system_to_stage(stage::POST_UPDATE, parent_update_system)
            .add_system_to_stage(
                stage::POST_UPDATE,
                transform_propagate_system::transform_propagate_system,
            );
    }
}
