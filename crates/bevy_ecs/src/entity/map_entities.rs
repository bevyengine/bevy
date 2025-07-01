pub use bevy_ecs_macros::MapEntities;
use indexmap::{IndexMap, IndexSet};

use crate::{
    entity::{hash_map::EntityHashMap, Entity},
    world::World,
};

use alloc::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    vec::Vec,
};
use bevy_platform::collections::{HashMap, HashSet};
use core::{
    hash::{BuildHasher, Hash},
    mem,
};
use smallvec::SmallVec;

use super::EntityIndexSet;

/// Operation to map all contained [`Entity`] fields in a type to new values.
///
/// As entity IDs are valid only for the [`World`] they're sourced from, using [`Entity`]
/// as references in components copied from another world will be invalid. This trait
/// allows defining custom mappings for these references via [`EntityMappers`](EntityMapper), which
/// inject the entity mapping strategy between your `MapEntities` type and the current world
/// (usually by using an [`EntityHashMap<Entity>`] between source entities and entities in the
/// current world).
///
/// Components use [`Component::map_entities`](crate::component::Component::map_entities) to map
/// entities in the context of scenes and entity cloning, which generally uses [`MapEntities`] internally
/// to map each field (see those docs for usage).
///
/// [`HashSet<Entity>`]: bevy_platform::collections::HashSet
///
/// ## Example
///
/// ```
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::entity::MapEntities;
///
/// #[derive(Component)]
/// struct Spring {
///     a: Entity,
///     b: Entity,
/// }
///
/// impl MapEntities for Spring {
///     fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
///         self.a = entity_mapper.get_mapped(self.a);
///         self.b = entity_mapper.get_mapped(self.b);
///     }
/// }
/// ```
pub trait MapEntities {
    /// Updates all [`Entity`] references stored inside using `entity_mapper`.
    ///
    /// Implementors should look up any and all [`Entity`] values stored within `self` and
    /// update them to the mapped values via `entity_mapper`.
    fn map_entities<E: EntityMapper>(&mut self, entity_mapper: &mut E);
}

impl MapEntities for Entity {
    fn map_entities<E: EntityMapper>(&mut self, entity_mapper: &mut E) {
        *self = entity_mapper.get_mapped(*self);
    }
}

impl<T: MapEntities> MapEntities for Option<T> {
    fn map_entities<E: EntityMapper>(&mut self, entity_mapper: &mut E) {
        if let Some(entities) = self {
            entities.map_entities(entity_mapper);
        }
    }
}

impl<K: MapEntities + Eq + Hash, V: MapEntities, S: BuildHasher + Default> MapEntities
    for HashMap<K, V, S>
{
    fn map_entities<E: EntityMapper>(&mut self, entity_mapper: &mut E) {
        *self = self
            .drain()
            .map(|(mut key_entities, mut value_entities)| {
                key_entities.map_entities(entity_mapper);
                value_entities.map_entities(entity_mapper);
                (key_entities, value_entities)
            })
            .collect();
    }
}

impl<T: MapEntities + Eq + Hash, S: BuildHasher + Default> MapEntities for HashSet<T, S> {
    fn map_entities<E: EntityMapper>(&mut self, entity_mapper: &mut E) {
        *self = self
            .drain()
            .map(|mut entities| {
                entities.map_entities(entity_mapper);
                entities
            })
            .collect();
    }
}

impl<K: MapEntities + Eq + Hash, V: MapEntities, S: BuildHasher + Default> MapEntities
    for IndexMap<K, V, S>
{
    fn map_entities<E: EntityMapper>(&mut self, entity_mapper: &mut E) {
        *self = self
            .drain(..)
            .map(|(mut key_entities, mut value_entities)| {
                key_entities.map_entities(entity_mapper);
                value_entities.map_entities(entity_mapper);
                (key_entities, value_entities)
            })
            .collect();
    }
}

