//! This module provides [`Relationship`] functionality. See the [`Relationship`] trait for more info.

mod related_methods;
mod relationship_query;
mod relationship_source_collection;

pub use related_methods::*;
pub use relationship_query::*;
pub use relationship_source_collection::*;

use crate::{
    component::{Component, ComponentId, Mutable},
    entity::Entity,
    system::{
        command::HandleError,
        entity_command::{self, CommandWithEntity},
        error_handler,
    },
    world::DeferredWorld,
};
use log::warn;

/// A [`Component`] on a "source" [`Entity`] that references another target [`Entity`], creating a "relationship" between them. Every [`Relationship`]
/// has a corresponding [`RelationshipSources`] type (and vice-versa), which exists on the "target" entity of a relationship and contains the list of all
/// "source" entities that relate to the given "target"
///
/// The [`Relationship`] component is the "source of truth" and the [`RelationshipSources`] component reflects that source of truth. When a [`Relationship`]
/// component is inserted on an [`Entity`], the corresponding [`RelationshipSources`] component is immediately inserted on the target component if it does
/// not already exist, and the "source" entity is automatically added to the [`RelationshipSources`] collection (this is done via "component hooks").
///
/// A common example of a [`Relationship`] is the parent / child relationship. Bevy ECS includes a canonical form of this via the [`Parent`](crate::hierarchy::Parent)
/// [`Relationship`] and the [`Children`](crate::hierarchy::Children) [`RelationshipSources`].
///
/// [`Relationship`] and [`RelationshipSources`] should always be derived via the [`Component`] trait to ensure the hooks are set up properly.
///
/// ```
/// # use bevy_ecs::component::Component;
/// # use bevy_ecs::entity::Entity;
/// #[derive(Component)]
/// #[relationship(relationship_sources = Children)]
/// pub struct Parent(pub Entity);
///
/// #[derive(Component)]
/// #[relationship_sources(relationship = Parent)]
/// pub struct Children(Vec<Entity>);
/// ```
///
/// When deriving [`RelationshipSources`] you can specify the `#[relationship_sources(despawn_descendants)]` attribute to
/// automatically despawn entities stored in an entity's [`RelationshipSources`] when that entity is despawned:
///
/// ```
/// # use bevy_ecs::component::Component;
/// # use bevy_ecs::entity::Entity;
/// #[derive(Component)]
/// #[relationship(relationship_sources = Children)]
/// pub struct Parent(pub Entity);
///
/// #[derive(Component)]
/// #[relationship_sources(relationship = Parent, despawn_descendants)]
/// pub struct Children(Vec<Entity>);
/// ```
///
// NOTE: The "deprecated" state is used to prevent users from mutating the internal RelationshipSource collection.
// These internals are allowed to modify the internal RelationshipSource collection.
#[allow(deprecated)]
pub trait Relationship: Component + Sized {
    /// The [`Component`] added to the "target" entities of this [`Relationship`], which contains the list of all "source"
    /// entities that relate to the "target".
    type RelationshipSources: RelationshipSources<Relationship = Self>;

    /// Gets the [`Entity`] ID of the related entity.
    fn get(&self) -> Entity;

    /// Creates this [`Relationship`] from the given `entity`.
    fn from(entity: Entity) -> Self;

    /// The `on_insert` component hook that maintains the [`Relationship`] / [`RelationshipSources`] connection.
    fn on_insert(mut world: DeferredWorld, entity: Entity, _: ComponentId) {
        let parent = world.entity(entity).get::<Self>().unwrap().get();
        if parent == entity {
            warn!(
                "The {}({parent:?}) relationship on entity {entity:?} points to itself. The invalid {} relationship has been removed.",
                core::any::type_name::<Self>(),
                core::any::type_name::<Self>()
            );
            world.commands().entity(entity).remove::<Self>();
        }
        if let Ok(mut parent_entity) = world.get_entity_mut(parent) {
            if let Some(mut relationship_sources) =
                parent_entity.get_mut::<Self::RelationshipSources>()
            {
                relationship_sources.collection_mut().add(entity);
            } else {
                let mut sources =
                    <Self::RelationshipSources as RelationshipSources>::with_capacity(1);
                sources.collection_mut().add(entity);
                world.commands().entity(parent).insert(sources);
            }
        } else {
            warn!(
                "The {}({parent:?}) relationship on entity {entity:?} relates to an entity that does not exist. The invalid {} relationship has been removed.",
                core::any::type_name::<Self>(),
                core::any::type_name::<Self>()
            );
            world.commands().entity(entity).remove::<Self>();
        }
    }

    /// The `on_replace` component hook that maintains the [`Relationship`] / [`RelationshipSources`] connection.
    // note: think of this as "on_drop"
    fn on_replace(mut world: DeferredWorld, entity: Entity, _: ComponentId) {
        let parent = world.entity(entity).get::<Self>().unwrap().get();
        if let Ok(mut parent_entity) = world.get_entity_mut(parent) {
            if let Some(mut relationship_sources) =
                parent_entity.get_mut::<Self::RelationshipSources>()
            {
                relationship_sources.collection_mut().remove(entity);
                if relationship_sources.len() == 0 {
                    if let Some(mut entity) = world.commands().get_entity(parent) {
                        entity.remove::<Self::RelationshipSources>();
                    }
                }
            }
        }
    }
}

