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
        commands::BuildChildrenTransformExt, components::*, TransformBundle, TransformPlugin,
    };
}

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_hierarchy::ValidParentCheckPlugin;
use prelude::{GlobalTransform, Transform};
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
/// This system runs during [`CoreSet::PostUpdate`](crate::CoreSet::PostUpdate). If you
/// update the [`Transform`] of an entity in this stage or after, you will notice a 1 frame lag
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
    /// [`CoreSet::PostUpdate`](crate::CoreSet::PostUpdate) stage.
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
            .register_type::<GlobalTransform>()
            .add_plugin(ValidParentCheckPlugin::<GlobalTransform>::default())
            // add transform systems to startup so the first update is "correct"
            .configure_set(TransformSystem::TransformPropagate.in_base_set(CoreSet::PostUpdate))
            .edit_schedule(CoreSchedule::Startup, |schedule| {
                schedule.configure_set(
                    TransformSystem::TransformPropagate.in_base_set(StartupSet::PostStartup),
                );
            })
            // FIXME: https://github.com/bevyengine/bevy/issues/4381
            // These systems cannot access the same entities,
            // due to subtle query filtering that is not yet correctly computed in the ambiguity detector
            .add_startup_system(
                sync_simple_transforms
                    .in_set(TransformSystem::TransformPropagate)
                    .ambiguous_with(PropagateTransformsSet),
            )
            .add_startup_system(
                propagate_transforms
                    .in_set(TransformSystem::TransformPropagate)
                    .in_set(PropagateTransformsSet),
            )
            .add_system(
                sync_simple_transforms
                    .in_set(TransformSystem::TransformPropagate)
                    .ambiguous_with(PropagateTransformsSet),
            )
            .add_system(
                propagate_transforms
                    .in_set(TransformSystem::TransformPropagate)
                    .in_set(PropagateTransformsSet),
            );
    }
}
