mod entity_ref;
mod spawn_batch;
mod world_cell;

pub use crate::change_detection::Mut;
pub use entity_ref::*;
pub use spawn_batch::*;
pub use world_cell::*;

use crate::{
    archetype::{ArchetypeComponentId, ArchetypeComponentInfo, ArchetypeId, Archetypes},
    bundle::{Bundle, BundleInserter, BundleSpawner, Bundles},
    change_detection::Ticks,
    component::{
        Component, ComponentDescriptor, ComponentId, ComponentTicks, Components, ComponentsError,
        StorageType,
    },
    entity::{AllocAtWithoutReplacement, Entities, Entity},
    query::{FilterFetch, QueryState, WorldQuery},
    storage::{Column, SparseSet, Storages},
};
use std::{
    any::TypeId,
    fmt,
    sync::atomic::{AtomicU32, Ordering},
};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct WorldId(u64);

impl Default for WorldId {
    fn default() -> Self {
        WorldId(rand::random())
    }
}

/// Stores and exposes operations on [entities](Entity), [components](Component), resources,
/// and their associated metadata.
///
/// Each [Entity] has a set of components. Each component can have up to one instance of each
/// component type. Entity components can be created, updated, removed, and queried using a given
/// [World].
///
/// # Resources
///
/// Worlds can also store *resources*, which are unique instances of a given type that don't
/// belong to a specific Entity. There are also *non send resources*, which can only be
/// accessed on the main thread.
///
/// ## Usage of global resources
///
/// 1. Insert the resource into the `World`, using [`World::insert_resource`].
/// 2. Fetch the resource from a system, using [`Res`](crate::system::Res) or [`ResMut`](crate::system::ResMut).
///
/// ```
/// # let mut world = World::default();
/// # let mut schedule = Schedule::default();
/// # schedule.add_stage("update", SystemStage::parallel());
/// # use bevy_ecs::prelude::*;
/// #
/// struct MyResource { value: u32 }
///
/// world.insert_resource(MyResource { value: 42 });
///
/// fn read_resource_system(resource: Res<MyResource>) {
///     assert_eq!(resource.value, 42);
/// }
///
/// fn write_resource_system(mut resource: ResMut<MyResource>) {
///     assert_eq!(resource.value, 42);
///     resource.value = 0;
///     assert_eq!(resource.value, 0);
/// }
/// #
/// # schedule.add_system_to_stage("update", read_resource_system.label("first"));
/// # schedule.add_system_to_stage("update", write_resource_system.after("first"));
/// # schedule.run_once(&mut world);
/// ```
pub struct World {
    id: WorldId,
    pub(crate) entities: Entities,
    pub(crate) components: Components,
    pub(crate) archetypes: Archetypes,
    pub(crate) storages: Storages,
    pub(crate) bundles: Bundles,
    pub(crate) removed_components: SparseSet<ComponentId, Vec<Entity>>,
    /// Access cache used by [WorldCell].
    pub(crate) archetype_component_access: ArchetypeComponentAccess,
    main_thread_validator: MainThreadValidator,
    pub(crate) change_tick: AtomicU32,
    pub(crate) last_change_tick: u32,
}

impl Default for World {
    fn default() -> Self {
        Self {
            id: Default::default(),
            entities: Default::default(),
            components: Default::default(),
            archetypes: Default::default(),
            storages: Default::default(),
            bundles: Default::default(),
            removed_components: Default::default(),
            archetype_component_access: Default::default(),
            main_thread_validator: Default::default(),
            // Default value is `1`, and `last_change_tick`s default to `0`, such that changes
            // are detected on first system runs and for direct world queries.
            change_tick: AtomicU32::new(1),
            last_change_tick: 0,
        }
    }
}

impl World {
    /// Creates a new empty [World]
    #[inline]
    pub fn new() -> World {
        World::default()
    }

    /// Retrieves this world's unique ID
    #[inline]
    pub fn id(&self) -> WorldId {
        self.id
    }

    /// Retrieves this world's [Entities] collection
    #[inline]
    pub fn entities(&self) -> &Entities {
        &self.entities
    }

    /// Retrieves this world's [Entities] collection mutably
    #[inline]
    pub fn entities_mut(&mut self) -> &mut Entities {
        &mut self.entities
    }

    /// Retrieves this world's [Archetypes] collection
    #[inline]
    pub fn archetypes(&self) -> &Archetypes {
        &self.archetypes
    }

    /// Retrieves this world's [Components] collection
    #[inline]
    pub fn components(&self) -> &Components {
        &self.components
    }

    /// Retrieves a mutable reference to this world's [Components] collection
    #[inline]
    pub fn components_mut(&mut self) -> &mut Components {
        &mut self.components
    }

    /// Retrieves this world's [Storages] collection
    #[inline]
    pub fn storages(&self) -> &Storages {
        &self.storages
    }

    /// Retrieves this world's [Bundles] collection
    #[inline]
    pub fn bundles(&self) -> &Bundles {
        &self.bundles
    }