/// A [`Component`] containing the collection of entities that relate to this [`Entity`] via the associated `Relationship` type.
/// See the [`Relationship`] documentation for more information.
///
// The "deprecated" state is used to prevent users from mutating the internal RelationshipSource collection.
// These internals are allowed to modify the internal RelationshipSource collection.
#[allow(deprecated)]
pub trait RelationshipSources: Component<Mutability = Mutable> + Sized {
    /// The [`Relationship`] that populates this [`RelationshipSources`] collection.
    type Relationship: Relationship<RelationshipSources = Self>;
    /// The collection type that stores the "source" entities for this [`RelationshipSources`] component.
    type Collection: RelationshipSourceCollection;

    /// Returns a reference to the stored [`RelationshipSources::Collection`].
    fn collection(&self) -> &Self::Collection;
    /// Returns a mutable reference to the stored [`RelationshipSources::Collection`].
    ///
    /// # Warning
    /// This should generally not be called by user code, as modifying the internal collection could invalidate the relationship.
    /// This uses the "deprecated" state to warn users about this.
    #[deprecated = "Modifying the internal RelationshipSource collection should only be done by internals as it can invalidate relationships."]
    fn collection_mut(&mut self) -> &mut Self::Collection;

    /// Creates a new [`RelationshipSources`] from the given [`RelationshipSources::Collection`].
    ///
    /// # Warning
    /// This should generally not be called by user code, as constructing the internal collection could invalidate the relationship.
    /// This uses the "deprecated" state to warn users about this.
    #[deprecated = "Creating a relationship source manually should only be done by internals as it can invalidate relationships."]
    fn from_collection(collection: Self::Collection) -> Self;

    /// The `on_replace` component hook that maintains the [`Relationship`] / [`RelationshipSources`] connection.
    // note: think of this as "on_drop"
    fn on_replace(mut world: DeferredWorld, entity: Entity, _: ComponentId) {
        // NOTE: this unsafe code is an optimization. We could make this safe, but it would require
        // copying the RelationshipSources collection
        // SAFETY: This only reads the Self component and queues Remove commands
        unsafe {
            let world = world.as_unsafe_world_cell();
            let sources = world.get_entity(entity).unwrap().get::<Self>().unwrap();
            let mut commands = world.get_raw_command_queue();
            for source_entity in sources.iter() {
                if world.get_entity(source_entity).is_some() {
                    commands.push(
                        entity_command::remove::<Self::Relationship>()
                            .with_entity(source_entity)
                            .handle_error_with(error_handler::silent()),
                    );
                } else {
                    warn!("Tried to despawn non-existent entity {}", source_entity);
                }
            }
        }
    }

    /// The `on_despawn` component hook that despawns entities stored in an entity's [`RelationshipSources`] when
    /// that entity is despawned.
    // note: think of this as "on_drop"
    fn on_despawn(mut world: DeferredWorld, entity: Entity, _: ComponentId) {
        // NOTE: this unsafe code is an optimization. We could make this safe, but it would require
        // copying the RelationshipSources collection
        // SAFETY: This only reads the Self component and queues despawn commands
        unsafe {
            let world = world.as_unsafe_world_cell();
            let sources = world.get_entity(entity).unwrap().get::<Self>().unwrap();
            let mut commands = world.get_raw_command_queue();
            for source_entity in sources.iter() {
                if world.get_entity(source_entity).is_some() {
                    commands.push(
                        entity_command::despawn()
                            .with_entity(source_entity)
                            .handle_error_with(error_handler::silent()),
                    );
                } else {
                    warn!("Tried to despawn non-existent entity {}", source_entity);
                }
            }
        }
    }

    /// Creates this [`RelationshipSources`] with the given pre-allocated entity capacity.
    fn with_capacity(capacity: usize) -> Self {
        let collection =
            <Self::Collection as RelationshipSourceCollection>::with_capacity(capacity);
        Self::from_collection(collection)
    }

    /// Iterates the entities stored in this collection.
    #[inline]
    fn iter(&self) -> impl DoubleEndedIterator<Item = Entity> {
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

#[cfg(test)]
mod tests {
    use crate as bevy_ecs;
    use crate::world::World;
    use crate::{component::Component, entity::Entity};
    use alloc::vec::Vec;

    #[test]
    fn custom_relationship() {
        #[derive(Component)]
        #[relationship(relationship_sources = LikedBy)]
        struct Likes(pub Entity);

        #[derive(Component)]
        #[relationship_sources(relationship = Likes)]
        struct LikedBy(Vec<Entity>);

        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn(Likes(a)).id();
        let c = world.spawn(Likes(a)).id();
        assert_eq!(world.entity(a).get::<LikedBy>().unwrap().0, &[b, c]);
    }
}
