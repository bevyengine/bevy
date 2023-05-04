#![allow(clippy::type_complexity)]
#![warn(missing_docs)]
#![warn(clippy::undocumented_unsafe_blocks)]
#![doc = include_str!("../README.md")]

pub mod commands;
/// The basic components of the transform crate
pub mod components;
/// Systems responsible for transform propagation
pub mod systems;

#[doc(hidden)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        commands::BuildChildrenTransformExt, components::*, Transform2dBundle, TransformBundle,
        TransformPlugin, TransformPoint,
    };
}

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_hierarchy::ValidParentCheckPlugin;
use bevy_math::{Affine3A, Mat4, Vec3};

use components::{GlobalTransform, GlobalTransform2d, Transform, Transform2d};
use systems::{propagate_transforms, sync_simple_transforms};

/// A [`Bundle`] of the [`Transform`] and [`GlobalTransform`]
/// [`Component`](bevy_ecs::component::Component)s, which describe the position of an entity.
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
/// [`TransformPropagate`](crate::TransformSystem::TransformPropagate).
///
/// This system runs during [`PostUpdate`](bevy_app::PostUpdate). If you
/// update the [`Transform`] of an entity in this schedule or after, you will notice a 1 frame lag
/// before the [`GlobalTransform`] is updated.
#[derive(Bundle, Clone, Copy, Debug, Default)]
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
    /// [`PostUpdate`](bevy_app::PostUpdate) schedule.
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

/// A [`Bundle`] of the [`Transform2d`] and [`GlobalTransform2d`]
/// [`Component`](bevy_ecs::component::Component)s, which describe the position of an entity.
///
/// * To place or move an entity, you should set its [`Transform2d`].
/// * To get the global transform of an entity, you should get its [`GlobalTransform2d`].
/// * For transform hierarchies to work correctly, you must have both a [`Transform2d`] and a [`GlobalTransform2d`].
///   * You may use the [`TransformBundle`] to guarantee this.
///
/// ## [`Transform2d`] and [`GlobalTransform2d`]
///
/// [`Transform2d`] is the position of an entity relative to its parent position, or the reference
/// frame if it doesn't have a parent.
///
/// [`GlobalTransform2d`] is the position of an entity relative to the reference frame.
///
/// [`GlobalTransform2d`] is updated from [`Transform2d`] by systems in the system set
/// [`TransformPropagate`](crate::TransformSystem::TransformPropagate).
///
/// This system runs during [`PostUpdate`](bevy_app::PostUpdate). If you
/// update the [`Transform2d`] of an entity in this schedule or after, you will notice a 1 frame lag
/// before the [`GlobalTransform2d`] is updated.
#[derive(Bundle, Clone, Copy, Debug, Default)]
pub struct Transform2dBundle {
    /// The transform of the entity.
    pub local: Transform2d,
    /// The global transform of the entity.
    pub global: GlobalTransform2d,
}

impl Transform2dBundle {
    /// An identity [`Transform2dBundle`] with no translation, rotation, and a scale of 1 on all axes.
    pub const IDENTITY: Self = Transform2dBundle {
        local: Transform2d::IDENTITY,
        global: GlobalTransform2d::IDENTITY,
    };

    /// Creates a new [`Transform2dBundle`] from a [`Transform2d`].
    ///
    /// This initializes [`GlobalTransform2d`] as identity, to be updated later by the
    /// [`PostUpdate`](bevy_app::PostUpdate) schedule.
    #[inline]
    pub const fn from_transform(transform: Transform2d) -> Self {
        Transform2dBundle {
            local: transform,
            ..Self::IDENTITY
        }
    }
}

impl From<Transform2d> for Transform2dBundle {
    #[inline]
    fn from(transform: Transform2d) -> Self {
        Self::from_transform(transform)
    }
}

/// Set enum for the systems relating to transform propagation
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum TransformSystem {
    /// Propagates changes in transform to children's [`GlobalTransform`](crate::components::GlobalTransform)
    TransformPropagate,
}

/// The base plugin for handling [`Transform`] components
#[derive(Default)]
pub struct TransformPlugin;

impl Plugin for TransformPlugin {
    fn build(&self, app: &mut App) {
        // A set for `propagate_transforms` to mark it as ambiguous with `sync_simple_transforms`.
        // Used instead of the `SystemTypeSet` as that would not allow multiple instances of the system.
        #[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
        struct PropagateTransformsSet;

        app.register_type::<Transform>()
            .register_type::<Transform2d>()
            .register_type::<GlobalTransform>()
            .register_type::<GlobalTransform2d>()
            .add_plugin(ValidParentCheckPlugin::<GlobalTransform>::default())
            .add_plugin(ValidParentCheckPlugin::<GlobalTransform2d>::default())
            .configure_set(
                PostStartup,
                PropagateTransformsSet.in_set(TransformSystem::TransformPropagate),
            )
            // add transform systems to startup so the first update is "correct"
            .add_systems(
                PostStartup,
                (
                    sync_simple_transforms::<Transform, GlobalTransform>
                        .in_set(TransformSystem::TransformPropagate)
                        // FIXME: https://github.com/bevyengine/bevy/issues/4381
                        // These systems cannot access the same entities,
                        // due to subtle query filtering that is not yet correctly computed in the ambiguity detector
                        .ambiguous_with(PropagateTransformsSet),
                    propagate_transforms::<Transform, GlobalTransform>
                        .in_set(PropagateTransformsSet),
                    sync_simple_transforms::<Transform2d, GlobalTransform2d>
                        .in_set(TransformSystem::TransformPropagate)
                        .ambiguous_with(PropagateTransformsSet),
                    propagate_transforms::<Transform2d, GlobalTransform2d>
                        .in_set(PropagateTransformsSet),
                ),
            )
            .configure_set(
                PostUpdate,
                PropagateTransformsSet.in_set(TransformSystem::TransformPropagate),
            )
            .add_systems(
                PostUpdate,
                (
                    sync_simple_transforms::<Transform, GlobalTransform>
                        .in_set(TransformSystem::TransformPropagate)
                        .ambiguous_with(PropagateTransformsSet),
                    propagate_transforms::<Transform, GlobalTransform>
                        .in_set(PropagateTransformsSet),
                    sync_simple_transforms::<Transform2d, GlobalTransform2d>
                        .in_set(TransformSystem::TransformPropagate)
                        .ambiguous_with(PropagateTransformsSet),
                    propagate_transforms::<Transform2d, GlobalTransform2d>
                        .in_set(PropagateTransformsSet),
                ),
            );
    }
}

/// A trait for point transformation methods.
pub trait TransformPoint {
    /// Transform a point.
    fn transform_point(&self, point: impl Into<Vec3>) -> Vec3;
}

impl TransformPoint for Transform {
    #[inline]
    fn transform_point(&self, point: impl Into<Vec3>) -> Vec3 {
        self.transform_point(point.into())
    }
}

impl TransformPoint for GlobalTransform {
    #[inline]
    fn transform_point(&self, point: impl Into<Vec3>) -> Vec3 {
        self.transform_point(point.into())
    }
}

impl TransformPoint for Mat4 {
    #[inline]
    fn transform_point(&self, point: impl Into<Vec3>) -> Vec3 {
        self.transform_point3(point.into())
    }
}

impl TransformPoint for Affine3A {
    #[inline]
    fn transform_point(&self, point: impl Into<Vec3>) -> Vec3 {
        self.transform_point3(point.into())
    }
}
