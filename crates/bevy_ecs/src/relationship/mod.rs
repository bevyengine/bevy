//! This module provides functionality to link entities to each other using specialized components called "relationships". See the [`Relationship`] trait for more info.

mod related_methods;
mod relationship_query;
mod relationship_source_collection;

use alloc::format;

pub use related_methods::*;
pub use relationship_query::*;
pub use relationship_source_collection::*;

use crate::{
    component::{Component, HookContext, Mutable},
    entity::{ComponentCloneCtx, Entity},
    system::{
        command::HandleError,
        entity_command::{self, CommandWithEntity},
        error_handler, Commands,
    },
    world::{DeferredWorld, EntityWorldMut},
};
use log::warn;

/// A [`Component`] on a "source" [`Entity`] that references another target [`Entity`], creating a "relationship" between them. Every [`Relationship`]
/// has a corresponding [`RelationshipTarget`] type (and vice-versa), which exists on the "target" entity of a relationship and contains the list of all
/// "source" entities that relate to the given "target"
///
/// The [`Relationship`] component is the "source of truth" and the [`RelationshipTarget`] component reflects that source of truth. When a [`Relationship`]
/// component is inserted on an [`Entity`], the corresponding [`RelationshipTarget`] component is immediately inserted on the target component if it does
/// not already exist, and the "source" entity is automatically added to the [`RelationshipTarget`] collection (this is done via "component hooks").
///
/// A common example of a [`Relationship`] is the parent / child relationship. Bevy ECS includes a canonical form of this via the [`ChildOf`](crate::hierarchy::ChildOf)
/// [`Relationship`] and the [`Children`](crate::hierarchy::Children) [`RelationshipTarget`].
///
/// [`Relationship`] and [`RelationshipTarget`] should always be derived via the [`Component`] trait to ensure the hooks are set up properly.
///
/// ```
/// # use bevy_ecs::component::Component;
/// # use bevy_ecs::entity::Entity;
/// #[derive(Component)]
/// #[relationship(relationship_target = Children)]
/// pub struct ChildOf(pub Entity);
///
/// #[derive(Component)]
/// #[relationship_target(relationship = ChildOf)]
/// pub struct Children(Vec<Entity>);
/// ```
///
/// When deriving [`RelationshipTarget`] you can specify the `#[relationship_target(linked_spawn)]` attribute to
/// automatically despawn entities stored in an entity's [`RelationshipTarget`] when that entity is despawned:
///
/// ```
/// # use bevy_ecs::component::Component;
/// # use bevy_ecs::entity::Entity;
/// #[derive(Component)]
/// #[relationship(relationship_target = Children)]
/// pub struct ChildOf(pub Entity);
///
/// #[derive(Component)]
/// #[relationship_target(relationship = ChildOf, linked_spawn)]
/// pub struct Children(Vec<Entity>);
/// ```
pub trait Relationship: Component + Sized {
    /// The [`Component`] added to the "target" entities of this [`Relationship`], which contains the list of all "source"
    /// entities that relate to the "target".
    type RelationshipTarget: RelationshipTarget<Relationship = Self>;

    /// Gets the [`Entity`] ID of the related entity.
    fn get(&self) -> Entity;

    /// Creates this [`Relationship`] from the given `entity`.
    fn from(entity: Entity) -> Self;

    /// The `on_insert` component hook that maintains the [`Relationship`] / [`RelationshipTarget`] connection.
    fn on_insert(mut world: DeferredWorld, HookContext { entity, caller, .. }: HookContext) {
        let target_entity = world.entity(entity).get::<Self>().unwrap().get();
        if target_entity == entity {
            warn!(
                "{}The {}({target_entity:?}) relationship on entity {entity:?} points to itself. The invalid {} relationship has been removed.",
                caller.map(|location|format!("{location}: ")).unwrap_or_default(),
                core::any::type_name::<Self>(),
                core::any::type_name::<Self>()
            );
            world.commands().entity(entity).remove::<Self>();
            return;
        }
        if let Ok(mut target_entity_mut) = world.get_entity_mut(target_entity) {
            if let Some(mut relationship_target) =
                target_entity_mut.get_mut::<Self::RelationshipTarget>()
            {
                relationship_target.collection_mut_risky().add(entity);
            } else {
                let mut target = <Self::RelationshipTarget as RelationshipTarget>::with_capacity(1);
                target.collection_mut_risky().add(entity);
                world.commands().entity(target_entity).insert(target);
            }
        } else {
            warn!(
                "{}The {}({target_entity:?}) relationship on entity {entity:?} relates to an entity that does not exist. The invalid {} relationship has been removed.",
                caller.map(|location|format!("{location}: ")).unwrap_or_default(),
                core::any::type_name::<Self>(),
                core::any::type_name::<Self>()
            );
            world.commands().entity(entity).remove::<Self>();
        }
    }

