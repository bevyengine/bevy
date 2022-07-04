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
    change_detection::{MutUntyped, Ticks},
    component::{
        Component, ComponentDescriptor, ComponentId, ComponentInfo, ComponentTicks, Components,
        StorageType,
    },
    entity::{AllocAtWithoutReplacement, Entities, Entity},
    query::{QueryState, WorldQuery},
    storage::{Column, SparseSet, Storages},
    system::Resource,
};
use bevy_ptr::{OwningPtr, Ptr, UnsafeCellDeref};
use bevy_utils::tracing::debug;
use std::{
    any::TypeId,
    fmt,
    sync::atomic::{AtomicU32, Ordering},
};
mod identifier;

pub use identifier::WorldId;
/// Stores and exposes operations on [entities](Entity), [components](Component), resources,
/// and their associated metadata.
///
/// Each [Entity] has a set of components. Each component can have up to one instance of each
/// component type. Entity components can be created, updated, removed, and queried using a given
/// [World].
///
/// For complex access patterns involving [`SystemParam`](crate::system::SystemParam),
/// consider using [`SystemState`](crate::system::SystemState).
///
/// To mutate different parts of the world simultaneously,
/// use [`World::resource_scope`] or [`SystemState`](crate::system::SystemState).
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
            id: WorldId::new().expect("More `bevy` `World`s have been created than is supported"),
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
    /// # Panics
    ///
    /// If [`usize::MAX`] [`World`]s have been created.
    /// This guarantee allows System Parameters to safely uniquely identify a [`World`],
    /// since its [`WorldId`] is unique
    #[inline]
    pub fn new() -> World {
        World::default()
    }

    /// Retrieves this [`World`]'s unique ID
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
    ///
    /// # Safety
    /// Mutable reference must not be used to put the [`Entities`] data
    /// in an invalid state for this [`World`]
    #[inline]
    pub unsafe fn entities_mut(&mut self) -> &mut Entities {
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

    /// Retrieves a [`WorldCell`], which safely enables multiple mutable World accesses at the same
    /// time, provided those accesses do not conflict with each other.
    #[inline]
    pub fn cell(&mut self) -> WorldCell<'_> {
        WorldCell::new(self)
    }

    /// Initializes a new [`Component`] type and returns the [`ComponentId`] created for it.
    pub fn init_component<T: Component>(&mut self) -> ComponentId {
        self.components.init_component::<T>(&mut self.storages)
    }

    /// Initializes a new [`Component`] type and returns the [`ComponentId`] created for it.
    ///
    /// This method differs from [`World::init_component`] in that it uses a [`ComponentDescriptor`]
    /// to initialize the new component type instead of statically available type information. This
    /// enables the dynamic initialization of new component definitions at runtime for advanced use cases.
    ///
    /// While the option to initialize a component from a descriptor is useful in type-erased
    /// contexts, the standard `World::init_component` function should always be used instead
    /// when type information is available at compile time.
    pub fn init_component_with_descriptor(
        &mut self,
        descriptor: ComponentDescriptor,
    ) -> ComponentId {
        self.components
            .init_component_with_descriptor(&mut self.storages, descriptor)
    }

    /// Returns the [`ComponentId`] of the given [`Component`] type `T`.
    ///
    /// The returned `ComponentId` is specific to the `World` instance
    /// it was retrieved from and should not be used with another `World` instance.
    ///
    /// Returns [`None`] if the `Component` type has not yet been initialized within
    /// the `World` using [`World::init_component`].
    ///
    /// ```rust
    /// use bevy_ecs::prelude::*;
    ///
    /// let mut world = World::new();
    ///
    /// #[derive(Component)]
    /// struct ComponentA;
    ///
    /// let component_a_id = world.init_component::<ComponentA>();
    ///
    /// assert_eq!(component_a_id, world.component_id::<ComponentA>().unwrap())
    /// ```
    #[inline]
    pub fn component_id<T: Component>(&self) -> Option<ComponentId> {
        self.components.component_id::<T>()
    }

    /// Retrieves an [`EntityRef`] that exposes read-only operations for the given `entity`.
    /// This will panic if the `entity` does not exist. Use [`World::get_entity`] if you want
    /// to check for entity existence instead of implicitly panic-ing.
    ///
    /// ```
    /// use bevy_ecs::{component::Component, world::World};
    ///
    /// #[derive(Component)]
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
        // Lazily evaluate panic!() via unwrap_or_else() to avoid allocation unless failure
        self.get_entity(entity)
            .unwrap_or_else(|| panic!("Entity {:?} does not exist", entity))
    }

    /// Retrieves an [`EntityMut`] that exposes read and write operations for the given `entity`.
    /// This will panic if the `entity` does not exist. Use [`World::get_entity_mut`] if you want
    /// to check for entity existence instead of implicitly panic-ing.
    ///
    /// ```
    /// use bevy_ecs::{component::Component, world::World};
    ///
    /// #[derive(Component)]
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn()
    ///     .insert(Position { x: 0.0, y: 0.0 })
    ///     .id();
    /// let mut entity_mut = world.entity_mut(entity);
    /// let mut position = entity_mut.get_mut::<Position>().unwrap();
    /// position.x = 1.0;
    /// ```
    #[inline]
    pub fn entity_mut(&mut self, entity: Entity) -> EntityMut {
        // Lazily evaluate panic!() via unwrap_or_else() to avoid allocation unless failure
        self.get_entity_mut(entity)
            .unwrap_or_else(|| panic!("Entity {:?} does not exist", entity))
    }

    /// Returns the components of an [`Entity`](crate::entity::Entity) through [`ComponentInfo`](crate::component::ComponentInfo).
    #[inline]
    pub fn inspect_entity(&self, entity: Entity) -> Vec<&ComponentInfo> {
        let entity_location = self
            .entities()
            .get(entity)
            .unwrap_or_else(|| panic!("Entity {:?} does not exist", entity));

        let archetype = self
            .archetypes()
            .get(entity_location.archetype_id)
            .unwrap_or_else(|| {
                panic!(
                    "Archetype {:?} does not exist",
                    entity_location.archetype_id
                )
            });

        archetype
            .components()
            .filter_map(|id| self.components().get_info(id))
            .collect()
    }

    /// Returns an [`EntityMut`] for the given `entity` (if it exists) or spawns one if it doesn't exist.
    /// This will return [`None`] if the `entity` exists with a different generation.
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
                // SAFETY: `entity` exists and `location` is that entity's location
                Some(unsafe { EntityMut::new(self, entity, location) })
            }
            AllocAtWithoutReplacement::DidNotExist => {
                // SAFETY: entity was just allocated
                Some(unsafe { self.spawn_at_internal(entity) })
            }
            AllocAtWithoutReplacement::ExistsWithWrongGeneration => None,
        }
    }

    /// Retrieves an [`EntityRef`] that exposes read-only operations for the given `entity`.
    /// Returns [`None`] if the `entity` does not exist. Use [`World::entity`] if you don't want
    /// to unwrap the [`EntityRef`] yourself.
    ///
    /// ```
    /// use bevy_ecs::{component::Component, world::World};
    ///
    /// #[derive(Component)]
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

    /// Retrieves an [`EntityMut`] that exposes read and write operations for the given `entity`.
    /// Returns [`None`] if the `entity` does not exist. Use [`World::entity_mut`] if you don't want
    /// to unwrap the [`EntityMut`] yourself.
    ///
    /// ```
    /// use bevy_ecs::{component::Component, world::World};
    ///
    /// #[derive(Component)]
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
        // SAFETY: `entity` exists and `location` is that entity's location
        Some(unsafe { EntityMut::new(self, entity, location) })
    }

    /// Spawns a new [`Entity`] and returns a corresponding [`EntityMut`], which can be used
    /// to add components to the entity or retrieve its id.
    ///
    /// ```
    /// use bevy_ecs::{component::Component, world::World};
    ///
    /// #[derive(Component)]
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    /// #[derive(Component)]
    /// struct Label(&'static str);
    /// #[derive(Component)]
    /// struct Num(u32);
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn()
    ///     .insert(Position { x: 0.0, y: 0.0 }) // add a single component
    ///     .insert_bundle((Num(1), Label("hello"))) // add a bundle of components
    ///     .id();
    ///
    /// let position = world.entity(entity).get::<Position>().unwrap();
    /// assert_eq!(position.x, 0.0);
    /// ```
    pub fn spawn(&mut self) -> EntityMut {
        self.flush();
        let entity = self.entities.alloc();
        // SAFETY: entity was just allocated
        unsafe { self.spawn_at_internal(entity) }
    }

    /// # Safety
    /// must be called on an entity that was just allocated
    unsafe fn spawn_at_internal(&mut self, entity: Entity) -> EntityMut {
        let archetype = self.archetypes.empty_mut();
        // PERF: consider avoiding allocating entities in the empty archetype unless needed
        let table_row = self.storages.tables[archetype.table_id()].allocate(entity);
        // SAFETY: no components are allocated by archetype.allocate() because the archetype is
        // empty
        let location = archetype.allocate(entity, table_row);
        // SAFETY: entity index was just allocated
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
    /// use bevy_ecs::{component::Component, entity::Entity, world::World};
    ///
    /// #[derive(Component)]
    /// struct Str(&'static str);
    /// #[derive(Component)]
    /// struct Num(u32);
    ///
    /// let mut world = World::new();
    /// let entities = world.spawn_batch(vec![
    ///   (Str("a"), Num(0)), // the first entity
    ///   (Str("b"), Num(1)), // the second entity
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
    /// use bevy_ecs::{component::Component, world::World};
    ///
    /// #[derive(Component)]
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
    /// use bevy_ecs::{component::Component, world::World};
    ///
    /// #[derive(Component)]
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
        // SAFETY: lifetimes enforce correct usage of returned borrow
        unsafe { get_mut(self, entity, self.get_entity(entity)?.location()) }
    }

    /// Despawns the given `entity`, if it exists. This will also remove all of the entity's
    /// [Component]s. Returns `true` if the `entity` is successfully despawned and `false` if
    /// the `entity` does not exist.
    /// ```
    /// use bevy_ecs::{component::Component, world::World};
    ///
    /// #[derive(Component)]
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
        debug!("Despawning entity {:?}", entity);
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

    /// Returns [`QueryState`] for the given [`WorldQuery`], which is used to efficiently
    /// run queries on the [`World`] by storing and reusing the [`QueryState`].
    /// ```
    /// use bevy_ecs::{component::Component, entity::Entity, world::World};
    ///
    /// #[derive(Component, Debug, PartialEq)]
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// #[derive(Component)]
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
    /// use bevy_ecs::{component::Component, entity::Entity, world::World};
    ///
    /// #[derive(Component, PartialEq, Eq, PartialOrd, Ord, Debug)]
    /// struct Order(i32);
    /// #[derive(Component, PartialEq, Debug)]
    /// struct Label(&'static str);
    ///
    /// let mut world = World::new();
    /// let a = world.spawn().insert_bundle((Order(2), Label("second"))).id();
    /// let b = world.spawn().insert_bundle((Order(3), Label("third"))).id();
    /// let c = world.spawn().insert_bundle((Order(1), Label("first"))).id();
    /// let mut entities = world.query::<(Entity, &Order, &Label)>()
    ///     .iter(&world)
    ///     .collect::<Vec<_>>();
    /// // Sort the query results by their `Order` component before comparing
    /// // to expected results. Query iteration order should not be relied on.
    /// entities.sort_by_key(|e| e.1);
    /// assert_eq!(entities, vec![
    ///     (c, &Order(1), &Label("first")),
    ///     (a, &Order(2), &Label("second")),
    ///     (b, &Order(3), &Label("third")),
    /// ]);
    /// ```
    #[inline]
    pub fn query<Q: WorldQuery>(&mut self) -> QueryState<Q, ()> {
        self.query_filtered::<Q, ()>()
    }

    /// Returns [`QueryState`] for the given filtered [`WorldQuery`], which is used to efficiently
    /// run queries on the [`World`] by storing and reusing the [`QueryState`].
    /// ```
    /// use bevy_ecs::{component::Component, entity::Entity, world::World, query::With};
    ///
    /// #[derive(Component)]
    /// struct A;
    /// #[derive(Component)]
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
    pub fn query_filtered<Q: WorldQuery, F: WorldQuery>(&mut self) -> QueryState<Q, F> {
        QueryState::new(self)
    }

    /// Returns an iterator of entities that had components of type `T` removed
    /// since the last call to [`World::clear_trackers`].
    pub fn removed<T: Component>(&self) -> std::iter::Cloned<std::slice::Iter<'_, Entity>> {
        if let Some(component_id) = self.components.get_id(TypeId::of::<T>()) {
            self.removed_with_id(component_id)
        } else {
            [].iter().cloned()
        }
    }

    /// Returns an iterator of entities that had components with the given `component_id` removed
    /// since the last call to [`World::clear_trackers`].
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

    /// Inserts a new resource with standard starting values.
    ///
    /// If the resource already exists, nothing happens.
    ///
    /// The value given by the [`FromWorld::from_world`] method will be used.
    /// Note that any resource with the `Default` trait automatically implements `FromWorld`,
    /// and those default values will be here instead.
    #[inline]
    pub fn init_resource<R: Resource + FromWorld>(&mut self) {
        if !self.contains_resource::<R>() {
            let resource = R::from_world(self);
            self.insert_resource(resource);
        }
    }

    /// Inserts a new resource with the given `value`.
    ///
    /// Resources are "unique" data of a given type.
    /// If you insert a resource of a type that already exists,
    /// you will overwrite any existing data.
    #[inline]
    pub fn insert_resource<R: Resource>(&mut self, value: R) {
        let component_id = self.components.init_resource::<R>();
        // SAFETY: component_id just initialized and corresponds to resource of type T
        unsafe { self.insert_resource_with_id(component_id, value) };
    }

    /// Inserts a new non-send resource with standard starting values.
    ///
    /// If the resource already exists, nothing happens.
    ///
    /// The value given by the [`FromWorld::from_world`] method will be used.
    /// Note that any resource with the `Default` trait automatically implements `FromWorld`,
    /// and those default values will be here instead.
    #[inline]
    pub fn init_non_send_resource<R: 'static + FromWorld>(&mut self) {
        if !self.contains_resource::<R>() {
            let resource = R::from_world(self);
            self.insert_non_send_resource(resource);
        }
    }

    /// Inserts a new non-send resource with the given `value`.
    ///
    /// `NonSend` resources cannot be sent across threads,
    /// and do not need the `Send + Sync` bounds.
    /// Systems with `NonSend` resources are always scheduled on the main thread.
    #[inline]
    pub fn insert_non_send_resource<R: 'static>(&mut self, value: R) {
        self.validate_non_send_access::<R>();
        let component_id = self.components.init_non_send::<R>();
        // SAFETY: component_id just initialized and corresponds to resource of type R
        unsafe { self.insert_resource_with_id(component_id, value) };
    }

    /// Removes the resource of a given type and returns it, if it exists. Otherwise returns [None].
    #[inline]
    pub fn remove_resource<R: Resource>(&mut self) -> Option<R> {
        // SAFETY: R is Send + Sync
        unsafe { self.remove_resource_unchecked() }
    }

    #[inline]
    pub fn remove_non_send_resource<R: 'static>(&mut self) -> Option<R> {
        self.validate_non_send_access::<R>();
        // SAFETY: we are on main thread
        unsafe { self.remove_resource_unchecked() }
    }

    #[inline]
    /// # Safety
    /// Only remove `NonSend` resources from the main thread
    /// as they cannot be sent across theads
    #[allow(unused_unsafe)]
    pub unsafe fn remove_resource_unchecked<R: 'static>(&mut self) -> Option<R> {
        let component_id = self.components.get_resource_id(TypeId::of::<R>())?;
        let resource_archetype = self.archetypes.resource_mut();
        let unique_components = resource_archetype.unique_components_mut();
        let column = unique_components.get_mut(component_id)?;
        if column.is_empty() {
            return None;
        }
        // SAFETY: if a resource column exists, row 0 exists as well. caller takes ownership of the
        // ptr value / drop is called when R is dropped
        let (ptr, _) = unsafe { column.swap_remove_and_forget_unchecked(0) };
        // SAFETY: column is of type R
        Some(unsafe { ptr.read::<R>() })
    }

    /// Returns `true` if a resource of type `R` exists. Otherwise returns `false`.
    #[inline]
    pub fn contains_resource<R: 'static>(&self) -> bool {
        let component_id =
            if let Some(component_id) = self.components.get_resource_id(TypeId::of::<R>()) {
                component_id
            } else {
                return false;
            };
        self.get_populated_resource_column(component_id).is_some()
    }

    pub fn is_resource_added<R: Resource>(&self) -> bool {
        let component_id =
            if let Some(component_id) = self.components.get_resource_id(TypeId::of::<R>()) {
                component_id
            } else {
                return false;
            };
        let column = if let Some(column) = self.get_populated_resource_column(component_id) {
            column
        } else {
            return false;
        };
        // SAFETY: resources table always have row 0
        let ticks = unsafe { column.get_ticks_unchecked(0).deref() };
        ticks.is_added(self.last_change_tick(), self.read_change_tick())
    }

    pub fn is_resource_changed<R: Resource>(&self) -> bool {
        let component_id =
            if let Some(component_id) = self.components.get_resource_id(TypeId::of::<R>()) {
                component_id
            } else {
                return false;
            };
        let column = if let Some(column) = self.get_populated_resource_column(component_id) {
            column
        } else {
            return false;
        };
        // SAFETY: resources table always have row 0
        let ticks = unsafe { column.get_ticks_unchecked(0).deref() };
        ticks.is_changed(self.last_change_tick(), self.read_change_tick())
    }

    /// Gets a reference to the resource of the given type
    ///
    /// # Panics
    ///
    /// Panics if the resource does not exist.
    /// Use [`get_resource`](World::get_resource) instead if you want to handle this case.
    ///
    /// If you want to instead insert a value if the resource does not exist,
    /// use [`get_resource_or_insert_with`](World::get_resource_or_insert_with).
    #[inline]
    #[track_caller]
    pub fn resource<R: Resource>(&self) -> &R {
        match self.get_resource() {
            Some(x) => x,
            None => panic!(
                "Requested resource {} does not exist in the `World`. 
                Did you forget to add it using `app.insert_resource` / `app.init_resource`? 
                Resources are also implicitly added via `app.add_event`,
                and can be added by plugins.",
                std::any::type_name::<R>()
            ),
        }
    }

    /// Gets a mutable reference to the resource of the given type
    ///
    /// # Panics
    ///
    /// Panics if the resource does not exist.
    /// Use [`get_resource_mut`](World::get_resource_mut) instead if you want to handle this case.
    ///
    /// If you want to instead insert a value if the resource does not exist,
    /// use [`get_resource_or_insert_with`](World::get_resource_or_insert_with).
    #[inline]
    #[track_caller]
    pub fn resource_mut<R: Resource>(&mut self) -> Mut<'_, R> {
        match self.get_resource_mut() {
            Some(x) => x,
            None => panic!(
                "Requested resource {} does not exist in the `World`. 
                Did you forget to add it using `app.insert_resource` / `app.init_resource`? 
                Resources are also implicitly added via `app.add_event`,
                and can be added by plugins.",
                std::any::type_name::<R>()
            ),
        }
    }

    /// Gets a reference to the resource of the given type if it exists
    #[inline]
    pub fn get_resource<R: Resource>(&self) -> Option<&R> {
        let component_id = self.components.get_resource_id(TypeId::of::<R>())?;
        // SAFETY: unique world access
        unsafe { self.get_resource_with_id(component_id) }
    }

    /// Gets a mutable reference to the resource of the given type if it exists
    #[inline]
    pub fn get_resource_mut<R: Resource>(&mut self) -> Option<Mut<'_, R>> {
        // SAFETY: unique world access
        unsafe { self.get_resource_unchecked_mut() }
    }

    // PERF: optimize this to avoid redundant lookups
    /// Gets a mutable reference to the resource of type `T` if it exists,
    /// otherwise inserts the resource using the result of calling `func`.
    #[inline]
    pub fn get_resource_or_insert_with<R: Resource>(
        &mut self,
        func: impl FnOnce() -> R,
    ) -> Mut<'_, R> {
        if !self.contains_resource::<R>() {
            self.insert_resource(func());
        }
        self.resource_mut()
    }

    /// Gets a mutable reference to the resource of the given type, if it exists
    /// Otherwise returns [None]
    ///
    /// # Safety
    /// This will allow aliased mutable access to the given resource type. The caller must ensure
    /// that there is either only one mutable access or multiple immutable accesses at a time.
    #[inline]
    pub unsafe fn get_resource_unchecked_mut<R: Resource>(&self) -> Option<Mut<'_, R>> {
        let component_id = self.components.get_resource_id(TypeId::of::<R>())?;
        self.get_resource_unchecked_mut_with_id(component_id)
    }

    /// Gets an immutable reference to the non-send resource of the given type, if it exists.
    ///
    /// # Panics
    ///
    /// Panics if the resource does not exist.
    /// Use [`get_non_send_resource`](World::get_non_send_resource) instead if you want to handle this case.
    #[inline]
    #[track_caller]
    pub fn non_send_resource<R: 'static>(&self) -> &R {
        match self.get_non_send_resource() {
            Some(x) => x,
            None => panic!(
                "Requested non-send resource {} does not exist in the `World`. 
                Did you forget to add it using `app.insert_non_send_resource` / `app.init_non_send_resource`? 
                Non-send resources can also be be added by plugins.",
                std::any::type_name::<R>()
            ),
        }
    }

    /// Gets a mutable reference to the non-send resource of the given type, if it exists.
    ///
    /// # Panics
    ///
    /// Panics if the resource does not exist.
    /// Use [`get_non_send_resource_mut`](World::get_non_send_resource_mut) instead if you want to handle this case.
    #[inline]
    #[track_caller]
    pub fn non_send_resource_mut<R: 'static>(&mut self) -> Mut<'_, R> {
        match self.get_non_send_resource_mut() {
            Some(x) => x,
            None => panic!(
                "Requested non-send resource {} does not exist in the `World`. 
                Did you forget to add it using `app.insert_non_send_resource` / `app.init_non_send_resource`? 
                Non-send resources can also be be added by plugins.",
                std::any::type_name::<R>()
            ),
        }
    }

    /// Gets a reference to the non-send resource of the given type, if it exists.
    /// Otherwise returns [None]
    #[inline]
    pub fn get_non_send_resource<R: 'static>(&self) -> Option<&R> {
        let component_id = self.components.get_resource_id(TypeId::of::<R>())?;
        // SAFETY: component id matches type T
        unsafe { self.get_non_send_with_id(component_id) }
    }

    /// Gets a mutable reference to the non-send resource of the given type, if it exists.
    /// Otherwise returns [None]
    #[inline]
    pub fn get_non_send_resource_mut<R: 'static>(&mut self) -> Option<Mut<'_, R>> {
        // SAFETY: unique world access
        unsafe { self.get_non_send_resource_unchecked_mut() }
    }

    /// Gets a mutable reference to the non-send resource of the given type, if it exists.
    /// Otherwise returns [None]
    ///
    /// # Safety
    /// This will allow aliased mutable access to the given non-send resource type. The caller must
    /// ensure that there is either only one mutable access or multiple immutable accesses at a time.
    #[inline]
    pub unsafe fn get_non_send_resource_unchecked_mut<R: 'static>(&self) -> Option<Mut<'_, R>> {
        let component_id = self.components.get_resource_id(TypeId::of::<R>())?;
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
    /// use bevy_ecs::{entity::Entity, world::World, component::Component};
    /// #[derive(Component)]
    /// struct A(&'static str);
    /// #[derive(Component, PartialEq, Debug)]
    /// struct B(f32);
    ///
    /// let mut world = World::new();
    /// let e0 = world.spawn().id();
    /// let e1 = world.spawn().id();
    /// world.insert_or_spawn_batch(vec![
    ///   (e0, (A("a"), B(0.0))), // the first entity
    ///   (e1, (A("b"), B(1.0))), // the second entity
    /// ]);
    ///
    /// assert_eq!(world.get::<B>(e0), Some(&B(0.0)));
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

        let bundle_info = self
            .bundles
            .init_info::<B>(&mut self.components, &mut self.storages);
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
                            // SAFETY: `entity` is valid, `location` matches entity, bundle matches inserter
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
                            // SAFETY: `entity` is valid, `location` matches entity, bundle matches inserter
                            unsafe { inserter.insert(entity, location.index, bundle) };
                            spawn_or_insert =
                                SpawnOrInsert::Insert(inserter, location.archetype_id);
                        }
                    };
                }
                AllocAtWithoutReplacement::DidNotExist => {
                    if let SpawnOrInsert::Spawn(ref mut spawner) = spawn_or_insert {
                        // SAFETY: `entity` is allocated (but non existent), bundle matches inserter
                        unsafe { spawner.spawn_non_existent(entity, bundle) };
                    } else {
                        let mut spawner = bundle_info.get_bundle_spawner(
                            &mut self.entities,
                            &mut self.archetypes,
                            &mut self.components,
                            &mut self.storages,
                            change_tick,
                        );
                        // SAFETY: `entity` is valid, `location` matches entity, bundle matches inserter
                        unsafe { spawner.spawn_non_existent(entity, bundle) };
                        spawn_or_insert = SpawnOrInsert::Spawn(spawner);
                    }
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

    /// Temporarily removes the requested resource from this [`World`], then re-adds it before returning.
    ///
    /// This enables safe simultaneous mutable access to both a resource and the rest of the [`World`].
    /// For more complex access patterns, consider using [`SystemState`](crate::system::SystemState).
    ///
    /// # Example
    /// ```
    /// use bevy_ecs::{component::Component, world::{World, Mut}};
    /// #[derive(Component)]
    /// struct A(u32);
    /// #[derive(Component)]
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
    pub fn resource_scope<R: Resource, U>(&mut self, f: impl FnOnce(&mut World, Mut<R>) -> U) -> U {
        let last_change_tick = self.last_change_tick();
        let change_tick = self.change_tick();

        let component_id = self
            .components
            .get_resource_id(TypeId::of::<R>())
            .unwrap_or_else(|| panic!("resource does not exist: {}", std::any::type_name::<R>()));
        let (ptr, mut ticks) = {
            let resource_archetype = self.archetypes.resource_mut();
            let unique_components = resource_archetype.unique_components_mut();
            let column = unique_components.get_mut(component_id).unwrap_or_else(|| {
                panic!("resource does not exist: {}", std::any::type_name::<R>())
            });
            assert!(
                !column.is_empty(),
                "resource does not exist: {}",
                std::any::type_name::<R>()
            );
            // SAFETY: if a resource column exists, row 0 exists as well. caller takes ownership of
            // the ptr value / drop is called when R is dropped
            unsafe { column.swap_remove_and_forget_unchecked(0) }
        };
        // SAFETY: pointer is of type R
        // Read the value onto the stack to avoid potential mut aliasing.
        let mut value = unsafe { ptr.read::<R>() };
        let value_mut = Mut {
            value: &mut value,
            ticks: Ticks {
                component_ticks: &mut ticks,
                last_change_tick,
                change_tick,
            },
        };
        let result = f(self, value_mut);
        assert!(!self.contains_resource::<R>());

        let resource_archetype = self.archetypes.resource_mut();
        let unique_components = resource_archetype.unique_components_mut();
        let column = unique_components
            .get_mut(component_id)
            .unwrap_or_else(|| panic!("resource does not exist: {}", std::any::type_name::<R>()));

        OwningPtr::make(value, |ptr| {
            // SAFETY: pointer is of type R
            unsafe {
                column.push(ptr, ticks);
            }
        });
        result
    }

    /// # Safety
    /// `component_id` must be assigned to a component of type `R`
    #[inline]
    pub(crate) unsafe fn get_resource_with_id<R: 'static>(
        &self,
        component_id: ComponentId,
    ) -> Option<&R> {
        let column = self.get_populated_resource_column(component_id)?;
        Some(column.get_data_ptr().deref::<R>())
    }

    /// # Safety
    /// `component_id` must be assigned to a component of type `R`
    /// Caller must ensure this doesn't violate Rust mutability rules for the given resource.
    #[inline]
    pub(crate) unsafe fn get_resource_unchecked_mut_with_id<R>(
        &self,
        component_id: ComponentId,
    ) -> Option<Mut<'_, R>> {
        let column = self.get_populated_resource_column(component_id)?;
        Some(Mut {
            value: column.get_data_ptr().assert_unique().deref_mut(),
            ticks: Ticks {
                component_ticks: column.get_ticks_unchecked(0).deref_mut(),
                last_change_tick: self.last_change_tick(),
                change_tick: self.read_change_tick(),
            },
        })
    }

    /// # Safety
    /// `component_id` must be assigned to a component of type `R`
    #[inline]
    pub(crate) unsafe fn get_non_send_with_id<R: 'static>(
        &self,
        component_id: ComponentId,
    ) -> Option<&R> {
        self.validate_non_send_access::<R>();
        self.get_resource_with_id(component_id)
    }

    /// # Safety
    /// `component_id` must be assigned to a component of type `R`.
    /// Caller must ensure this doesn't violate Rust mutability rules for the given resource.
    #[inline]
    pub(crate) unsafe fn get_non_send_unchecked_mut_with_id<R: 'static>(
        &self,
        component_id: ComponentId,
    ) -> Option<Mut<'_, R>> {
        self.validate_non_send_access::<R>();
        self.get_resource_unchecked_mut_with_id(component_id)
    }

    /// # Safety
    /// `component_id` must be valid and correspond to a resource component of type `R`
    #[inline]
    unsafe fn insert_resource_with_id<R>(&mut self, component_id: ComponentId, value: R) {
        let change_tick = self.change_tick();
        let column = self.initialize_resource_internal(component_id);
        if column.is_empty() {
            // SAFETY: column is of type R and has been allocated above
            OwningPtr::make(value, |ptr| {
                column.push(ptr, ComponentTicks::new(change_tick));
            });
        } else {
            // SAFETY: column is of type R and has already been allocated
            *column.get_data_unchecked_mut(0).deref_mut::<R>() = value;
            column.get_ticks_unchecked_mut(0).set_changed(change_tick);
        }
    }

    /// Inserts a new resource with the given `value`. Will replace the value if it already existed.
    ///
    /// **You should prefer to use the typed API [`World::insert_resource`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    ///
    /// # Safety
    /// The value referenced by `value` must be valid for the given [`ComponentId`] of this world
    pub unsafe fn insert_resource_by_id(
        &mut self,
        component_id: ComponentId,
        value: OwningPtr<'_>,
    ) {
        let change_tick = self.change_tick();

        self.components().get_info(component_id).unwrap_or_else(|| {
            panic!(
                "insert_resource_by_id called with component id which doesn't exist in this world"
            )
        });
        // SAFETY: component_id is valid, checked by the lines above
        let column = self.initialize_resource_internal(component_id);
        if column.is_empty() {
            // SAFETY: column is of type R and has been allocated above
            column.push(value, ComponentTicks::new(change_tick));
        } else {
            let ptr = column.get_data_unchecked_mut(0);
            std::ptr::copy_nonoverlapping::<u8>(
                value.as_ptr(),
                ptr.as_ptr(),
                column.item_layout().size(),
            );
            column.get_ticks_unchecked_mut(0).set_changed(change_tick);
        }
    }

    /// # Safety
    /// `component_id` must be valid for this world
    #[inline]
    unsafe fn initialize_resource_internal(&mut self, component_id: ComponentId) -> &mut Column {
        // SAFETY: resource archetype always exists
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

    pub(crate) fn initialize_resource<R: Resource>(&mut self) -> ComponentId {
        let component_id = self.components.init_resource::<R>();
        // SAFETY: resource initialized above
        unsafe { self.initialize_resource_internal(component_id) };
        component_id
    }

    pub(crate) fn initialize_non_send_resource<R: 'static>(&mut self) -> ComponentId {
        let component_id = self.components.init_non_send::<R>();
        // SAFETY: resource initialized above
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
        assert!(
            self.main_thread_validator.is_main_thread(),
            "attempted to access NonSend resource {} off of the main thread",
            std::any::type_name::<T>(),
        );
    }

    pub(crate) fn validate_non_send_access_untyped(&self, name: &str) {
        assert!(
            self.main_thread_validator.is_main_thread(),
            "attempted to access NonSend resource {} off of the main thread",
            name
        );
    }

    /// Empties queued entities and adds them to the empty [Archetype](crate::archetype::Archetype).
    /// This should be called before doing operations that might operate on queued entities,
    /// such as inserting a [Component].
    pub(crate) fn flush(&mut self) {
        let empty_archetype = self.archetypes.empty_mut();
        let table = &mut self.storages.tables[empty_archetype.table_id()];
        // PERF: consider pre-allocating space for flushed entities
        // SAFETY: entity is set to a valid location
        unsafe {
            self.entities.flush(|entity, location| {
                // SAFETY: no components are allocated by archetype.allocate() because the archetype
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

impl World {
    /// Gets a resource to the resource with the id [`ComponentId`] if it exists.
    /// The returned pointer must not be used to modify the resource, and must not be
    /// dereferenced after the immutable borrow of the [`World`] ends.
    ///
    /// **You should prefer to use the typed API [`World::get_resource`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    #[inline]
    pub fn get_resource_by_id(&self, component_id: ComponentId) -> Option<Ptr<'_>> {
        let info = self.components.get_info(component_id)?;
        if !info.is_send_and_sync() {
            self.validate_non_send_access_untyped(info.name());
        }

        let column = self.get_populated_resource_column(component_id)?;
        Some(column.get_data_ptr())
    }

    /// Gets a resource to the resource with the id [`ComponentId`] if it exists.
    /// The returned pointer may be used to modify the resource, as long as the mutable borrow
    /// of the [`World`] is still valid.
    ///
    /// **You should prefer to use the typed API [`World::get_resource_mut`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    #[inline]
    pub fn get_resource_mut_by_id(&mut self, component_id: ComponentId) -> Option<MutUntyped<'_>> {
        let info = self.components.get_info(component_id)?;
        if !info.is_send_and_sync() {
            self.validate_non_send_access_untyped(info.name());
        }

        let column = self.get_populated_resource_column(component_id)?;

        // SAFETY: get_data_ptr requires that the mutability rules are not violated, and the caller promises
        // to only modify the resource while the mutable borrow of the world is valid
        let ticks = Ticks {
            // SAFETY:
            // - index is in-bounds because the column is initialized and non-empty
            // - no other reference to the ticks of the same row can exist at the same time
            component_ticks: unsafe { &mut *column.get_ticks_unchecked(0).get() },
            last_change_tick: self.last_change_tick(),
            change_tick: self.read_change_tick(),
        };

        Some(MutUntyped {
            // SAFETY: world access is unique, so no other reference can exist at the same time
            value: unsafe { column.get_data_ptr().assert_unique() },
            ticks,
        })
    }

    /// Removes the resource of a given type, if it exists. Otherwise returns [None].
    ///
    /// **You should prefer to use the typed API [`World::remove_resource`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    pub fn remove_resource_by_id(&mut self, component_id: ComponentId) -> Option<()> {
        let info = self.components.get_info(component_id)?;
        if !info.is_send_and_sync() {
            self.validate_non_send_access_untyped(info.name());
        }

        let resource_archetype = self.archetypes.resource_mut();
        let unique_components = resource_archetype.unique_components_mut();
        let column = unique_components.get_mut(component_id)?;
        if column.is_empty() {
            return None;
        }
        // SAFETY: if a resource column exists, row 0 exists as well
        unsafe { column.swap_remove_unchecked(0) };

        Some(())
    }

    /// Retrieves a mutable untyped reference to the given `entity`'s [Component] of the given [`ComponentId`].
    /// Returns [None] if the `entity` does not have a [Component] of the given type.
    ///
    /// **You should prefer to use the typed API [`World::get_mut`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    #[inline]
    pub fn get_by_id(&self, entity: Entity, component_id: ComponentId) -> Option<Ptr<'_>> {
        self.components().get_info(component_id)?;
        // SAFETY: entity_location is valid, component_id is valid as checked by the line above
        unsafe {
            get_component(
                self,
                component_id,
                entity,
                self.get_entity(entity)?.location(),
            )
        }
    }

    /// Retrieves a mutable untyped reference to the given `entity`'s [Component] of the given [`ComponentId`].
    /// Returns [None] if the `entity` does not have a [Component] of the given type.
    ///
    /// **You should prefer to use the typed API [`World::get_mut`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    #[inline]
    pub fn get_mut_by_id(
        &mut self,
        entity: Entity,
        component_id: ComponentId,
    ) -> Option<MutUntyped<'_>> {
        self.components().get_info(component_id)?;
        // SAFETY: entity_location is valid, component_id is valid as checked by the line above
        unsafe {
            get_mut_by_id(
                self,
                entity,
                self.get_entity(entity)?.location(),
                component_id,
            )
        }
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

// TODO: remove allow on lint - https://github.com/bevyengine/bevy/issues/3666
#[allow(clippy::non_send_fields_in_send_ty)]
// SAFETY: all methods on the world ensure that non-send resources are only accessible on the main thread
unsafe impl Send for World {}
// SAFETY: all methods on the world ensure that non-send resources are only accessible on the main thread
unsafe impl Sync for World {}

/// Creates an instance of the type this trait is implemented for
/// using data from the supplied [World].
///
/// This can be helpful for complex initialization or context-aware defaults.
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

#[cfg(test)]
mod tests {
    use super::World;
    use crate::{
        change_detection::DetectChanges,
        component::{ComponentDescriptor, ComponentId, ComponentInfo, StorageType},
        ptr::OwningPtr,
    };
    use bevy_ecs_macros::Component;
    use bevy_utils::HashSet;
    use std::{
        any::TypeId,
        panic,
        sync::{
            atomic::{AtomicBool, AtomicU32, Ordering},
            Arc, Mutex,
        },
    };

    // For bevy_ecs_macros
    use crate as bevy_ecs;

    type ID = u8;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum DropLogItem {
        Create(ID),
        Drop(ID),
    }

    #[derive(Component)]
    struct MayPanicInDrop {
        drop_log: Arc<Mutex<Vec<DropLogItem>>>,
        expected_panic_flag: Arc<AtomicBool>,
        should_panic: bool,
        id: u8,
    }

    impl MayPanicInDrop {
        fn new(
            drop_log: &Arc<Mutex<Vec<DropLogItem>>>,
            expected_panic_flag: &Arc<AtomicBool>,
            should_panic: bool,
            id: u8,
        ) -> Self {
            println!("creating component with id {}", id);
            drop_log.lock().unwrap().push(DropLogItem::Create(id));

            Self {
                drop_log: Arc::clone(drop_log),
                expected_panic_flag: Arc::clone(expected_panic_flag),
                should_panic,
                id,
            }
        }
    }

    impl Drop for MayPanicInDrop {
        fn drop(&mut self) {
            println!("dropping component with id {}", self.id);

            {
                let mut drop_log = self.drop_log.lock().unwrap();
                drop_log.push(DropLogItem::Drop(self.id));
                // Don't keep the mutex while panicking, or we'll poison it.
                drop(drop_log);
            }

            if self.should_panic {
                self.expected_panic_flag.store(true, Ordering::SeqCst);
                panic!("testing what happens on panic inside drop");
            }
        }
    }

    struct DropTestHelper {
        drop_log: Arc<Mutex<Vec<DropLogItem>>>,
        /// Set to `true` right before we intentionally panic, so that if we get
        /// a panic, we know if it was intended or not.
        expected_panic_flag: Arc<AtomicBool>,
    }

    impl DropTestHelper {
        pub fn new() -> Self {
            Self {
                drop_log: Arc::new(Mutex::new(Vec::<DropLogItem>::new())),
                expected_panic_flag: Arc::new(AtomicBool::new(false)),
            }
        }

        pub fn make_component(&self, should_panic: bool, id: ID) -> MayPanicInDrop {
            MayPanicInDrop::new(&self.drop_log, &self.expected_panic_flag, should_panic, id)
        }

        pub fn finish(self, panic_res: std::thread::Result<()>) -> Vec<DropLogItem> {
            let drop_log = self.drop_log.lock().unwrap();
            let expected_panic_flag = self.expected_panic_flag.load(Ordering::SeqCst);

            if !expected_panic_flag {
                match panic_res {
                    Ok(()) => panic!("Expected a panic but it didn't happen"),
                    Err(e) => panic::resume_unwind(e),
                }
            }

            drop_log.to_owned()
        }
    }

    #[test]
    fn panic_while_overwriting_component() {
        let helper = DropTestHelper::new();

        let res = panic::catch_unwind(|| {
            let mut world = World::new();
            world
                .spawn()
                .insert(helper.make_component(true, 0))
                .insert(helper.make_component(false, 1));

            println!("Done inserting! Dropping world...");
        });

        let drop_log = helper.finish(res);

        assert_eq!(
            &*drop_log,
            [
                DropLogItem::Create(0),
                DropLogItem::Create(1),
                DropLogItem::Drop(0),
                DropLogItem::Drop(1),
            ]
        );
    }

    #[derive(Component)]
    struct TestResource(u32);

    #[test]
    fn get_resource_by_id() {
        let mut world = World::new();
        world.insert_resource(TestResource(42));
        let component_id = world
            .components()
            .get_resource_id(std::any::TypeId::of::<TestResource>())
            .unwrap();

        let resource = world.get_resource_by_id(component_id).unwrap();
        // SAFETY: `TestResource` is the correct resource type
        let resource = unsafe { resource.deref::<TestResource>() };

        assert_eq!(resource.0, 42);
    }

    #[test]
    fn get_resource_mut_by_id() {
        let mut world = World::new();
        world.insert_resource(TestResource(42));
        let component_id = world
            .components()
            .get_resource_id(std::any::TypeId::of::<TestResource>())
            .unwrap();

        {
            let mut resource = world.get_resource_mut_by_id(component_id).unwrap();
            resource.set_changed();
            // SAFETY: `TestResource` is the correct resource type
            let resource = unsafe { resource.into_inner().deref_mut::<TestResource>() };
            resource.0 = 43;
        }

        let resource = world.get_resource_by_id(component_id).unwrap();
        // SAFETY: `TestResource` is the correct resource type
        let resource = unsafe { resource.deref::<TestResource>() };

        assert_eq!(resource.0, 43);
    }

    #[test]
    fn custom_resource_with_layout() {
        static DROP_COUNT: AtomicU32 = AtomicU32::new(0);

        let mut world = World::new();

        // SAFETY: the drop function is valid for the layout and the data will be safe to access from any thread
        let descriptor = unsafe {
            ComponentDescriptor::new_with_layout(
                "Custom Test Component".to_string(),
                StorageType::Table,
                std::alloc::Layout::new::<[u8; 8]>(),
                Some(|ptr| {
                    let data = ptr.read::<[u8; 8]>();
                    assert_eq!(data, [0, 1, 2, 3, 4, 5, 6, 7]);
                    DROP_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }),
            )
        };

        let component_id = world.init_component_with_descriptor(descriptor);

        let value: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
        OwningPtr::make(value, |ptr| {
            // SAFETY: value is valid for the component layout
            unsafe {
                world.insert_resource_by_id(component_id, ptr);
            }
        });

        // SAFETY: [u8; 8] is the correct type for the resource
        let data = unsafe {
            world
                .get_resource_by_id(component_id)
                .unwrap()
                .deref::<[u8; 8]>()
        };
        assert_eq!(*data, [0, 1, 2, 3, 4, 5, 6, 7]);

        assert!(world.remove_resource_by_id(component_id).is_some());

        assert_eq!(DROP_COUNT.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    #[should_panic = "insert_resource_by_id called with component id which doesn't exist in this world"]
    fn insert_resource_by_id_invalid_component_id() {
        let invalid_component_id = ComponentId::new(usize::MAX);

        let mut world = World::new();
        OwningPtr::make((), |ptr| {
            // SAFETY: ptr must be valid for the component_id `invalid_component_id` which is invalid, but checked by `insert_resource_by_id`
            unsafe {
                world.insert_resource_by_id(invalid_component_id, ptr);
            }
        });
    }

    #[derive(Component)]
    struct Foo;

    #[derive(Component)]
    struct Bar;

    #[derive(Component)]
    struct Baz;

    #[test]
    fn inspect_entity_components() {
        let mut world = World::new();
        let ent0 = world.spawn().insert_bundle((Foo, Bar, Baz)).id();
        let ent1 = world.spawn().insert_bundle((Foo, Bar)).id();
        let ent2 = world.spawn().insert_bundle((Bar, Baz)).id();
        let ent3 = world.spawn().insert_bundle((Foo, Baz)).id();
        let ent4 = world.spawn().insert_bundle((Foo,)).id();
        let ent5 = world.spawn().insert_bundle((Bar,)).id();
        let ent6 = world.spawn().insert_bundle((Baz,)).id();

        fn to_type_ids(component_infos: Vec<&ComponentInfo>) -> HashSet<Option<TypeId>> {
            component_infos
                .into_iter()
                .map(|component_info| component_info.type_id())
                .collect()
        }

        let foo_id = TypeId::of::<Foo>();
        let bar_id = TypeId::of::<Bar>();
        let baz_id = TypeId::of::<Baz>();
        assert_eq!(
            to_type_ids(world.inspect_entity(ent0)),
            [Some(foo_id), Some(bar_id), Some(baz_id)].into()
        );
        assert_eq!(
            to_type_ids(world.inspect_entity(ent1)),
            [Some(foo_id), Some(bar_id)].into()
        );
        assert_eq!(
            to_type_ids(world.inspect_entity(ent2)),
            [Some(bar_id), Some(baz_id)].into()
        );
        assert_eq!(
            to_type_ids(world.inspect_entity(ent3)),
            [Some(foo_id), Some(baz_id)].into()
        );
        assert_eq!(
            to_type_ids(world.inspect_entity(ent4)),
            [Some(foo_id)].into()
        );
        assert_eq!(
            to_type_ids(world.inspect_entity(ent5)),
            [Some(bar_id)].into()
        );
        assert_eq!(
            to_type_ids(world.inspect_entity(ent6)),
            [Some(baz_id)].into()
        );
    }
}
