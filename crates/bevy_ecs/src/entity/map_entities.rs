use crate::{entity::Entity, world::World};
use bevy_utils::{Entry, HashMap};

/// Operation to map all contained [`Entity`] fields in a type to new values.
///
/// As entity IDs are valid only for the [`World`] they're sourced from, using [`Entity`]
/// as references in components copied from another world will be invalid. This trait
/// allows defining custom mappings for these references via [`EntityMap`].
///
/// Implementing this trait correctly is required for properly loading components
/// with entity references from scenes.
///
/// ## Example
///
/// ```rust
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::entity::{EntityMapper, MapEntities};
///
/// #[derive(Component)]
/// struct Spring {
///     a: Entity,
///     b: Entity,
/// }
///
/// impl MapEntities for Spring {
///     fn map_entities(&mut self, entity_mapper: &mut EntityMapper) {
///         self.a = entity_mapper.get_or_reserve(self.a);
///         self.b = entity_mapper.get_or_reserve(self.b);
///     }
/// }
/// ```
///
/// [`World`]: crate::world::World
pub trait MapEntities {
    /// Updates all [`Entity`] references stored inside using `entity_map`.
    ///
    /// Implementors should look up any and all [`Entity`] values stored within and
    /// update them to the mapped values via `entity_mapper`.
    fn map_entities(&mut self, entity_mapper: &mut EntityMapper);
}

/// A mapping from one set of entities to another.
///
/// The API generally follows [`HashMap`], but each [`Entity`] is returned by value, as they are [`Copy`].
///
/// This is typically used to coordinate data transfer between sets of entities, such as between a scene and the world
/// or over the network. This is required as [`Entity`] identifiers are opaque; you cannot and do not want to reuse
/// identifiers directly.
///
/// On its own, an `EntityMap` is not capable of allocating new entity identifiers, which is needed to map references
/// to entities that lie outside the source entity set. To do this, an `EntityMap` can be wrapped in an
/// [`EntityMapper`] which scopes it to a particular destination [`World`] and allows new identifiers to be allocated.
/// This functionality can be accessed through [`Self::world_scope()`].
#[derive(Default, Debug)]
pub struct EntityMap {
    map: HashMap<Entity, Entity>,
}

impl EntityMap {
    /// Inserts an entities pair into the map.
    ///
    /// If the map did not have `from` present, [`None`] is returned.
    ///
    /// If the map did have `from` present, the value is updated, and the old value is returned.
    pub fn insert(&mut self, from: Entity, to: Entity) -> Option<Entity> {
        self.map.insert(from, to)
    }

    /// Removes an `entity` from the map, returning the mapped value of it if the `entity` was previously in the map.
    pub fn remove(&mut self, entity: Entity) -> Option<Entity> {
        self.map.remove(&entity)
    }

    /// Gets the given entity's corresponding entry in the map for in-place manipulation.
    pub fn entry(&mut self, entity: Entity) -> Entry<'_, Entity, Entity> {
        self.map.entry(entity)
    }

    /// Returns the corresponding mapped entity.
    pub fn get(&self, entity: Entity) -> Option<Entity> {
        self.map.get(&entity).copied()
    }

    /// An iterator visiting all keys in arbitrary order.
    pub fn keys(&self) -> impl Iterator<Item = Entity> + '_ {
        self.map.keys().cloned()
    }

    /// An iterator visiting all values in arbitrary order.
    pub fn values(&self) -> impl Iterator<Item = Entity> + '_ {
        self.map.values().cloned()
    }

    /// Returns the number of elements in the map.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns true if the map contains no elements.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// An iterator visiting all (key, value) pairs in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = (Entity, Entity)> + '_ {
        self.map.iter().map(|(from, to)| (*from, *to))
    }

    /// Creates an [`EntityMapper`] from this [`EntityMap`] and scoped to the provided [`World`], then calls the
    /// provided function with it. This allows one to allocate new entity references in the provided `World` that are
    /// guaranteed to never point at a living entity now or in the future. This functionality is useful for safely
    /// mapping entity identifiers that point at entities outside the source world. The passed function, `f`, is called
    /// within the scope of the passed world. Its return value is then returned from `world_scope` as the generic type
    /// parameter `R`.
    pub fn world_scope<R>(
        &mut self,
        world: &mut World,
        f: impl FnOnce(&mut World, &mut EntityMapper) -> R,
    ) -> R {
        let mut mapper = EntityMapper::new(self, world);
        let result = f(world, &mut mapper);
        mapper.finish(world);
        result
    }
}

