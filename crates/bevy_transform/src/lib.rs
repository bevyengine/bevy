#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

/// The basic components of the transform crate
pub mod components;
/// Establishing and updating the transform hierarchy
pub mod hierarchy;
/// Propagating transform changes down the transform hierarchy
pub mod transform_propagate_system;

#[doc(hidden)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{components::*, hierarchy::*, TransformPlugin};
}

use bevy_app::prelude::*;
use bevy_ecs::schedule::{ParallelSystemDescriptorCoercion, SystemLabel};
use prelude::{parent_update_system, Children, GlobalTransform, Parent, PreviousParent, Transform};

/// The base plugin for handling [`Transform`] components
#[derive(Default)]
pub struct TransformPlugin;

/// Label enum for the types of systems relating to transform
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum TransformSystem {
    /// Propagates changes in transform to childrens' [`GlobalTransform`]
    TransformPropagate,
    /// Updates [`Parent`] when changes in the hierarchy occur
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
            .add_system_to_stage(
                CoreStage::PostUpdate,
                parent_update_system.label(TransformSystem::ParentUpdate),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                transform_propagate_system::transform_propagate_system
                    .label(TransformSystem::TransformPropagate)
                    .after(TransformSystem::ParentUpdate),
            );
    }
}
