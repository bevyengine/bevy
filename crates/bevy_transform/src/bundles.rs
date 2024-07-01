use bevy_ecs::bundle::Bundle;

use crate::prelude::{GlobalTransform, Transform};

/// A [`Bundle`] of the [`Transform`] and [`GlobalTransform`]
/// [`Component`]s, which describe the position of an entity.
///
/// * To place or move an entity, you should set its [`Transform`].
/// * To get the global transform of an entity, you should get its [`GlobalTransform`].
/// * For transform hierarchies to work correctly, you must have both a [`Transform`] and a [`GlobalTransform`].
///   * You may use the [`TransformBundle`] to guarantee this.
///
/// ## [`Transform`] and [`GlobalTransform`]
///
/// [`Transform`] is the position of an entity relative to its parent position, or the reference
/// frame if it doesn't have a parent.
///
/// [`GlobalTransform`] is the position of an entity relative to the reference frame.
///
/// [`GlobalTransform`] is updated from [`Transform`] by systems in the system set
/// [`TransformPropagate`](TransformSystem::TransformPropagate).
///
/// This system runs during [`PostUpdate`]. If you
/// update the [`Transform`] of an entity in this schedule or after, you will notice a 1 frame lag
/// before the [`GlobalTransform`] is updated.
#[derive(Clone, Copy, Debug, Default, Bundle)]
pub struct TransformBundle {
    /// The transform of the entity.
    pub local: Transform,
    /// The global transform of the entity.
    pub global: GlobalTransform,
}

impl TransformBundle {
    /// An identity [`TransformBundle`] with no translation, rotation, and a scale of 1 on all axes.
    pub const IDENTITY: Self = TransformBundle {
        local: Transform::IDENTITY,
        global: GlobalTransform::IDENTITY,
    };

    /// Creates a new [`TransformBundle`] from a [`Transform`].
    ///
    /// This initializes [`GlobalTransform`] as identity, to be updated later by the
    /// [`PostUpdate`] schedule.
    #[inline]
    pub const fn from_transform(transform: Transform) -> Self {
        TransformBundle {
            local: transform,
            ..Self::IDENTITY
        }
    }
}

impl From<Transform> for TransformBundle {
    #[inline]
    fn from(transform: Transform) -> Self {
        Self::from_transform(transform)
    }
}
