pub mod components;
pub mod hierarchy;
pub mod transform_propagate_system;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{components::*, hierarchy::*, TransformPlugin};
}

use bevy_app::prelude::*;
use bevy_ecs::{prelude::{StageConfig, ScheduleConfig}, schedule::SystemLabel};
use prelude::{parent_update_system, Children, GlobalTransform, Parent, PreviousParent, Transform};

#[derive(Default)]
pub struct TransformPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum TransformSystem {
    TransformPropagate,
    ParentUpdate,
}

impl Plugin for TransformPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Children>()
            .register_type::<Parent>()
            .register_type::<PreviousParent>()
            .register_type::<Transform>()
            .register_type::<GlobalTransform>()
            // add transform systems to startup so the first update is "correct"
            .add_startup_system_to_stage(
                StartupStage::PostStartup,
                parent_update_system.label(TransformSystem::ParentUpdate),
            )
            .add_startup_system_to_stage(
                StartupStage::PostStartup,
                transform_propagate_system::transform_propagate_system
                    .label(TransformSystem::TransformPropagate)
                    .after(TransformSystem::ParentUpdate),
            )
            .add_system(
                parent_update_system
                    .stage(CoreStage::PostUpdate)
                    .label(TransformSystem::ParentUpdate),
            )
            .add_system(
                transform_propagate_system::transform_propagate_system
                    .stage(CoreStage::PostUpdate)
                    .label(TransformSystem::TransformPropagate)
                    .after(TransformSystem::ParentUpdate),
            );
    }
}
