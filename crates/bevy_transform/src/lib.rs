pub mod components;
pub mod hierarchy;
pub mod transform_propagate_system;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{components::*, hierarchy::*, TransformBundle, TransformPlugin};
}

use bevy_app::prelude::*;
use bevy_ecs::{
    bundle::Bundle,
    schedule::{ParallelSystemDescriptorCoercion, SystemLabel},
};
use prelude::{parent_update_system, Children, GlobalTransform, Parent, PreviousParent, Transform};

/// A [`Bundle`] of the [`Transform`] and [`GlobalTransform`]
/// [`Component`](bevy_ecs::component::Component)s, which describe the position of an entity.
///
/// * To place or move an entity, you should set its [`Transform`].
/// * To get the global position of an entity, you should get its [`GlobalTransform`].
/// * To be displayed, an entity must have both a [`Transform`] and a [`GlobalTransform`].
///   * You may use the [`TransformBundle`] to guaranty this.
///
/// ## [`Transform`] and [`GlobalTransform`]
///
/// [`Transform`] is the position of an entity relative to its parent position, or the reference
/// frame if it doesn't have a [`Parent`](Parent).
///
/// [`GlobalTransform`] is the position of an entity relative to the reference frame.
///
/// [`GlobalTransform`] is updated from [`Transform`] in the system
/// [`transform_propagate_system`](crate::transform_propagate_system::transform_propagate_system).
///
/// In pseudo code:
/// ```ignore
/// for entity in entities_without_parent:
///     set entity.global_transform to entity.transform
///     recursively:
///         set parent to current entity
///         for child in parent.children:
///             set child.global_transform to parent.global_transform * child.transform
/// ```
///
/// This system runs in stage [`CoreStage::PostUpdate`](crate::CoreStage::PostUpdate). If you
/// update the[`Transform`] of an entity in this stage or after, you will notice a 1 frame lag
/// before the [`GlobalTransform`] is updated.
#[derive(Default, Bundle, Clone, Debug)]
pub struct TransformBundle {
    pub local: Transform,
    pub global: GlobalTransform,
}

impl TransformBundle {
    /// Creates a new [`TransformBundle`] from a [`Transform`] and a [`GlobalTransform`].
    pub fn new(local: Transform, global: GlobalTransform) -> Self {
        TransformBundle { local, global }
    }

    /// Creates a new [`TransformBundle`] from a [`Transform`] and leaving [`GlobalTransform`] with
    /// no translation, rotation, and a scale of 1 on all axes.
    #[inline]
    pub fn from_transform(transform: Transform) -> Self {
        TransformBundle {
            local: transform,
            ..Default::default()
        }
    }

    /// Creates a new identity [`TransformBundle`], with no translation, rotation, and a scale of 1
    /// on all axes.
    #[inline]
    pub const fn identity() -> Self {
        TransformBundle {
            local: Transform::identity(),
            // Note: `..Default::default()` cannot be used here, because it isn't const
            global: GlobalTransform::identity(),
        }
    }
}

impl From<Transform> for TransformBundle {
    fn from(transform: Transform) -> Self {
        Self::from_transform(transform)
    }
}

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
