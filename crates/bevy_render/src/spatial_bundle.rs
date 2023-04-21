use bevy_ecs::prelude::Bundle;
use bevy_transform::prelude::{GlobalTransform, GlobalTransform2d, Transform, Transform2d};

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
            ..Self::INHERITED_IDENTITY
        }
    }

    /// A visible [`SpatialBundle`], with no translation, rotation, and a scale of 1 on all axes.
    pub const INHERITED_IDENTITY: Self = SpatialBundle {
        visibility: Visibility::Inherited,
        computed: ComputedVisibility::HIDDEN,
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

/// A [`Bundle`] with the following [`Component`](bevy_ecs::component::Component)s:
/// * [`Visibility`] and [`ComputedVisibility`], which describe the visibility of an entity
/// * [`Transform2d`] and [`GlobalTransform2d`], which describe the position of an entity
///
/// * To show or hide an entity, you should set its [`Visibility`].
/// * To get the computed visibility of an entity, you should get its [`ComputedVisibility`].
/// * To place or move an entity, you should set its [`Transform2d`].
/// * To get the global transform of an entity, you should get its [`GlobalTransform2d`].
/// * For hierarchies to work correctly, you must have all four components.
///   * You may use the [`Spatial2dBundle`] to guarantee this.
#[derive(Bundle, Debug, Default)]
pub struct Spatial2dBundle {
    /// The visibility of the entity.
    pub visibility: Visibility,
    /// The computed visibility of the entity.
    pub computed: ComputedVisibility,
    /// The transform of the entity.
    pub transform: Transform2d,
    /// The global transform of the entity.
    pub global_transform: GlobalTransform2d,
}

impl Spatial2dBundle {
    /// Creates a new [`Spatial2dBundle`] from a [`Transform`].
    ///
    /// This initializes [`GlobalTransform`] as identity, and visibility as visible
    #[inline]
    pub const fn from_transform(transform: Transform2d) -> Self {
        Spatial2dBundle {
            transform,
            ..Self::INHERITED_IDENTITY
        }
    }

    /// A visible [`Spatial2dBundle`], with no translation, rotation, and a scale of 1 on all axes.
    pub const INHERITED_IDENTITY: Self = Spatial2dBundle {
        visibility: Visibility::Inherited,
        computed: ComputedVisibility::HIDDEN,
        transform: Transform2d::IDENTITY,
        global_transform: GlobalTransform2d::IDENTITY,
    };

    /// An invisible [`Spatial2dBundle`], with no translation, rotation, and a scale of 1 on all axes.
    pub const HIDDEN_IDENTITY: Self = Spatial2dBundle {
        visibility: Visibility::Hidden,
        ..Self::INHERITED_IDENTITY
    };
}

impl From<Transform2d> for Spatial2dBundle {
    #[inline]
    fn from(transform: Transform2d) -> Self {
        Self::from_transform(transform)
    }
}
