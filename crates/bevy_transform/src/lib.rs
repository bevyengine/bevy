pub mod components;
pub mod hierarchy;
pub mod transform_propagate_system;

pub mod prelude {
    pub use crate::{components::*, hierarchy::*, TransformPlugin};
}

use bevy_app::{prelude::*, startup_stage};
use bevy_ecs::{IntoSystem, SystemStage};
use bevy_reflect::RegisterTypeBuilder;
use prelude::{parent_update_system, Children, GlobalTransform, Parent, PreviousParent, Transform};

pub mod stage {
    pub const STARTUP_UPDATE_HIERARCHY: &str = "transform_startup_update_hierarchy";
    pub const UPDATE_HIERARCHY: &str = "transform_update_hierarchy";
}

#[derive(Default)]
pub struct TransformPlugin;

impl Plugin for TransformPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.register_type::<Children>()
            .register_type::<Parent>()
            .register_type::<PreviousParent>()
            .register_type::<Transform>()
            .register_type::<GlobalTransform>()
            // add transform systems to startup so the first update is "correct"
            .add_startup_stage_after(
                startup_stage::STARTUP,
                stage::STARTUP_UPDATE_HIERARCHY,
                SystemStage::parallel(),
            )
            .add_startup_system_to_stage(
                stage::STARTUP_UPDATE_HIERARCHY,
                parent_update_system.system(),
            )
            .add_startup_system_to_stage(
                startup_stage::POST_STARTUP,
                transform_propagate_system::transform_propagate_system.system(),
            )
            .add_stage_after(
                bevy_app::stage::UPDATE,
                stage::UPDATE_HIERARCHY,
                SystemStage::parallel(),
            )
            .add_system_to_stage(stage::UPDATE_HIERARCHY, parent_update_system.system())
            .add_system_to_stage(
                bevy_app::stage::POST_UPDATE,
                transform_propagate_system::transform_propagate_system.system(),
            );
    }
}
