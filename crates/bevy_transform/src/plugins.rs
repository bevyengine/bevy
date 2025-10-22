use crate::systems::{mark_dirty_trees, propagate_parent_transforms, sync_simple_transforms};
use bevy_app::{App, Plugin, PostStartup, PostUpdate};
use bevy_ecs::schedule::{IntoScheduleConfigs, SystemSet};

/// Set enum for the systems relating to transform propagation
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum TransformSystems {
    /// Propagates changes in transform to children's [`GlobalTransform`](crate::components::GlobalTransform)
    Propagate,
}

/// Deprecated alias for [`TransformSystems`].
#[deprecated(since = "0.17.0", note = "Renamed to `TransformSystems`.")]
pub type TransformSystem = TransformSystems;

/// The base plugin for handling [`Transform`](crate::components::Transform) components
#[derive(Default)]
pub struct TransformPlugin;

impl Plugin for TransformPlugin {
    fn build(&self, app: &mut App) {
        app
            // add transform systems to startup so the first update is "correct"
            .add_systems(
                PostStartup,
                (
                    mark_dirty_trees,
                    propagate_parent_transforms,
                    sync_simple_transforms,
                )
                    .chain()
                    .in_set(TransformSystems::Propagate),
            )
            .add_systems(
                PostUpdate,
                (
                    mark_dirty_trees,
                    propagate_parent_transforms,
                    // TODO: Adjust the internal parallel queries to make this system more efficiently share and fill CPU time.
                    sync_simple_transforms,
                )
                    .chain()
                    .in_set(TransformSystems::Propagate),
            );
    }
}