impl<T: MapEntities + Eq + Hash, S: BuildHasher + Default> MapEntities for IndexSet<T, S> {
    fn map_entities<E: EntityMapper>(&mut self, entity_mapper: &mut E) {
        *self = self
            .drain(..)
            .map(|mut entities| {
                entities.map_entities(entity_mapper);
                entities
            })
            .collect();
    }
}

impl MapEntities for EntityIndexSet {
    fn map_entities<E: EntityMapper>(&mut self, entity_mapper: &mut E) {
        *self = self
            .drain(..)
            .map(|e| entity_mapper.get_mapped(e))
            .collect();
    }
}

impl<K: MapEntities + Ord, V: MapEntities> MapEntities for BTreeMap<K, V> {
    fn map_entities<E: EntityMapper>(&mut self, entity_mapper: &mut E) {
        *self = mem::take(self)
            .into_iter()
            .map(|(mut key_entities, mut value_entities)| {
                key_entities.map_entities(entity_mapper);
                value_entities.map_entities(entity_mapper);
                (key_entities, value_entities)
            })
            .collect();
    }
}

impl<T: MapEntities + Ord> MapEntities for BTreeSet<T> {
    fn map_entities<E: EntityMapper>(&mut self, entity_mapper: &mut E) {
        *self = mem::take(self)
            .into_iter()
            .map(|mut entities| {
                entities.map_entities(entity_mapper);
                entities
            })
            .collect();
    }
}

impl<T: MapEntities, const N: usize> MapEntities for [T; N] {
    fn map_entities<E: EntityMapper>(&mut self, entity_mapper: &mut E) {
        for entities in self.iter_mut() {
            entities.map_entities(entity_mapper);
        }
    }
}

impl<T: MapEntities> MapEntities for Vec<T> {
    fn map_entities<E: EntityMapper>(&mut self, entity_mapper: &mut E) {
        for entities in self.iter_mut() {
            entities.map_entities(entity_mapper);
        }
    }
}

impl<T: MapEntities> MapEntities for VecDeque<T> {
    fn map_entities<E: EntityMapper>(&mut self, entity_mapper: &mut E) {
        for entities in self.iter_mut() {
            entities.map_entities(entity_mapper);
        }
    }
}

impl<T: MapEntities, A: smallvec::Array<Item = T>> MapEntities for SmallVec<A> {
    fn map_entities<E: EntityMapper>(&mut self, entity_mapper: &mut E) {
        for entities in self.iter_mut() {
            entities.map_entities(entity_mapper);
        }
    }
}

/// An implementor of this trait knows how to map an [`Entity`] into another [`Entity`].
///
/// Usually this is done by using an [`EntityHashMap<Entity>`] to map source entities
/// (mapper inputs) to the current world's entities (mapper outputs).
///
/// More generally, this can be used to map [`Entity`] references between any two [`Worlds`](World).
///
/// This is used by [`MapEntities`] implementors.
///
/// ## Example
///
/// ```
/// # use bevy_ecs::entity::{Entity, EntityMapper};
/// # use bevy_ecs::entity::EntityHashMap;
/// #
/// pub struct SimpleEntityMapper {
///   map: EntityHashMap<Entity>,
/// }
///
/// // Example implementation of EntityMapper where we map an entity to another entity if it exists
/// // in the underlying `EntityHashMap`, otherwise we just return the original entity.
/// impl EntityMapper for SimpleEntityMapper {
///     fn get_mapped(&mut self, entity: Entity) -> Entity {
///         self.map.get(&entity).copied().unwrap_or(entity)
///     }
///
///     fn set_mapped(&mut self, source: Entity, target: Entity) {
///         self.map.insert(source, target);
///     }
/// }
/// ```
pub trait EntityMapper {
    /// Returns the "target" entity that maps to the given `source`.
    fn get_mapped(&mut self, source: Entity) -> Entity;

    /// Maps the `target` entity to the given `source`. For some implementations this might not actually determine the result
    /// of [`EntityMapper::get_mapped`].
    fn set_mapped(&mut self, source: Entity, target: Entity);
}

