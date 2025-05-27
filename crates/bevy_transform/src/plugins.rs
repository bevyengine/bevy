use crate::systems::{mark_dirty_trees, propagate_parent_transforms, sync_simple_transforms};
use bevy_app::{App, Plugin, PostStartup, PostUpdate};
use bevy_ecs::schedule::{IntoScheduleConfigs, SystemSet};

use crate::components::{GlobalTransform, Transform2d, Transform3d, TransformTreeChanged};

/// Set enum for the systems relating to transform propagation
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum TransformSystems {
    /// Propagates changes in transform to children's [`GlobalTransform`](crate::components::GlobalTransform)
    Propagate,
}

/// Deprecated alias for [`TransformSystems`].
#[deprecated(since = "0.17.0", note = "Renamed to `TransformSystems`.")]
pub type TransformSystem = TransformSystems;

/// The base plugin for handling [`Transform3d`](crate::components::Transform) components
#[derive(Default)]
pub struct TransformPlugin;

impl Plugin for TransformPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "bevy_reflect")]
        app.register_type::<Transform3d>()
            .register_type::<TransformTreeChanged>()
            .register_type::<GlobalTransform>();

        app
            // add transform systems to startup so the first update is "correct"
            .add_systems(
                PostStartup,
                (mark_dirty_trees, propagate_parent_transforms)
                    .chain()
                    .in_set(TransformSystems::Propagate),
            )
            .add_systems(
                PostStartup,
                (
                    sync_simple_transforms::<Transform3d>,
                    sync_simple_transforms::<Transform2d>,
                )
                    .after(propagate_parent_transforms)
                    .in_set(TransformSystems::Propagate),
            )
            .add_systems(
                PostUpdate,
                (mark_dirty_trees, propagate_parent_transforms)
                    .chain()
                    .in_set(TransformSystems::Propagate),
            )
            .add_systems(
                PostUpdate,
                (
                    sync_simple_transforms::<Transform3d>,
                    sync_simple_transforms::<Transform2d>,
                )
                    .after(propagate_parent_transforms)
                    .in_set(TransformSystems::Propagate),
            );
    }
}
