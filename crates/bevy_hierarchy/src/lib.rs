#![warn(missing_docs)]
//! `bevy_hierarchy` can be used to define hierarchies of entities.
//!
//! Most commonly, these hierarchies are used for inheriting [`Transform`](bevy_transform::components::Transform) values
//! from the [`Parent`] to its [`Children`].

/// The basic components of the hierarchy
pub mod components;
/// Establishing and updating the transform hierarchy
pub mod hierarchy;
/// Propagating transform changes down the transform hierarchy
pub mod transform_propagate_system;

#[doc(hidden)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{components::*, hierarchy::*, HierarchyPlugin};
}

use bevy_app::prelude::*;
use bevy_ecs::schedule::{ParallelSystemDescriptorCoercion, SystemLabel};
use prelude::{parent_update_system, Children, Parent, PreviousParent};

/// The base plugin for handling [`Parent`] and [`Children`] components
#[derive(Default)]
pub struct HierarchyPlugin;

/// Label enum for the types of systems relating to transform
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum HierarchySystem {
    /// Propagates changes in transform to childrens' [`GlobalTransform`]
    TransformPropagate,
    /// Updates [`Parent`] when changes in the hierarchy occur
    ParentUpdate,
}

impl Plugin for HierarchyPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Children>()
            .register_type::<Parent>()
            .register_type::<PreviousParent>()
            // add transform systems to startup so the first update is "correct"
            .add_startup_system_to_stage(
                StartupStage::PostStartup,
                parent_update_system.label(HierarchySystem::ParentUpdate),
            )
            .add_startup_system_to_stage(
                StartupStage::PostStartup,
                transform_propagate_system::transform_propagate_system
                    .label(HierarchySystem::TransformPropagate)
                    .after(HierarchySystem::ParentUpdate),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                parent_update_system.label(HierarchySystem::ParentUpdate),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                transform_propagate_system::transform_propagate_system
                    .label(HierarchySystem::TransformPropagate)
                    .after(HierarchySystem::ParentUpdate),
            );
    }
}