    /// Retrieves a [WorldCell], which safely enables multiple mutable World accesses at the same
    /// time, provided those accesses do not conflict with each other.
    #[inline]
    pub fn cell(&mut self) -> WorldCell<'_> {
        WorldCell::new(self)
    }

    /// Registers a new component using the given [ComponentDescriptor]. Components do not need to
    /// be manually registered. This just provides a way to override default configuration.
    /// Attempting to register a component with a type that has already been used by [World]
    /// will result in an error.
    ///
    /// The default component storage type can be overridden like this:
    ///
    /// ```
    /// use bevy_ecs::{component::{ComponentDescriptor, StorageType}, world::World};
    ///
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// let mut world = World::new();
    /// world.register_component(ComponentDescriptor::new::<Position>(StorageType::SparseSet)).unwrap();
    /// ```
    pub fn register_component(
        &mut self,
        descriptor: ComponentDescriptor,
    ) -> Result<ComponentId, ComponentsError> {
        let storage_type = descriptor.storage_type();
        let component_id = self.components.add(descriptor)?;
        // ensure sparse set is created for SparseSet components
        if storage_type == StorageType::SparseSet {
            // SAFE: just created
            let info = unsafe { self.components.get_info_unchecked(component_id) };
            self.storages.sparse_sets.get_or_insert(info);
        }

        Ok(component_id)
    }

    /// Retrieves an [EntityRef] that exposes read-only operations for the given `entity`.
    /// This will panic if the `entity` does not exist. Use [World::get_entity] if you want
    /// to check for entity existence instead of implicitly panic-ing.
    ///
    /// ```
    /// use bevy_ecs::world::World;
    ///
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn()
    ///     .insert(Position { x: 0.0, y: 0.0 })
    ///     .id();
    ///
    /// let position = world.entity(entity).get::<Position>().unwrap();
    /// assert_eq!(position.x, 0.0);
    /// ```
    #[inline]
    pub fn entity(&self, entity: Entity) -> EntityRef {
        self.get_entity(entity).expect("Entity does not exist")
    }

    /// Retrieves an [EntityMut] that exposes read and write operations for the given `entity`.
    /// This will panic if the `entity` does not exist. Use [World::get_entity_mut] if you want
    /// to check for entity existence instead of implicitly panic-ing.
    ///
    /// ```
    /// use bevy_ecs::world::World;
    ///
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn()
    ///     .insert(Position { x: 0.0, y: 0.0 })
    ///     .id();
    ///
    /// let mut position = world.entity_mut(entity).get_mut::<Position>().unwrap();
    /// position.x = 1.0;
    /// ```
    #[inline]
    pub fn entity_mut(&mut self, entity: Entity) -> EntityMut {
        self.get_entity_mut(entity).expect("Entity does not exist")
    }

    /// Returns an [EntityMut] for the given `entity` (if it exists) or spawns one if it doesn't exist.
    /// This will return [None] if the `entity` exists with a different generation.
    ///
    /// # Note
    /// Spawning a specific `entity` value is rarely the right choice. Most apps should favor [`World::spawn`].
    /// This method should generally only be used for sharing entities across apps, and only when they have a
    /// scheme worked out to share an ID space (which doesn't happen by default).
    #[inline]
    pub fn get_or_spawn(&mut self, entity: Entity) -> Option<EntityMut> {
        self.flush();
        match self.entities.alloc_at_without_replacement(entity) {
            AllocAtWithoutReplacement::Exists(location) => {
                // SAFE: `entity` exists and `location` is that entity's location
                Some(unsafe { EntityMut::new(self, entity, location) })
            }
            AllocAtWithoutReplacement::DidNotExist => {
                // SAFE: entity was just allocated
                Some(unsafe { self.spawn_at_internal(entity) })
            }
            AllocAtWithoutReplacement::ExistsWithWrongGeneration => None,
        }
    }

    /// Retrieves an [EntityRef] that exposes read-only operations for the given `entity`.
    /// Returns [None] if the `entity` does not exist. Use [World::entity] if you don't want
    /// to unwrap the [EntityRef] yourself.
    ///
    /// ```
    /// use bevy_ecs::world::World;
    ///
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn()
    ///     .insert(Position { x: 0.0, y: 0.0 })
    ///     .id();
    ///
    /// let entity_ref = world.get_entity(entity).unwrap();
    /// let position = entity_ref.get::<Position>().unwrap();
    /// assert_eq!(position.x, 0.0);
    /// ```
    #[inline]
    pub fn get_entity(&self, entity: Entity) -> Option<EntityRef> {
        let location = self.entities.get(entity)?;
        Some(EntityRef::new(self, entity, location))
    }

    /// Retrieves an [EntityMut] that exposes read and write operations for the given `entity`.
    /// Returns [None] if the `entity` does not exist. Use [World::entity_mut] if you don't want
    /// to unwrap the [EntityMut] yourself.
    ///
    /// ```
    /// use bevy_ecs::world::World;
    ///
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn()
    ///     .insert(Position { x: 0.0, y: 0.0 })
    ///     .id();
    ///
    /// let mut entity_mut = world.get_entity_mut(entity).unwrap();
    /// let mut position = entity_mut.get_mut::<Position>().unwrap();
    /// position.x = 1.0;
    /// ```
    #[inline]
    pub fn get_entity_mut(&mut self, entity: Entity) -> Option<EntityMut> {
        let location = self.entities.get(entity)?;
        // SAFE: `entity` exists and `location` is that entity's location
        Some(unsafe { EntityMut::new(self, entity, location) })
    }

    /// Spawns a new [Entity] and returns a corresponding [EntityMut], which can be used
    /// to add components to the entity or retrieve its id.
    ///
    /// ```
    /// use bevy_ecs::world::World;
    ///
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn()
    ///     .insert(Position { x: 0.0, y: 0.0 }) // add a single component
    ///     .insert_bundle((1, 2.0, "hello")) // add a bundle of components
    ///     .id();
    ///
    /// let position = world.entity(entity).get::<Position>().unwrap();
    /// assert_eq!(position.x, 0.0);
    /// ```
    pub fn spawn(&mut self) -> EntityMut {
        self.flush();
        let entity = self.entities.alloc();
        // SAFE: entity was just allocated
        unsafe { self.spawn_at_internal(entity) }
    }

    /// # Safety
    /// must be called on an entity that was just allocated
    unsafe fn spawn_at_internal(&mut self, entity: Entity) -> EntityMut {
        let archetype = self.archetypes.empty_mut();
        // PERF: consider avoiding allocating entities in the empty archetype unless needed
        let table_row = self.storages.tables[archetype.table_id()].allocate(entity);
        // SAFE: no components are allocated by archetype.allocate() because the archetype is
        // empty
        let location = archetype.allocate(entity, table_row);
        // SAFE: entity index was just allocated
        self.entities
            .meta
            .get_unchecked_mut(entity.id() as usize)
            .location = location;
        EntityMut::new(self, entity, location)
    }

    /// Spawns a batch of entities with the same component [Bundle] type. Takes a given [Bundle]
    /// iterator and returns a corresponding [Entity] iterator.
    /// This is more efficient than spawning entities and adding components to them individually,
    /// but it is limited to spawning entities with the same [Bundle] type, whereas spawning
    /// individually is more flexible.
    ///
    /// ```
    /// use bevy_ecs::{entity::Entity, world::World};
    ///
    /// let mut world = World::new();
    /// let entities = world.spawn_batch(vec![
    ///   ("a", 0.0), // the first entity
    ///   ("b", 1.0), // the second entity
    /// ]).collect::<Vec<Entity>>();
    ///
    /// assert_eq!(entities.len(), 2);
    /// ```
    pub fn spawn_batch<I>(&mut self, iter: I) -> SpawnBatchIter<'_, I::IntoIter>
    where
        I: IntoIterator,
        I::Item: Bundle,
    {
        SpawnBatchIter::new(self, iter.into_iter())
    }

    /// Retrieves a reference to the given `entity`'s [Component] of the given type.
    /// Returns [None] if the `entity` does not have a [Component] of the given type.
    /// ```
    /// use bevy_ecs::world::World;
    ///
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn()
    ///     .insert(Position { x: 0.0, y: 0.0 })
    ///     .id();
    /// let position = world.get::<Position>(entity).unwrap();
    /// assert_eq!(position.x, 0.0);
    /// ```
    #[inline]
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.get_entity(entity)?.get()
    }

    /// Retrieves a mutable reference to the given `entity`'s [Component] of the given type.
    /// Returns [None] if the `entity` does not have a [Component] of the given type.
    /// ```
    /// use bevy_ecs::world::World;
    ///
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn()
    ///     .insert(Position { x: 0.0, y: 0.0 })
    ///     .id();
    /// let mut position = world.get_mut::<Position>(entity).unwrap();
    /// position.x = 1.0;
    /// ```
    #[inline]
    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<Mut<T>> {
        self.get_entity_mut(entity)?.get_mut()
    }

    /// Despawns the given `entity`, if it exists. This will also remove all of the entity's
    /// [Component]s. Returns `true` if the `entity` is successfully despawned and `false` if
    /// the `entity` does not exist.
    /// ```
    /// use bevy_ecs::world::World;
    ///
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn()
    ///     .insert(Position { x: 0.0, y: 0.0 })
    ///     .id();
    /// assert!(world.despawn(entity));
    /// assert!(world.get_entity(entity).is_none());
    /// assert!(world.get::<Position>(entity).is_none());
    /// ```
    #[inline]
    pub fn despawn(&mut self, entity: Entity) -> bool {
        self.get_entity_mut(entity)
            .map(|e| {
                e.despawn();
                true
            })
            .unwrap_or(false)
    }

    /// Clears component tracker state
    pub fn clear_trackers(&mut self) {
        for entities in self.removed_components.values_mut() {
            entities.clear();
        }

        self.last_change_tick = self.increment_change_tick();
    }

    /// Returns [QueryState] for the given [WorldQuery], which is used to efficiently
    /// run queries on the [World] by storing and reusing the [QueryState].
    /// ```
    /// use bevy_ecs::{entity::Entity, world::World};
    ///
    /// #[derive(Debug, PartialEq)]
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// struct Velocity {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// let mut world = World::new();
    /// let entities = world.spawn_batch(vec![
    ///     (Position { x: 0.0, y: 0.0}, Velocity { x: 1.0, y: 0.0 }),    
    ///     (Position { x: 0.0, y: 0.0}, Velocity { x: 0.0, y: 1.0 }),    
    /// ]).collect::<Vec<Entity>>();
    ///
    /// let mut query = world.query::<(&mut Position, &Velocity)>();
    /// for (mut position, velocity) in query.iter_mut(&mut world) {
    ///    position.x += velocity.x;
    ///    position.y += velocity.y;
    /// }     
    ///
    /// assert_eq!(world.get::<Position>(entities[0]).unwrap(), &Position { x: 1.0, y: 0.0 });
    /// assert_eq!(world.get::<Position>(entities[1]).unwrap(), &Position { x: 0.0, y: 1.0 });
    /// ```
    ///
    /// To iterate over entities in a deterministic order,
    /// sort the results of the query using the desired component as a key.
    /// Note that this requires fetching the whole result set from the query
    /// and allocation of a [Vec] to store it.
    ///
    /// ```
    /// use bevy_ecs::{entity::Entity, world::World};
    /// let mut world = World::new();
    /// let a = world.spawn().insert_bundle((2, 4.0)).id();
    /// let b = world.spawn().insert_bundle((3, 5.0)).id();
    /// let c = world.spawn().insert_bundle((1, 6.0)).id();
    /// let mut entities = world.query::<(Entity, &i32, &f64)>()
    ///     .iter(&world)
    ///     .collect::<Vec<_>>();
    /// // Sort the query results by their `i32` component before comparing
    /// // to expected results. Query iteration order should not be relied on.
    /// entities.sort_by_key(|e| e.1);
    /// assert_eq!(entities, vec![(c, &1, &6.0), (a, &2, &4.0), (b, &3, &5.0)]);
    /// ```
    #[inline]
    pub fn query<Q: WorldQuery>(&mut self) -> QueryState<Q, ()> {
        QueryState::new(self)
    }

    /// Returns [QueryState] for the given filtered [WorldQuery], which is used to efficiently
    /// run queries on the [World] by storing and reusing the [QueryState].
    /// ```
    /// use bevy_ecs::{entity::Entity, world::World, query::With};
    ///
    /// struct A;
    /// struct B;
    ///
    /// let mut world = World::new();
    /// let e1 = world.spawn().insert(A).id();
    /// let e2 = world.spawn().insert_bundle((A, B)).id();
    ///
    /// let mut query = world.query_filtered::<Entity, With<B>>();
    /// let matching_entities = query.iter(&world).collect::<Vec<Entity>>();
    ///
    /// assert_eq!(matching_entities, vec![e2]);
    /// ```
    #[inline]
    pub fn query_filtered<Q: WorldQuery, F: WorldQuery>(&mut self) -> QueryState<Q, F>
    where
        F::Fetch: FilterFetch,
    {
        QueryState::new(self)
    }

    /// Returns an iterator of entities that had components of type `T` removed
    /// since the last call to [World::clear_trackers].
    pub fn removed<T: Component>(&self) -> std::iter::Cloned<std::slice::Iter<'_, Entity>> {
        if let Some(component_id) = self.components.get_id(TypeId::of::<T>()) {
            self.removed_with_id(component_id)
        } else {
            [].iter().cloned()
        }
    }

    /// Returns an iterator of entities that had components with the given `component_id` removed
    /// since the last call to [World::clear_trackers].
    pub fn removed_with_id(
        &self,
        component_id: ComponentId,
    ) -> std::iter::Cloned<std::slice::Iter<'_, Entity>> {
        if let Some(removed) = self.removed_components.get(component_id) {
            removed.iter().cloned()
        } else {
            [].iter().cloned()
        }
    }

    /// Inserts a new resource with the given `value`.
    /// Resources are "unique" data of a given type.
    #[inline]
    pub fn insert_resource<T: Component>(&mut self, value: T) {
        let component_id = self.components.get_or_insert_resource_id::<T>();
        // SAFE: component_id just initialized and corresponds to resource of type T
        unsafe { self.insert_resource_with_id(component_id, value) };
    }

    /// Inserts a new non-send resource with the given `value`.
    /// Resources are "unique" data of a given type.
    #[inline]
    pub fn insert_non_send<T: 'static>(&mut self, value: T) {
        self.validate_non_send_access::<T>();
        let component_id = self.components.get_or_insert_non_send_resource_id::<T>();
        // SAFE: component_id just initialized and corresponds to resource of type T
        unsafe { self.insert_resource_with_id(component_id, value) };
    }

    /// Removes the resource of a given type and returns it, if it exists. Otherwise returns [None].
    /// Resources are "unique" data of a given type.
    #[inline]
    pub fn remove_resource<T: Component>(&mut self) -> Option<T> {
        // SAFE: T is Send + Sync
        unsafe { self.remove_resource_unchecked() }
    }

    #[inline]
    pub fn remove_non_send<T: 'static>(&mut self) -> Option<T> {
        self.validate_non_send_access::<T>();
        // SAFE: we are on main thread
        unsafe { self.remove_resource_unchecked() }
    }

    #[inline]
    /// # Safety
    /// make sure you're on main thread if T isn't Send + Sync
    #[allow(unused_unsafe)]
    pub unsafe fn remove_resource_unchecked<T: 'static>(&mut self) -> Option<T> {
        let component_id = self.components.get_resource_id(TypeId::of::<T>())?;
        let resource_archetype = self.archetypes.resource_mut();
        let unique_components = resource_archetype.unique_components_mut();
        let column = unique_components.get_mut(component_id)?;
        if column.is_empty() {
            return None;
        }
        // SAFE: if a resource column exists, row 0 exists as well. caller takes ownership of the
        // ptr value / drop is called when T is dropped
        let (ptr, _) = unsafe { column.swap_remove_and_forget_unchecked(0) };
        // SAFE: column is of type T
        Some(unsafe { ptr.cast::<T>().read() })
    }

    /// Returns `true` if a resource of type `T` exists. Otherwise returns `false`.
    #[inline]
    pub fn contains_resource<T: Component>(&self) -> bool {
        let component_id =
            if let Some(component_id) = self.components.get_resource_id(TypeId::of::<T>()) {
                component_id
            } else {
                return false;
            };
        self.get_populated_resource_column(component_id).is_some()
    }

    /// Gets a reference to the resource of the given type, if it exists. Otherwise returns [None]
    /// Resources are "unique" data of a given type.
    #[inline]
    pub fn get_resource<T: Component>(&self) -> Option<&T> {
        let component_id = self.components.get_resource_id(TypeId::of::<T>())?;
        unsafe { self.get_resource_with_id(component_id) }
    }

    pub fn is_resource_added<T: Component>(&self) -> bool {
        let component_id =
            if let Some(component_id) = self.components.get_resource_id(TypeId::of::<T>()) {
                component_id
            } else {
                return false;
            };
        let column = if let Some(column) = self.get_populated_resource_column(component_id) {
            column
        } else {
            return false;
        };
        // SAFE: resources table always have row 0
        let ticks = unsafe { column.get_ticks_unchecked(0) };
        ticks.is_added(self.last_change_tick(), self.read_change_tick())
    }

    pub fn is_resource_changed<T: Component>(&self) -> bool {
        let component_id =
            if let Some(component_id) = self.components.get_resource_id(TypeId::of::<T>()) {
                component_id
            } else {
                return false;
            };
        let column = if let Some(column) = self.get_populated_resource_column(component_id) {
            column
        } else {
            return false;
        };
        // SAFE: resources table always have row 0
        let ticks = unsafe { column.get_ticks_unchecked(0) };
        ticks.is_changed(self.last_change_tick(), self.read_change_tick())
    }

    /// Gets a mutable reference to the resource of the given type, if it exists. Otherwise returns
    /// [None] Resources are "unique" data of a given type.
    #[inline]
    pub fn get_resource_mut<T: Component>(&mut self) -> Option<Mut<'_, T>> {
        // SAFE: unique world access
        unsafe { self.get_resource_unchecked_mut() }
    }

    // PERF: optimize this to avoid redundant lookups
    /// Gets a resource of type `T` if it exists, otherwise inserts the resource using the result of
    /// calling `func`.
    #[inline]
    pub fn get_resource_or_insert_with<T: Component>(
        &mut self,
        func: impl FnOnce() -> T,
    ) -> Mut<'_, T> {
        if !self.contains_resource::<T>() {
            self.insert_resource(func());
        }
        self.get_resource_mut().unwrap()
    }

    /// Gets a mutable reference to the resource of the given type, if it exists. Otherwise returns
    /// [None] Resources are "unique" data of a given type.
    ///
    /// # Safety
    /// This will allow aliased mutable access to the given resource type. The caller must ensure
    /// that only one mutable access exists at a time.
    #[inline]
    pub unsafe fn get_resource_unchecked_mut<T: Component>(&self) -> Option<Mut<'_, T>> {
        let component_id = self.components.get_resource_id(TypeId::of::<T>())?;
        self.get_resource_unchecked_mut_with_id(component_id)
    }

    /// Gets a reference to the non-send resource of the given type, if it exists. Otherwise returns
    /// [None] Resources are "unique" data of a given type.
    #[inline]
    pub fn get_non_send_resource<T: 'static>(&self) -> Option<&T> {
        let component_id = self.components.get_resource_id(TypeId::of::<T>())?;
        // SAFE: component id matches type T
        unsafe { self.get_non_send_with_id(component_id) }
    }

    /// Gets a mutable reference to the non-send resource of the given type, if it exists. Otherwise
    /// returns [None] Resources are "unique" data of a given type.
    #[inline]
    pub fn get_non_send_resource_mut<T: 'static>(&mut self) -> Option<Mut<'_, T>> {
        // SAFE: unique world access
        unsafe { self.get_non_send_resource_unchecked_mut() }
    }

    /// Gets a mutable reference to the non-send resource of the given type, if it exists. Otherwise
    /// returns [None] Resources are "unique" data of a given type.
    ///
    /// # Safety
    /// This will allow aliased mutable access to the given non-send resource type. The caller must
    /// ensure that only one mutable access exists at a time.
    #[inline]
    pub unsafe fn get_non_send_resource_unchecked_mut<T: 'static>(&self) -> Option<Mut<'_, T>> {
        let component_id = self.components.get_resource_id(TypeId::of::<T>())?;
        self.get_non_send_unchecked_mut_with_id(component_id)
    }

    /// For a given batch of ([Entity], [Bundle]) pairs, either spawns each [Entity] with the given
    /// bundle (if the entity does not exist), or inserts the [Bundle] (if the entity already exists).
    /// This is faster than doing equivalent operations one-by-one.
    /// Returns [Ok] if all entities were successfully inserted into or spawned. Otherwise it returns an [Err]
    /// with a list of entities that could not be spawned or inserted into. A "spawn or insert" operation can
    /// only fail if an [Entity] is passed in with an "invalid generation" that conflicts with an existing [Entity].
    ///
    /// # Note
    /// Spawning a specific `entity` value is rarely the right choice. Most apps should use [`World::spawn_batch`].
    /// This method should generally only be used for sharing entities across apps, and only when they have a scheme
    /// worked out to share an ID space (which doesn't happen by default).
    ///
    /// ```
    /// use bevy_ecs::{entity::Entity, world::World};
    ///
    /// let mut world = World::new();
    /// let e0 = world.spawn().id();
    /// let e1 = world.spawn().id();
    /// world.insert_or_spawn_batch(vec![
    ///   (e0, ("a", 0.0)), // the first entity
    ///   (e1, ("b", 1.0)), // the second entity
    /// ]);
    ///
    /// assert_eq!(world.get::<f64>(e0), Some(&0.0));
    /// ```
    pub fn insert_or_spawn_batch<I, B>(&mut self, iter: I) -> Result<(), Vec<Entity>>
    where
        I: IntoIterator,
        I::IntoIter: Iterator<Item = (Entity, B)>,
        B: Bundle,
    {
        self.flush();

        let iter = iter.into_iter();
        let change_tick = *self.change_tick.get_mut();

        let bundle_info = self.bundles.init_info::<B>(&mut self.components);
        enum SpawnOrInsert<'a, 'b> {
            Spawn(BundleSpawner<'a, 'b>),
            Insert(BundleInserter<'a, 'b>, ArchetypeId),
        }

        impl<'a, 'b> SpawnOrInsert<'a, 'b> {
            fn entities(&mut self) -> &mut Entities {
                match self {
                    SpawnOrInsert::Spawn(spawner) => spawner.entities,
                    SpawnOrInsert::Insert(inserter, _) => inserter.entities,
                }
            }
        }
        let mut spawn_or_insert = SpawnOrInsert::Spawn(bundle_info.get_bundle_spawner(
            &mut self.entities,
            &mut self.archetypes,
            &mut self.components,
            &mut self.storages,
            change_tick,
        ));

        let mut invalid_entities = Vec::new();
        for (entity, bundle) in iter {
            match spawn_or_insert
                .entities()
                .alloc_at_without_replacement(entity)
            {
                AllocAtWithoutReplacement::Exists(location) => {
                    match spawn_or_insert {
                        SpawnOrInsert::Insert(ref mut inserter, archetype)
                            if location.archetype_id == archetype =>
                        {
                            // SAFE: `entity` is valid, `location` matches entity, bundle matches inserter
                            unsafe { inserter.insert(entity, location.index, bundle) };
                        }
                        _ => {
                            let mut inserter = bundle_info.get_bundle_inserter(
                                &mut self.entities,
                                &mut self.archetypes,
                                &mut self.components,
                                &mut self.storages,
                                location.archetype_id,
                                change_tick,
                            );
                            // SAFE: `entity` is valid, `location` matches entity, bundle matches inserter
                            unsafe { inserter.insert(entity, location.index, bundle) };
                            spawn_or_insert =
                                SpawnOrInsert::Insert(inserter, location.archetype_id);
                        }
                    };
                }
                AllocAtWithoutReplacement::DidNotExist => {
                    match spawn_or_insert {
                        SpawnOrInsert::Spawn(ref mut spawner) => {
                            // SAFE: `entity` is allocated (but non existent), bundle matches inserter
                            unsafe { spawner.spawn_non_existent(entity, bundle) };
                        }
                        _ => {
                            let mut spawner = bundle_info.get_bundle_spawner(
                                &mut self.entities,
                                &mut self.archetypes,
                                &mut self.components,
                                &mut self.storages,
                                change_tick,
                            );
                            // SAFE: `entity` is valid, `location` matches entity, bundle matches inserter
                            unsafe { spawner.spawn_non_existent(entity, bundle) };
                            spawn_or_insert = SpawnOrInsert::Spawn(spawner);
                        }
                    };
                }
                AllocAtWithoutReplacement::ExistsWithWrongGeneration => {
                    invalid_entities.push(entity);
                }
            }
        }

        if invalid_entities.is_empty() {
            Ok(())
        } else {
            Err(invalid_entities)
        }
    }

    /// Temporarily removes the requested resource from this [World], then re-adds it before
    /// returning. This enables safe mutable access to a resource while still providing mutable
    /// world access
    /// ```
    /// use bevy_ecs::world::{World, Mut};
    /// struct A(u32);
    /// struct B(u32);
    /// let mut world = World::new();
    /// world.insert_resource(A(1));
    /// let entity = world.spawn().insert(B(1)).id();
    ///
    /// world.resource_scope(|world, mut a: Mut<A>| {
    ///     let b = world.get_mut::<B>(entity).unwrap();
    ///     a.0 += b.0;
    /// });
    /// assert_eq!(world.get_resource::<A>().unwrap().0, 2);
    /// ```
    pub fn resource_scope<T: Component, U>(
        &mut self,
        f: impl FnOnce(&mut World, Mut<T>) -> U,
    ) -> U {
        let component_id = self
            .components
            .get_resource_id(TypeId::of::<T>())
            .unwrap_or_else(|| panic!("resource does not exist: {}", std::any::type_name::<T>()));
        let (ptr, mut ticks) = {
            let resource_archetype = self.archetypes.resource_mut();
            let unique_components = resource_archetype.unique_components_mut();
            let column = unique_components.get_mut(component_id).unwrap_or_else(|| {
                panic!("resource does not exist: {}", std::any::type_name::<T>())
            });
            if column.is_empty() {
                panic!("resource does not exist: {}", std::any::type_name::<T>());
            }
            // SAFE: if a resource column exists, row 0 exists as well. caller takes ownership of
            // the ptr value / drop is called when T is dropped
            unsafe { column.swap_remove_and_forget_unchecked(0) }
        };
        // SAFE: pointer is of type T
        let value = Mut {
            value: unsafe { &mut *ptr.cast::<T>() },
            ticks: Ticks {
                component_ticks: &mut ticks,
                last_change_tick: self.last_change_tick(),
                change_tick: self.change_tick(),
            },
        };
        let result = f(self, value);
        let resource_archetype = self.archetypes.resource_mut();
        let unique_components = resource_archetype.unique_components_mut();
        let column = unique_components
            .get_mut(component_id)
            .unwrap_or_else(|| panic!("resource does not exist: {}", std::any::type_name::<T>()));
        unsafe {
            // SAFE: pointer is of type T
            column.push(ptr, ticks);
        }
        result
    }

    /// # Safety
    /// `component_id` must be assigned to a component of type T
    #[inline]
    pub(crate) unsafe fn get_resource_with_id<T: 'static>(
        &self,
        component_id: ComponentId,
    ) -> Option<&T> {
        let column = self.get_populated_resource_column(component_id)?;
        Some(&*column.get_data_ptr().as_ptr().cast::<T>())
    }

    /// # Safety
    /// `component_id` must be assigned to a component of type T.
    /// Caller must ensure this doesn't violate Rust mutability rules for the given resource.
    #[inline]
    pub(crate) unsafe fn get_resource_unchecked_mut_with_id<T>(
        &self,
        component_id: ComponentId,
    ) -> Option<Mut<'_, T>> {
        let column = self.get_populated_resource_column(component_id)?;
        Some(Mut {
            value: &mut *column.get_data_ptr().cast::<T>().as_ptr(),
            ticks: Ticks {
                component_ticks: &mut *column.get_ticks_mut_ptr_unchecked(0),
                last_change_tick: self.last_change_tick(),
                change_tick: self.read_change_tick(),
            },
        })
    }

    /// # Safety
    /// `component_id` must be assigned to a component of type T
    #[inline]
    pub(crate) unsafe fn get_non_send_with_id<T: 'static>(
        &self,
        component_id: ComponentId,
    ) -> Option<&T> {
        self.validate_non_send_access::<T>();
        self.get_resource_with_id(component_id)
    }

    /// # Safety
    /// `component_id` must be assigned to a component of type T.
    /// Caller must ensure this doesn't violate Rust mutability rules for the given resource.
    #[inline]
    pub(crate) unsafe fn get_non_send_unchecked_mut_with_id<T: 'static>(
        &self,
        component_id: ComponentId,
    ) -> Option<Mut<'_, T>> {
        self.validate_non_send_access::<T>();
        self.get_resource_unchecked_mut_with_id(component_id)
    }

    /// # Safety
    /// `component_id` must be valid and correspond to a resource component of type T
    #[inline]
    unsafe fn insert_resource_with_id<T>(&mut self, component_id: ComponentId, mut value: T) {
        let change_tick = self.change_tick();
        let column = self.initialize_resource_internal(component_id);
        if column.is_empty() {
            // SAFE: column is of type T and has been allocated above
            let data = (&mut value as *mut T).cast::<u8>();
            std::mem::forget(value);
            column.push(data, ComponentTicks::new(change_tick));
        } else {
            // SAFE: column is of type T and has already been allocated
            *column.get_data_unchecked(0).cast::<T>() = value;
            column.get_ticks_unchecked_mut(0).set_changed(change_tick);
        }
    }

    /// # Safety
    /// `component_id` must be valid and correspond to a resource component of type T
    #[inline]
    unsafe fn initialize_resource_internal(&mut self, component_id: ComponentId) -> &mut Column {
        // SAFE: resource archetype always exists
        let resource_archetype = self
            .archetypes
            .archetypes
            .get_unchecked_mut(ArchetypeId::RESOURCE.index());
        let resource_archetype_components = &mut resource_archetype.components;
        let archetype_component_count = &mut self.archetypes.archetype_component_count;
        let components = &self.components;
        resource_archetype
            .unique_components
            .get_or_insert_with(component_id, || {
                resource_archetype_components.insert(
                    component_id,
                    ArchetypeComponentInfo {
                        archetype_component_id: ArchetypeComponentId::new(
                            *archetype_component_count,
                        ),
                        storage_type: StorageType::Table,
                    },
                );
                *archetype_component_count += 1;
                let component_info = components.get_info_unchecked(component_id);
                Column::with_capacity(component_info, 1)
            })
    }

    pub(crate) fn initialize_resource<T: Component>(&mut self) -> ComponentId {
        let component_id = self.components.get_or_insert_resource_id::<T>();
        // SAFE: resource initialized above
        unsafe { self.initialize_resource_internal(component_id) };
        component_id
    }

    pub(crate) fn initialize_non_send_resource<T: 'static>(&mut self) -> ComponentId {
        let component_id = self.components.get_or_insert_non_send_resource_id::<T>();
        // SAFE: resource initialized above
        unsafe { self.initialize_resource_internal(component_id) };
        component_id
    }

    /// returns the resource column if the requested resource exists
    pub(crate) fn get_populated_resource_column(
        &self,
        component_id: ComponentId,
    ) -> Option<&Column> {
        let resource_archetype = self.archetypes.resource();
        let unique_components = resource_archetype.unique_components();
        unique_components.get(component_id).and_then(|column| {
            if column.is_empty() {
                None
            } else {
                Some(column)
            }
        })
    }

    pub(crate) fn validate_non_send_access<T: 'static>(&self) {
        if !self.main_thread_validator.is_main_thread() {
            panic!(
                "attempted to access NonSend resource {} off of the main thread",
                std::any::type_name::<T>()
            );
        }
    }

    /// Empties queued entities and adds them to the empty [Archetype].
    /// This should be called before doing operations that might operate on queued entities,
    /// such as inserting a [Component].
    pub(crate) fn flush(&mut self) {
        let empty_archetype = self.archetypes.empty_mut();
        unsafe {
            let table = &mut self.storages.tables[empty_archetype.table_id()];
            // PERF: consider pre-allocating space for flushed entities
            // SAFE: entity is set to a valid location
            self.entities.flush(|entity, location| {
                // SAFE: no components are allocated by archetype.allocate() because the archetype
                // is empty
                *location = empty_archetype.allocate(entity, table.allocate(entity));
            });
        }
    }

    #[inline]
    pub fn increment_change_tick(&self) -> u32 {
        self.change_tick.fetch_add(1, Ordering::AcqRel)
    }

    #[inline]
    pub fn read_change_tick(&self) -> u32 {
        self.change_tick.load(Ordering::Acquire)
    }

    #[inline]
    pub fn change_tick(&mut self) -> u32 {
        *self.change_tick.get_mut()
    }

    #[inline]
    pub fn last_change_tick(&self) -> u32 {
        self.last_change_tick
    }

    pub fn check_change_ticks(&mut self) {
        // Iterate over all component change ticks, clamping their age to max age
        // PERF: parallelize
        let change_tick = self.change_tick();
        self.storages.tables.check_change_ticks(change_tick);
        self.storages.sparse_sets.check_change_ticks(change_tick);
        let resource_archetype = self.archetypes.resource_mut();
        for column in resource_archetype.unique_components.values_mut() {
            column.check_change_ticks(change_tick);
        }
    }

    pub fn clear_entities(&mut self) {
        self.storages.tables.clear();
        self.storages.sparse_sets.clear();
        self.archetypes.clear_entities();
        self.entities.clear();
    }
}

impl fmt::Debug for World {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("World")
            .field("id", &self.id)
            .field("entity_count", &self.entities.len())
            .field("archetype_count", &self.archetypes.len())
            .field("component_count", &self.components.len())
            .field(
                "resource_count",
                &self.archetypes.resource().unique_components.len(),
            )
            .finish()
    }
}

unsafe impl Send for World {}
unsafe impl Sync for World {}

/// Creates `Self` using data from the given [World]
pub trait FromWorld {
    /// Creates `Self` using data from the given [World]
    fn from_world(world: &mut World) -> Self;
}

impl<T: Default> FromWorld for T {
    fn from_world(_world: &mut World) -> Self {
        T::default()
    }
}

struct MainThreadValidator {
    main_thread: std::thread::ThreadId,
}

impl MainThreadValidator {
    fn is_main_thread(&self) -> bool {
        self.main_thread == std::thread::current().id()
    }
}

impl Default for MainThreadValidator {
    fn default() -> Self {
        Self {
            main_thread: std::thread::current().id(),
        }
    }
}
