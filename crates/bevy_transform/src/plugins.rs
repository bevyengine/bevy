use crate::systems::{propagate_transforms_par, sync_simple_transforms};
use bevy_app::{App, Plugin, PostStartup, PostUpdate};
use bevy_ecs::schedule::{IntoSystemConfigs, IntoSystemSetConfigs, SystemSet};

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
                sync_simple_transforms
                    .in_set(TransformSystem::TransformPropagate)
                    // FIXME: https://github.com/bevyengine/bevy/issues/4381
                    // These systems cannot access the same entities,
                    // due to subtle query filtering that is not yet correctly computed in the ambiguity detector
                    .ambiguous_with(PropagateTransformsSet),
                // propagate_transforms.in_set(PropagateTransformsSet),
                propagate_transforms_par.in_set(PropagateTransformsSet),
            ),
        )
        .configure_sets(
            PostUpdate,
            PropagateTransformsSet.in_set(TransformSystem::TransformPropagate),
        )
        .add_systems(
            PostUpdate,
            (
                sync_simple_transforms
                    .in_set(TransformSystem::TransformPropagate)
                    .ambiguous_with(PropagateTransformsSet),
                // propagate_transforms.in_set(PropagateTransformsSet),
                propagate_transforms_par.in_set(PropagateTransformsSet),
            ),
        );
    }
}