    /// The `on_replace` component hook that maintains the [`Relationship`] / [`RelationshipTarget`] connection.
    // note: think of this as "on_drop"
    fn on_replace(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
        let target_entity = world.entity(entity).get::<Self>().unwrap().get();
        if let Ok(mut target_entity_mut) = world.get_entity_mut(target_entity) {
            if let Some(mut relationship_target) =
                target_entity_mut.get_mut::<Self::RelationshipTarget>()
            {
                relationship_target.collection_mut_risky().remove(entity);
                if relationship_target.len() == 0 {
                    if let Some(mut entity) = world.commands().get_entity(target_entity) {
                        // this "remove" operation must check emptiness because in the event that an identical
                        // relationship is inserted on top, this despawn would result in the removal of that identical
                        // relationship ... not what we want!
                        entity.queue(|mut entity: EntityWorldMut| {
                            if entity
                                .get::<Self::RelationshipTarget>()
                                .is_some_and(RelationshipTarget::is_empty)
                            {
                                entity.remove::<Self::RelationshipTarget>();
                            }
                        });
                    }
                }
            }
        }
    }
}

/// The iterator type for the source entities in a [`RelationshipTarget`] collection,
/// as defined in the [`RelationshipSourceCollection`] trait.
pub type SourceIter<'w, R> =
    <<R as RelationshipTarget>::Collection as RelationshipSourceCollection>::SourceIter<'w>;

/// A [`Component`] containing the collection of entities that relate to this [`Entity`] via the associated `Relationship` type.
/// See the [`Relationship`] documentation for more information.
pub trait RelationshipTarget: Component<Mutability = Mutable> + Sized {
    /// If this is true, when despawning or cloning (when [recursion is enabled](crate::entity::EntityClonerBuilder::recursive)), the related entities targeting this entity will also be despawned or cloned.
    ///
    /// For example, this is set to `true` for Bevy's built-in parent-child relation, defined by [`ChildOf`](crate::prelude::ChildOf) and [`Children`](crate::prelude::Children).
    /// This means that when a parent is despawned, any children targeting that parent are also despawned (and the same applies to cloning).
    ///
    /// To get around this behavior, you can first break the relationship between entities, and *then* despawn or clone.
    /// This defaults to false when derived.
    const LINKED_SPAWN: bool;
    /// The [`Relationship`] that populates this [`RelationshipTarget`] collection.
    type Relationship: Relationship<RelationshipTarget = Self>;
    /// The collection type that stores the "source" entities for this [`RelationshipTarget`] component.
    ///
    /// Check the list of types which implement [`RelationshipSourceCollection`] for the data structures that can be used inside of your component.
    /// If you need a new collection type, you can implement the [`RelationshipSourceCollection`] trait
    /// for a type you own which wraps the collection you want to use (to avoid the orphan rule),
    /// or open an issue on the Bevy repository to request first-party support for your collection type.
    type Collection: RelationshipSourceCollection;

    /// Returns a reference to the stored [`RelationshipTarget::Collection`].
    fn collection(&self) -> &Self::Collection;
    /// Returns a mutable reference to the stored [`RelationshipTarget::Collection`].
    ///
    /// # Warning
    /// This should generally not be called by user code, as modifying the internal collection could invalidate the relationship.
    fn collection_mut_risky(&mut self) -> &mut Self::Collection;

    /// Creates a new [`RelationshipTarget`] from the given [`RelationshipTarget::Collection`].
    ///
    /// # Warning
    /// This should generally not be called by user code, as constructing the internal collection could invalidate the relationship.
    fn from_collection_risky(collection: Self::Collection) -> Self;

    /// The `on_replace` component hook that maintains the [`Relationship`] / [`RelationshipTarget`] connection.
    // note: think of this as "on_drop"
    fn on_replace(mut world: DeferredWorld, HookContext { entity, caller, .. }: HookContext) {
        // NOTE: this unsafe code is an optimization. We could make this safe, but it would require
        // copying the RelationshipTarget collection
        // SAFETY: This only reads the Self component and queues Remove commands
        unsafe {
            let world = world.as_unsafe_world_cell();
            let relationship_target = world.get_entity(entity).unwrap().get::<Self>().unwrap();
            let mut commands = world.get_raw_command_queue();
            for source_entity in relationship_target.iter() {
                if world.get_entity(source_entity).is_some() {
                    commands.push(
                        entity_command::remove::<Self::Relationship>()
                            .with_entity(source_entity)
                            .handle_error_with(error_handler::silent()),
                    );
                } else {
                    warn!(
                        "{}Tried to despawn non-existent entity {}",
                        caller
                            .map(|location| format!("{location}: "))
                            .unwrap_or_default(),
                        source_entity
                    );
                }
            }
        }
    }

