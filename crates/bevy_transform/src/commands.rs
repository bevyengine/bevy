//! Extension to [`EntityCommands`] to modify `bevy_hierarchy` hierarchies
//! while preserving [`GlobalTransform`].

use crate::prelude::{GlobalTransform, Transform};
use bevy_ecs::{prelude::Entity, system::EntityCommands, world::Command, world::World};
use bevy_hierarchy::{PushChild, RemoveParent};

/// Command similar to [`PushChild`], but updating the child transform to keep
/// it at the same [`GlobalTransform`].
///
/// You most likely want to use [`BuildChildrenTransformExt::set_parent_in_place`]
/// method on [`EntityCommands`] instead.
pub struct PushChildInPlace {
    /// Parent entity to add the child to.
    pub parent: Entity,
    /// Child entity to add.
    pub child: Entity,
}
impl Command for PushChildInPlace {
    fn apply(self, world: &mut World) {
        let hierarchy_command = PushChild {
            child: self.child,
            parent: self.parent,
        };
        hierarchy_command.apply(world);
        // FIXME: Replace this closure with a `try` block. See: https://github.com/rust-lang/rust/issues/31436.
        let mut update_transform = || {
            let parent = *world.get_entity(self.parent)?.get::<GlobalTransform>()?;
            let child_global = *world.get_entity(self.child)?.get::<GlobalTransform>()?;
            let mut child_entity = world.get_entity_mut(self.child)?;
            let mut child = child_entity.get_mut::<Transform>()?;
            *child = child_global.reparented_to(&parent);
            Some(())
        };
        update_transform();
    }
}
/// Command similar to [`RemoveParent`], but updating the child transform to keep
/// it at the same [`GlobalTransform`].
///
/// You most likely want to use [`BuildChildrenTransformExt::remove_parent_in_place`]
/// method on [`EntityCommands`] instead.
pub struct RemoveParentInPlace {
    /// [`Entity`] whose parent must be removed.
    pub child: Entity,
}
impl Command for RemoveParentInPlace {
    fn apply(self, world: &mut World) {
        let hierarchy_command = RemoveParent { child: self.child };
        hierarchy_command.apply(world);
        // FIXME: Replace this closure with a `try` block. See: https://github.com/rust-lang/rust/issues/31436.
        let mut update_transform = || {
            let child_global = *world.get_entity(self.child)?.get::<GlobalTransform>()?;
            let mut child_entity = world.get_entity_mut(self.child)?;
            let mut child = child_entity.get_mut::<Transform>()?;
            *child = child_global.compute_transform();
            Some(())
        };
        update_transform();
    }
}
/// Collection of methods similar to [`BuildChildren`](bevy_hierarchy::BuildChildren), but preserving each
/// entity's [`GlobalTransform`].
pub trait BuildChildrenTransformExt {
    /// Change this entity's parent while preserving this entity's [`GlobalTransform`]
    /// by updating its [`Transform`].
    ///
    /// See [`BuildChildren::set_parent`](bevy_hierarchy::BuildChildren::set_parent) for a method that doesn't update the
    /// [`Transform`].
    ///
    /// Note that both the hierarchy and transform updates will only execute
    /// the next time commands are applied
    /// (during [`apply_deferred`](bevy_ecs::schedule::apply_deferred)).
    fn set_parent_in_place(&mut self, parent: Entity) -> &mut Self;

    /// Make this entity parentless while preserving this entity's [`GlobalTransform`]
    /// by updating its [`Transform`] to be equal to its current [`GlobalTransform`].
    ///
    /// See [`BuildChildren::remove_parent`](bevy_hierarchy::BuildChildren::remove_parent) for a method that doesn't update the
    /// [`Transform`].
    ///
    /// Note that both the hierarchy and transform updates will only execute
    /// the next time commands are applied
    /// (during [`apply_deferred`](bevy_ecs::schedule::apply_deferred)).
    fn remove_parent_in_place(&mut self) -> &mut Self;
}
impl BuildChildrenTransformExt for EntityCommands<'_> {
    fn set_parent_in_place(&mut self, parent: Entity) -> &mut Self {
        let child = self.id();
        self.commands().add(PushChildInPlace { child, parent });
        self
    }

    fn remove_parent_in_place(&mut self) -> &mut Self {
        let child = self.id();
        self.commands().add(RemoveParentInPlace { child });
        self
    }
}
