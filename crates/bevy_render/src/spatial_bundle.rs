use bevy_ecs::prelude::Bundle;
use bevy_transform::prelude::{GlobalTransform, Transform};

use crate::view::{InheritedVisibility, ViewVisibility, Visibility};

/// A [`Bundle`] with the following [`Component`](bevy_ecs::component::Component)s:
/// * [`Visibility`], and [`InheritedVisibility`], which describe the visibility of an entity
/// * [`Transform`] and [`GlobalTransform`], which describe the position of an entity
///
/// * To show or hide an entity, you should set its [`Visibility`].
/// * To get the computed visibility of an entity, you should get its [`InheritedVisibility`] or [`ViewVisibility`] components.
/// * To place or move an entity, you should set its [`Transform`].
/// * To get the global transform of an entity, you should get its [`GlobalTransform`].
/// * For hierarchies to work correctly, you must have all four components.
///   * You may use the [`SpatialBundle`] to guarantee this.
#[derive(Bundle, Debug, Default)]
pub struct SpatialBundle {
    /// The visibility of the entity.
    pub visibility: Visibility,
    /// The inherited visibility of the entity.
    pub inherited_visibility: InheritedVisibility,
    /// The view visibility of the entity.
    pub view_visibility: ViewVisibility,
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
            ..Self::INHERITED_IDENTITY
        }
    }

    /// A visible [`SpatialBundle`], with no translation, rotation, and a scale of 1 on all axes.
    pub const INHERITED_IDENTITY: Self = SpatialBundle {
        visibility: Visibility::Inherited,
        inherited_visibility: InheritedVisibility::HIDDEN,
        view_visibility: ViewVisibility::HIDDEN,
        transform: Transform::IDENTITY,
        global_transform: GlobalTransform::IDENTITY,
    };

    /// An invisible [`SpatialBundle`], with no translation, rotation, and a scale of 1 on all axes.
    pub const HIDDEN_IDENTITY: Self = SpatialBundle {
        visibility: Visibility::Hidden,
        ..Self::INHERITED_IDENTITY
    };
}

impl From<Transform> for SpatialBundle {
    #[inline]
    fn from(transform: Transform) -> Self {
        Self::from_transform(transform)
    }
}