    /// The `on_despawn` component hook that despawns entities stored in an entity's [`RelationshipTarget`] when
    /// that entity is despawned.
    // note: think of this as "on_drop"
    fn on_despawn(mut world: DeferredWorld, HookContext { entity, caller, .. }: HookContext) {
        // NOTE: this unsafe code is an optimization. We could make this safe, but it would require
        // copying the RelationshipTarget collection
        // SAFETY: This only reads the Self component and queues despawn commands
        unsafe {
            let world = world.as_unsafe_world_cell();
            let relationship_target = world.get_entity(entity).unwrap().get::<Self>().unwrap();
            let mut commands = world.get_raw_command_queue();
            for source_entity in relationship_target.iter() {
                if world.get_entity(source_entity).is_some() {
                    commands.push(
                        entity_command::despawn()
                            .with_entity(source_entity)
                            .handle_error_with(error_handler::silent()),
                    );
                } else {
                    warn!(
                        "{}Tried to despawn non-existent entity {}",
                        caller
                            .map(|location| format!("{location}: "))
                            .unwrap_or_default(),
                        source_entity
                    );
                }
            }
        }
    }

    /// Creates this [`RelationshipTarget`] with the given pre-allocated entity capacity.
    fn with_capacity(capacity: usize) -> Self {
        let collection =
            <Self::Collection as RelationshipSourceCollection>::with_capacity(capacity);
        Self::from_collection_risky(collection)
    }

    /// Iterates the entities stored in this collection.
    #[inline]
    fn iter(&self) -> SourceIter<'_, Self> {
        self.collection().iter()
    }

    /// Returns the number of entities in this collection.
    #[inline]
    fn len(&self) -> usize {
        self.collection().len()
    }

    /// Returns true if this entity collection is empty.
    #[inline]
    fn is_empty(&self) -> bool {
        self.collection().is_empty()
    }
}

/// The "clone behavior" for [`RelationshipTarget`]. This actually creates an empty
/// [`RelationshipTarget`] instance with space reserved for the number of targets in the
/// original instance. The [`RelationshipTarget`] will then be populated with the proper components
/// when the corresponding [`Relationship`] sources of truth are inserted. Cloning the actual entities
/// in the original [`RelationshipTarget`] would result in duplicates, so we don't do that!
///
/// This will also queue up clones of the relationship sources if the [`EntityCloner`](crate::entity::EntityCloner) is configured
/// to spawn recursively.
pub fn clone_relationship_target<T: RelationshipTarget>(
    _commands: &mut Commands,
    context: &mut ComponentCloneCtx,
) {
    if let Some(component) = context.read_source_component::<T>() {
        if context.is_recursive() && T::LINKED_SPAWN {
            for entity in component.iter() {
                context.queue_entity_clone(entity);
            }
        }
        context.write_target_component(T::with_capacity(component.len()));
    }
}

#[cfg(test)]
mod tests {
    use crate::world::World;
    use crate::{component::Component, entity::Entity};
    use alloc::vec::Vec;

    #[test]
    fn custom_relationship() {
        #[derive(Component)]
        #[relationship(relationship_target = LikedBy)]
        struct Likes(pub Entity);

        #[derive(Component)]
        #[relationship_target(relationship = Likes)]
        struct LikedBy(Vec<Entity>);

        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn(Likes(a)).id();
        let c = world.spawn(Likes(a)).id();
        assert_eq!(world.entity(a).get::<LikedBy>().unwrap().0, &[b, c]);
    }

    #[test]
    fn self_relationship_fails() {
        #[derive(Component)]
        #[relationship(relationship_target = RelTarget)]
        struct Rel(Entity);

        #[derive(Component)]
        #[relationship_target(relationship = Rel)]
        struct RelTarget(Vec<Entity>);

        let mut world = World::new();
        let a = world.spawn_empty().id();
        world.entity_mut(a).insert(Rel(a));
        assert!(!world.entity(a).contains::<Rel>());
        assert!(!world.entity(a).contains::<RelTarget>());
    }

    #[test]
    fn relationship_with_missing_target_fails() {
        #[derive(Component)]
        #[relationship(relationship_target = RelTarget)]
        struct Rel(Entity);

        #[derive(Component)]
        #[relationship_target(relationship = Rel)]
        struct RelTarget(Vec<Entity>);

        let mut world = World::new();
        let a = world.spawn_empty().id();
        world.despawn(a);
        let b = world.spawn(Rel(a)).id();
        assert!(!world.entity(b).contains::<Rel>());
        assert!(!world.entity(b).contains::<RelTarget>());
    }
}
