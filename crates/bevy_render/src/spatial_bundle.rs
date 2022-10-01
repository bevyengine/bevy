use bevy_ecs::prelude::Bundle;
use bevy_transform::prelude::{GlobalTransform, Transform};

use crate::view::{ComputedVisibility, Visibility};

/// A [`Bundle`] with the following [`Component`](bevy_ecs::component::Component)s:
/// * [`Visibility`] and [`ComputedVisibility`], which describe the visibility of an entity
/// * [`Transform`] and [`GlobalTransform`], which describe the position of an entity
///
/// * To show or hide an entity, you should set its [`Visibility`].
/// * To get the computed visibility of an entity, you should get its [`ComputedVisibility`].
/// * To place or move an entity, you should set its [`Transform`].
/// * To get the global transform of an entity, you should get its [`GlobalTransform`].
/// * For hierarchies to work correctly, you must have all four components.
///   * You may use the [`SpatialBundle`] to guarantee this.
#[derive(Bundle, Debug, Default)]
pub struct SpatialBundle {
    /// The visibility of the entity.
    pub visibility: Visibility,
    /// The computed visibility of the entity.
    pub computed: ComputedVisibility,
    /// The transform of the entity.
    pub transform: Transform,
    /// The global transform of the entity.
    pub global_transform: GlobalTransform,
}

impl SpatialBundle {
    /// Creates a new [`SpatialBundle`] from a [`Transform`].
    ///
    /// This initializes [`GlobalTransform`] as identity, and visibility as visible
    #[inline]
    pub const fn from_transform(transform: Transform) -> Self {
        SpatialBundle {
            transform,
            ..Self::VISIBLE_IDENTITY
        }
    }

    /// A visible [`SpatialBundle`], with no translation, rotation, and a scale of 1 on all axes.
    pub const VISIBLE_IDENTITY: Self = SpatialBundle {
        visibility: Visibility::VISIBLE,
        computed: ComputedVisibility::INVISIBLE,
        transform: Transform::IDENTITY,
        global_transform: GlobalTransform::IDENTITY,
    };

    /// An invisible [`SpatialBundle`], with no translation, rotation, and a scale of 1 on all axes.
    pub const INVISIBLE_IDENTITY: Self = SpatialBundle {
        visibility: Visibility::INVISIBLE,
        ..Self::VISIBLE_IDENTITY
    };
}

impl From<Transform> for SpatialBundle {
    #[inline]
    fn from(transform: Transform) -> Self {
        Self::from_transform(transform)
    }
}
