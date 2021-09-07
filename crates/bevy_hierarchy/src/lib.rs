pub mod components;
pub mod hierarchy;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{components::*, hierarchy::*};
}

use bevy_app::prelude::*;
use bevy_ecs::schedule::{ParallelSystemDescriptorCoercion, SystemLabel};
use prelude::{parent_update_system, Children, Parent, PreviousParent};

#[derive(Default)]
pub struct HierarchyPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub struct ParentUpdate;

impl Plugin for HierarchyPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Children>()
            .register_type::<Parent>()
            .register_type::<PreviousParent>()
            // add hierarchy system to startup so the first update is "correct"
            .add_startup_system_to_stage(
                StartupStage::PostStartup,
                parent_update_system.label(ParentUpdate),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                parent_update_system.label(ParentUpdate),
            );
    }
}