impl EntityMapper for () {
    #[inline]
    fn get_mapped(&mut self, source: Entity) -> Entity {
        source
    }

    #[inline]
    fn set_mapped(&mut self, _source: Entity, _target: Entity) {}
}

impl EntityMapper for (Entity, Entity) {
    #[inline]
    fn get_mapped(&mut self, source: Entity) -> Entity {
        if source == self.0 {
            self.1
        } else {
            source
        }
    }

    fn set_mapped(&mut self, _source: Entity, _target: Entity) {}
}

impl EntityMapper for &mut dyn EntityMapper {
    fn get_mapped(&mut self, source: Entity) -> Entity {
        (*self).get_mapped(source)
    }

    fn set_mapped(&mut self, source: Entity, target: Entity) {
        (*self).set_mapped(source, target);
    }
}

impl EntityMapper for SceneEntityMapper<'_> {
    /// Returns the corresponding mapped entity or reserves a new dead entity ID in the current world if it is absent.
    fn get_mapped(&mut self, source: Entity) -> Entity {
        if let Some(&mapped) = self.map.get(&source) {
            return mapped;
        }

        // this new entity reference is specifically designed to never represent any living entity
        let new = Entity::from_raw_and_generation(
            self.dead_start.row(),
            self.dead_start.generation.after_versions(self.generations),
        );
        self.generations = self.generations.wrapping_add(1);

        self.map.insert(source, new);

        new
    }

    fn set_mapped(&mut self, source: Entity, target: Entity) {
        self.map.insert(source, target);
    }
}

impl EntityMapper for EntityHashMap<Entity> {
    /// Returns the corresponding mapped entity or returns `entity` if there is no mapped entity
    fn get_mapped(&mut self, source: Entity) -> Entity {
        self.get(&source).cloned().unwrap_or(source)
    }

    fn set_mapped(&mut self, source: Entity, target: Entity) {
        self.insert(source, target);
    }
}

/// A wrapper for [`EntityHashMap<Entity>`], augmenting it with the ability to allocate new [`Entity`] references in a destination
/// world. These newly allocated references are guaranteed to never point to any living entity in that world.
///
/// References are allocated by returning increasing generations starting from an internally initialized base
/// [`Entity`]. After it is finished being used, this entity is despawned and the requisite number of generations reserved.
pub struct SceneEntityMapper<'m> {
    /// A mapping from one set of entities to another.
    ///
    /// This is typically used to coordinate data transfer between sets of entities, such as between a scene and the world
    /// or over the network. This is required as [`Entity`] identifiers are opaque; you cannot and do not want to reuse
    /// identifiers directly.
    ///
    /// On its own, a [`EntityHashMap<Entity>`] is not capable of allocating new entity identifiers, which is needed to map references
    /// to entities that lie outside the source entity set. This functionality can be accessed through [`SceneEntityMapper::world_scope()`].
    map: &'m mut EntityHashMap<Entity>,
    /// A base [`Entity`] used to allocate new references.
    dead_start: Entity,
    /// The number of generations this mapper has allocated thus far.
    generations: u32,
}

