use crate::{
    entity::Entity,
    identifier::masks::{IdentifierMask, HIGH_MASK},
    world::World,
};

use super::EntityHashMap;

/// Operation to map all contained [`Entity`] fields in a type to new values.
///
/// As entity IDs are valid only for the [`World`] they're sourced from, using [`Entity`]
/// as references in components copied from another world will be invalid. This trait
/// allows defining custom mappings for these references via [`EntityMappers`](EntityMapper), which
/// inject the entity mapping strategy between your `MapEntities` type and the current world
/// (usually by using an [`EntityHashMap<Entity>`] between source entities and entities in the
/// current world).
///
/// Implementing this trait correctly is required for properly loading components
/// with entity references from scenes.
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
///         self.a = entity_mapper.map_entity(self.a);
///         self.b = entity_mapper.map_entity(self.b);
///     }
/// }
/// ```
pub trait MapEntities {
    /// Updates all [`Entity`] references stored inside using `entity_mapper`.
    ///
    /// Implementors should look up any and all [`Entity`] values stored within `self` and
    /// update them to the mapped values via `entity_mapper`.
    fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M);
}

/// An implementor of this trait knows how to map an [`Entity`] into another [`Entity`].
///
/// Usually this is done by using an [`EntityHashMap<Entity>`] to map source entities
/// (mapper inputs) to the current world's entities (mapper outputs).
///
/// More generally, this can be used to map [`Entity`] references between any two [`Worlds`](World).
///
/// Note that this trait is _not_ [object safe](https://doc.rust-lang.org/reference/items/traits.html#object-safety).
/// Please see [`DynEntityMapper`] for an object safe alternative.
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
///     fn map_entity(&mut self, entity: Entity) -> Entity {
///         self.map.get(&entity).copied().unwrap_or(entity)
///     }
///
///     fn mappings(&self) -> impl Iterator<Item = (Entity, Entity)> {
///         self.map.iter().map(|(&source, &target)| (source, target))
///     }
/// }
/// ```
pub trait EntityMapper {
    /// Map an entity to another entity
    fn map_entity(&mut self, entity: Entity) -> Entity;

    /// Iterate over all entity to entity mappings.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_ecs::entity::{Entity, EntityMapper};
    /// # fn example(mapper: impl EntityMapper) {
    /// for (source, target) in mapper.mappings() {
    ///     println!("Will map from {source} to {target}");
    /// }
    /// # }
    /// ```
    fn mappings(&self) -> impl Iterator<Item = (Entity, Entity)>;
}

/// An [object safe](https://doc.rust-lang.org/reference/items/traits.html#object-safety) version
/// of [`EntityMapper`]. This trait is automatically implemented for type that implements `EntityMapper`.
pub trait DynEntityMapper {
    /// Map an entity to another entity.
    ///
    /// This is an [object safe](https://doc.rust-lang.org/reference/items/traits.html#object-safety)
    /// alternative to [`EntityMapper::map_entity`].
    fn dyn_map_entity(&mut self, entity: Entity) -> Entity;

    /// Iterate over all entity to entity mappings.
    ///
    /// This is an [object safe](https://doc.rust-lang.org/reference/items/traits.html#object-safety)
    /// alternative to [`EntityMapper::mappings`].
    fn dyn_mappings(&self) -> Vec<(Entity, Entity)>;
}

impl<T: EntityMapper> DynEntityMapper for T {
    fn dyn_map_entity(&mut self, entity: Entity) -> Entity {
        <T as EntityMapper>::map_entity(self, entity)
    }

    fn dyn_mappings(&self) -> Vec<(Entity, Entity)> {
        <T as EntityMapper>::mappings(self).collect()
    }
}

impl EntityMapper for SceneEntityMapper<'_> {
    /// Returns the corresponding mapped entity or reserves a new dead entity ID in the current world if it is absent.
    fn map_entity(&mut self, entity: Entity) -> Entity {
        if let Some(&mapped) = self.map.get(&entity) {
            return mapped;
        }

        // this new entity reference is specifically designed to never represent any living entity
        let new = Entity::from_raw_and_generation(
            self.dead_start.index(),
            IdentifierMask::inc_masked_high_by(self.dead_start.generation, self.generations),
        );

        // Prevent generations counter from being a greater value than HIGH_MASK.
        self.generations = (self.generations + 1) & HIGH_MASK;

        self.map.insert(entity, new);

        new
    }

    fn mappings(&self) -> impl Iterator<Item = (Entity, Entity)> {
        self.map.iter().map(|(&source, &target)| (source, target))
    }
}

/// A wrapper for [`EntityHashMap<Entity>`], augmenting it with the ability to allocate new [`Entity`] references in a destination
/// world. These newly allocated references are guaranteed to never point to any living entity in that world.
///
/// References are allocated by returning increasing generations starting from an internally initialized base
/// [`Entity`]. After it is finished being used by [`MapEntities`] implementations, this entity is despawned and the
/// requisite number of generations reserved.
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
        Self {
            map,
            // SAFETY: Entities data is kept in a valid state via `EntityMapper::world_scope`
            dead_start: unsafe { world.entities_mut().alloc() },
            generations: 0,
        }
    }

    /// Reserves the allocated references to dead entities within the world. This frees the temporary base
    /// [`Entity`] while reserving extra generations via [`crate::entity::Entities::reserve_generations`]. Because this
    /// renders the [`SceneEntityMapper`] unable to safely allocate any more references, this method takes ownership of
    /// `self` in order to render it unusable.
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
    use crate::entity::DynEntityMapper;
    use crate::{
        entity::{Entity, EntityHashMap, EntityMapper, SceneEntityMapper},
        world::World,
    };
    use bevy_utils::assert_object_safe;

    #[test]
    fn entity_mapper() {
        const FIRST_IDX: u32 = 1;
        const SECOND_IDX: u32 = 2;

        let mut map = EntityHashMap::default();
        let mut world = World::new();
        let mut mapper = SceneEntityMapper::new(&mut map, &mut world);

        let mapped_ent = Entity::from_raw(FIRST_IDX);
        let dead_ref = mapper.map_entity(mapped_ent);

        assert_eq!(
            dead_ref,
            mapper.map_entity(mapped_ent),
            "should persist the allocated mapping from the previous line"
        );
        assert_eq!(
            mapper.map_entity(Entity::from_raw(SECOND_IDX)).index(),
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
        let mut map = EntityHashMap::default();
        let mut world = World::new();

        let dead_ref = SceneEntityMapper::world_scope(&mut map, &mut world, |_, mapper| {
            mapper.map_entity(Entity::from_raw(0))
        });

        // Next allocated entity should be a further generation on the same index
        let entity = world.spawn_empty().id();
        assert_eq!(entity.index(), dead_ref.index());
        assert!(entity.generation() > dead_ref.generation());
    }

    #[test]
    fn entity_mapper_iteration() {
        let mut old_world = World::new();
        let mut new_world = World::new();

        let mut map = EntityHashMap::default();
        let mut mapper = SceneEntityMapper::new(&mut map, &mut new_world);

        assert_eq!(mapper.mappings().collect::<Vec<_>>(), vec![]);

        let old_entity = old_world.spawn_empty().id();

        let new_entity = mapper.map_entity(old_entity);

        assert_eq!(
            mapper.mappings().collect::<Vec<_>>(),
            vec![(old_entity, new_entity)]
        );
    }

    #[test]
    fn dyn_entity_mapper_object_safe() {
        assert_object_safe::<dyn DynEntityMapper>();
    }
}
