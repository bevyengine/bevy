use crate::systems::{
    compute_transform_leaves, propagate_parent_transforms, sync_simple_transforms,
};
use bevy_app::{App, Plugin, PostStartup, PostUpdate};
use bevy_ecs::schedule::{IntoScheduleConfigs, SystemSet};

/// Set enum for the systems relating to transform propagation
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum TransformSystem {
    /// Propagates changes in transform to children's [`GlobalTransform`](crate::components::GlobalTransform)
    TransformPropagate,
}

/// The base plugin for handling [`Transform`](crate::components::Transform) components
#[derive(Default)]
pub struct TransformPlugin;

impl Plugin for TransformPlugin {
    fn build(&self, app: &mut App) {
        // A set for `propagate_transforms` to mark it as ambiguous with `sync_simple_transforms`.
        // Used instead of the `SystemTypeSet` as that would not allow multiple instances of the system.
        #[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
        struct PropagateTransformsSet;

        #[cfg(feature = "bevy_reflect")]
        app.register_type::<crate::components::Transform>()
            .register_type::<crate::components::GlobalTransform>();

        app.configure_sets(
            PostStartup,
            PropagateTransformsSet.in_set(TransformSystem::TransformPropagate),
        )
        // add transform systems to startup so the first update is "correct"
        .add_systems(
            PostStartup,
            (
                propagate_parent_transforms,
                (compute_transform_leaves, sync_simple_transforms)
                    .ambiguous_with(TransformSystem::TransformPropagate),
            )
                .chain()
                .in_set(PropagateTransformsSet),
        )
        .configure_sets(
            PostUpdate,
            PropagateTransformsSet.in_set(TransformSystem::TransformPropagate),
        )
        .add_systems(
            PostUpdate,
            (
                propagate_parent_transforms,
                (compute_transform_leaves, sync_simple_transforms) // TODO: Adjust the internal parallel queries to make these parallel systems more efficiently share and fill CPU time.
                    .ambiguous_with(TransformSystem::TransformPropagate),
            )
                .chain()
                .in_set(PropagateTransformsSet),
        );
    }
}