impl<'m> SceneEntityMapper<'m> {
    /// Gets a reference to the underlying [`EntityHashMap<Entity>`].
    pub fn get_map(&'m self) -> &'m EntityHashMap<Entity> {
        self.map
    }

    /// Gets a mutable reference to the underlying [`EntityHashMap<Entity>`].
    pub fn get_map_mut(&'m mut self) -> &'m mut EntityHashMap<Entity> {
        self.map
    }

    /// Creates a new [`SceneEntityMapper`], spawning a temporary base [`Entity`] in the provided [`World`]
    pub fn new(map: &'m mut EntityHashMap<Entity>, world: &mut World) -> Self {
        // We're going to be calling methods on `Entities` that require advance
        // flushing, such as `alloc` and `free`.
        world.flush_entities();
        Self {
            map,
            // SAFETY: Entities data is kept in a valid state via `EntityMapper::world_scope`
            dead_start: unsafe { world.entities_mut().alloc() },
            generations: 0,
        }
    }

    /// Reserves the allocated references to dead entities within the world. This frees the temporary base
    /// [`Entity`] while reserving extra generations. Because this makes the [`SceneEntityMapper`] unable to
    /// safely allocate any more references, this method takes ownership of `self` in order to render it unusable.
    pub fn finish(self, world: &mut World) {
        // SAFETY: Entities data is kept in a valid state via `EntityMap::world_scope`
        let entities = unsafe { world.entities_mut() };
        assert!(entities.free(self.dead_start).is_some());
        assert!(entities.reserve_generations(self.dead_start.index(), self.generations));
    }

    /// Creates an [`SceneEntityMapper`] from a provided [`World`] and [`EntityHashMap<Entity>`], then calls the
    /// provided function with it. This allows one to allocate new entity references in this [`World`] that are
    /// guaranteed to never point at a living entity now or in the future. This functionality is useful for safely
    /// mapping entity identifiers that point at entities outside the source world. The passed function, `f`, is called
    /// within the scope of this world. Its return value is then returned from `world_scope` as the generic type
    /// parameter `R`.
    pub fn world_scope<R>(
        entity_map: &'m mut EntityHashMap<Entity>,
        world: &mut World,
        f: impl FnOnce(&mut World, &mut Self) -> R,
    ) -> R {
        let mut mapper = Self::new(entity_map, world);
        let result = f(world, &mut mapper);
        mapper.finish(world);
        result
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        entity::{Entity, EntityHashMap, EntityMapper, SceneEntityMapper},
        world::World,
    };

    #[test]
    fn entity_mapper() {
        let mut map = EntityHashMap::default();
        let mut world = World::new();
        let mut mapper = SceneEntityMapper::new(&mut map, &mut world);

        let mapped_ent = Entity::from_raw_u32(1).unwrap();
        let dead_ref = mapper.get_mapped(mapped_ent);

        assert_eq!(
            dead_ref,
            mapper.get_mapped(mapped_ent),
            "should persist the allocated mapping from the previous line"
        );
        assert_eq!(
            mapper.get_mapped(Entity::from_raw_u32(2).unwrap()).index(),
            dead_ref.index(),
            "should re-use the same index for further dead refs"
        );

        mapper.finish(&mut world);
        // Next allocated entity should be a further generation on the same index
        let entity = world.spawn_empty().id();
        assert_eq!(entity.index(), dead_ref.index());
        assert!(entity
            .generation()
            .cmp_approx(&dead_ref.generation())
            .is_gt());
    }

    #[test]
    fn world_scope_reserves_generations() {
        let mut map = EntityHashMap::default();
        let mut world = World::new();

        let dead_ref = SceneEntityMapper::world_scope(&mut map, &mut world, |_, mapper| {
            mapper.get_mapped(Entity::from_raw_u32(0).unwrap())
        });

        // Next allocated entity should be a further generation on the same index
        let entity = world.spawn_empty().id();
        assert_eq!(entity.index(), dead_ref.index());
        assert!(entity
            .generation()
            .cmp_approx(&dead_ref.generation())
            .is_gt());
    }

    #[test]
    fn entity_mapper_no_panic() {
        let mut world = World::new();
        // "Dirty" the `Entities`, requiring a flush afterward.
        world.entities.reserve_entity();
        assert!(world.entities.needs_flush());

        // Create and exercise a SceneEntityMapper - should not panic because it flushes
        // `Entities` first.
        SceneEntityMapper::world_scope(&mut Default::default(), &mut world, |_, m| {
            m.get_mapped(Entity::PLACEHOLDER);
        });

        // The SceneEntityMapper should leave `Entities` in a flushed state.
        assert!(!world.entities.needs_flush());
    }
}
