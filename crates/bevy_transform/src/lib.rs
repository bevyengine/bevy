pub mod components;
pub mod transform_propagate_system;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{components::*, TransformPlugin};
}

use bevy_app::prelude::*;
use bevy_ecs::schedule::{ParallelSystemDescriptorCoercion, SystemLabel};
use bevy_hierarchy::ParentUpdate;
use prelude::{GlobalTransform, Transform};

#[derive(Default)]
pub struct TransformPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub struct TransformPropagate;

impl Plugin for TransformPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Transform>()
            .register_type::<GlobalTransform>()
            // add transform systems to startup so the first update is "correct"
            .add_startup_system_to_stage(
                StartupStage::PostStartup,
                transform_propagate_system::transform_propagate_system
                    .label(TransformPropagate)
                    .after(ParentUpdate),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                transform_propagate_system::transform_propagate_system
                    .label(TransformPropagate)
                    .after(ParentUpdate),
            );
    }
}
