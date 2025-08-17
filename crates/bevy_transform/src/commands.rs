//! Extension to [`EntityCommands`] to modify [`bevy_ecs::hierarchy`] hierarchies.
//! while preserving [`GlobalTransform`].

use crate::prelude::{GlobalTransform, Transform};
use bevy_ecs::{entity::Entity, hierarchy::ChildOf, system::EntityCommands, world::EntityWorldMut};

/// Collection of methods similar to the built-in parenting methods on [`EntityWorldMut`] and [`EntityCommands`], but preserving each
/// entity's [`GlobalTransform`].
pub trait BuildChildrenTransformExt {
    /// Change this entity's parent while preserving this entity's [`GlobalTransform`]
    /// by updating its [`Transform`].
    ///
    /// Insert the [`ChildOf`] component directly if you don't want to also update the [`Transform`].
    ///
    /// Note that both the hierarchy and transform updates will only execute
    /// the next time commands are applied
    /// (during [`ApplyDeferred`](bevy_ecs::schedule::ApplyDeferred)).
    fn set_parent_in_place(&mut self, parent: Entity) -> &mut Self;

    /// Make this entity parentless while preserving this entity's [`GlobalTransform`]
    /// by updating its [`Transform`] to be equal to its current [`GlobalTransform`].
    ///
    /// See [`EntityWorldMut::remove::<ChildOf>`] or [`EntityCommands::remove::<ChildOf>`] for a method that doesn't update the [`Transform`].
    ///
    /// Note that both the hierarchy and transform updates will only execute
    /// the next time commands are applied
    /// (during [`ApplyDeferred`](bevy_ecs::schedule::ApplyDeferred)).
    fn remove_parent_in_place(&mut self) -> &mut Self;
}

impl BuildChildrenTransformExt for EntityCommands<'_> {
    fn set_parent_in_place(&mut self, parent: Entity) -> &mut Self {
        self.queue(move |mut entity: EntityWorldMut| {
            entity.set_parent_in_place(parent);
        })
    }

    fn remove_parent_in_place(&mut self) -> &mut Self {
        self.queue(move |mut entity: EntityWorldMut| {
            entity.remove_parent_in_place();
        })
    }
}

impl BuildChildrenTransformExt for EntityWorldMut<'_> {
    fn set_parent_in_place(&mut self, parent: Entity) -> &mut Self {
        // FIXME: Replace this closure with a `try` block. See: https://github.com/rust-lang/rust/issues/31436.
        let mut update_transform = || {
            let child = self.id();
            let parent_global = self.world_scope(|world| {
                world
                    .get_entity_mut(parent)
                    .ok()?
                    .add_child(child)
                    .get::<GlobalTransform>()
                    .copied()
            })?;
            let child_global = self.get::<GlobalTransform>()?;
            let new_child_local = child_global.reparented_to(&parent_global);
            let mut child_local = self.get_mut::<Transform>()?;
            *child_local = new_child_local;
            Some(())
        };
        update_transform();
        self
    }

    fn remove_parent_in_place(&mut self) -> &mut Self {
        self.remove::<ChildOf>();
        // FIXME: Replace this closure with a `try` block. See: https://github.com/rust-lang/rust/issues/31436.
        let mut update_transform = || {
            let global = self.get::<GlobalTransform>()?;
            let new_local = global.compute_transform();
            let mut local = self.get_mut::<Transform>()?;
            *local = new_local;
            Some(())
        };
        update_transform();
        self
    }
}
