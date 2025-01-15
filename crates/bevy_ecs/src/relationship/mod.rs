mod related_methods;
mod relationship_query;
mod relationship_source_collection;

pub use related_methods::*;
pub use relationship_query::*;
pub use relationship_source_collection::*;

pub use bevy_ecs_macros::{Relationship, RelationshipSources};

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

// The "deprecated" state is used to prevent users from mutating the internal RelationshipSource collection.
// These internals are allowed to modify the internal RelationshipSource collection.
#[allow(deprecated)]
pub trait Relationship: Component + Sized {
    type RelationshipSources: RelationshipSources<Relationship = Self>;
    /// Gets the [`Entity`] ID of the related entity.
    fn get(&self) -> Entity;
    fn set(&mut self, entity: Entity);
    fn from(entity: Entity) -> Self;
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

// The "deprecated" state is used to prevent users from mutating the internal RelationshipSource collection.
// These internals are allowed to modify the internal RelationshipSource collection.
#[allow(deprecated)]
pub trait RelationshipSources: Component<Mutability = Mutable> + Sized {
    type Relationship: Relationship<RelationshipSources = Self>;
    type Collection: RelationshipSourceCollection;

    fn collection(&self) -> &Self::Collection;
    #[deprecated = "Modifying the internal RelationshipSource collection should only be done by internals as it can invalidate relationships."]
    fn collection_mut(&mut self) -> &mut Self::Collection;
    #[deprecated = "Creating a relationship source manually should only be done by internals as it can invalidate relationships."]
    fn from_collection(collection: Self::Collection) -> Self;

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

    fn with_capacity(capacity: usize) -> Self {
        let collection =
            <Self::Collection as RelationshipSourceCollection>::with_capacity(capacity);
        Self::from_collection(collection)
    }

    #[inline]
    fn iter(&self) -> impl DoubleEndedIterator<Item = Entity> {
        self.collection().iter()
    }

    #[inline]
    fn len(&self) -> usize {
        self.collection().len()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.collection().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate as bevy_ecs;
    use crate::world::World;
    use crate::{
        entity::Entity,
        relationship::{Relationship, RelationshipSources},
    };
    use alloc::vec::Vec;

    #[test]
    fn custom_relationship() {
        #[derive(Relationship)]
        #[relationship_sources(LikedBy)]
        struct Likes(pub Entity);

        #[derive(RelationshipSources)]
        #[relationship(Likes)]
        struct LikedBy(Vec<Entity>);

        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn(Likes(a)).id();
        let c = world.spawn(Likes(a)).id();
        assert_eq!(world.entity(a).get::<LikedBy>().unwrap().0, &[b, c]);
    }
}