/// A wrapper for [`EntityMap`], augmenting it with the ability to allocate new [`Entity`] references in a destination
/// world. These newly allocated references are guaranteed to never point to any living entity in that world.
///
/// References are allocated by returning increasing generations starting from an internally initialized base
/// [`Entity`]. After it is finished being used by [`MapEntities`] implementations, this entity is despawned and the
/// requisite number of generations reserved.
pub struct EntityMapper<'m> {
    /// The wrapped [`EntityMap`].
    map: &'m mut EntityMap,
    /// A base [`Entity`] used to allocate new references.
    dead_start: Entity,
    /// The number of generations this mapper has allocated thus far.
    generations: u32,
}

impl<'m> EntityMapper<'m> {
    /// Returns the corresponding mapped entity or reserves a new dead entity ID if it is absent.
    pub fn get_or_reserve(&mut self, entity: Entity) -> Entity {
        if let Some(mapped) = self.map.get(entity) {
            return mapped;
        }

        // this new entity reference is specifically designed to never represent any living entity
        let new = Entity {
            generation: self.dead_start.generation + self.generations,
            index: self.dead_start.index,
        };
        self.generations += 1;

        self.map.insert(entity, new);

        new
    }

    /// Gets a reference to the underlying [`EntityMap`].
    pub fn get_map(&'m self) -> &'m EntityMap {
        self.map
    }

    /// Gets a mutable reference to the underlying [`EntityMap`]
    pub fn get_map_mut(&'m mut self) -> &'m mut EntityMap {
        self.map
    }

    /// Creates a new [`EntityMapper`], spawning a temporary base [`Entity`] in the provided [`World`]
    fn new(map: &'m mut EntityMap, world: &mut World) -> Self {
        Self {
            map,
            // SAFETY: Entities data is kept in a valid state via `EntityMap::world_scope`
            dead_start: unsafe { world.entities_mut().alloc() },
            generations: 0,
        }
    }

    /// Reserves the allocated references to dead entities within the world. This frees the temporary base
    /// [`Entity`] while reserving extra generations via [`crate::entity::Entities::reserve_generations`]. Because this
    /// renders the [`EntityMapper`] unable to safely allocate any more references, this method takes ownership of
    /// `self` in order to render it unusable.
    fn finish(self, world: &mut World) {
        // SAFETY: Entities data is kept in a valid state via `EntityMap::world_scope`
        let entities = unsafe { world.entities_mut() };
        assert!(entities.free(self.dead_start).is_some());
        assert!(entities.reserve_generations(self.dead_start.index, self.generations));
    }
}

#[cfg(test)]
mod tests {
    use super::{EntityMap, EntityMapper};
    use crate::{entity::Entity, world::World};

    #[test]
    fn entity_mapper() {
        const FIRST_IDX: u32 = 1;
        const SECOND_IDX: u32 = 2;

        let mut map = EntityMap::default();
        let mut world = World::new();
        let mut mapper = EntityMapper::new(&mut map, &mut world);

        let mapped_ent = Entity::new(FIRST_IDX, 0);
        let dead_ref = mapper.get_or_reserve(mapped_ent);

        assert_eq!(
            dead_ref,
            mapper.get_or_reserve(mapped_ent),
            "should persist the allocated mapping from the previous line"
        );
        assert_eq!(
            mapper.get_or_reserve(Entity::new(SECOND_IDX, 0)).index(),
            dead_ref.index(),
            "should re-use the same index for further dead refs"
        );

        mapper.finish(&mut world);
        // Next allocated entity should be a further generation on the same index
        let entity = world.spawn_empty().id();
        assert_eq!(entity.index(), dead_ref.index());
        assert!(entity.generation() > dead_ref.generation());
    }

    #[test]
    fn world_scope_reserves_generations() {
        let mut map = EntityMap::default();
        let mut world = World::new();

        let dead_ref = map.world_scope(&mut world, |_, mapper| {
            mapper.get_or_reserve(Entity::new(0, 0))
        });

        // Next allocated entity should be a further generation on the same index
        let entity = world.spawn_empty().id();
        assert_eq!(entity.index(), dead_ref.index());
        assert!(entity.generation() > dead_ref.generation());
    }
}
