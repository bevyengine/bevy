mod entity_ref;
mod spawn_batch;
mod world_cell;

pub use crate::change_detection::Mut;
pub use entity_ref::*;
pub use spawn_batch::*;
pub use world_cell::*;

use crate::{
    archetype::{ArchetypeComponentId, ArchetypeId, Archetypes},
    bundle::{Bundle, BundleInserter, BundleSpawner, Bundles},
    change_detection::{MutUntyped, Ticks},
    component::{
        Component, ComponentDescriptor, ComponentId, ComponentInfo, Components, TickCells,
    },
    entity::{AllocAtWithoutReplacement, Entities, Entity},
    query::{QueryState, ReadOnlyWorldQuery, WorldQuery},
    storage::{ResourceData, SparseSet, Storages},
    system::Resource,
};
use bevy_ptr::{OwningPtr, Ptr};
use bevy_utils::tracing::warn;
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
/// ## Resources
///
/// Worlds can also store [`Resource`]s,
/// which are unique instances of a given type that don't belong to a specific Entity.
/// There are also *non send resources*, which can only be accessed on the main thread.
/// See [`Resource`] for usage.
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
    /// let entity = world.spawn(Position { x: 0.0, y: 0.0 }).id();
    /// let position = world.entity(entity).get::<Position>().unwrap();
    /// assert_eq!(position.x, 0.0);
    /// ```
    #[inline]
    pub fn entity(&self, entity: Entity) -> EntityRef {
        // Lazily evaluate panic!() via unwrap_or_else() to avoid allocation unless failure
        self.get_entity(entity)
            .unwrap_or_else(|| panic!("Entity {entity:?} does not exist"))
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
    /// let entity = world.spawn(Position { x: 0.0, y: 0.0 }).id();
    /// let mut entity_mut = world.entity_mut(entity);
    /// let mut position = entity_mut.get_mut::<Position>().unwrap();
    /// position.x = 1.0;
    /// ```
    #[inline]
    pub fn entity_mut(&mut self, entity: Entity) -> EntityMut {
        // Lazily evaluate panic!() via unwrap_or_else() to avoid allocation unless failure
        self.get_entity_mut(entity)
            .unwrap_or_else(|| panic!("Entity {entity:?} does not exist"))
    }

    /// Returns the components of an [`Entity`](crate::entity::Entity) through [`ComponentInfo`](crate::component::ComponentInfo).
    #[inline]
    pub fn inspect_entity(&self, entity: Entity) -> Vec<&ComponentInfo> {
        let entity_location = self
            .entities()
            .get(entity)
            .unwrap_or_else(|| panic!("Entity {entity:?} does not exist"));

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
                Some(unsafe { self.spawn_at_empty_internal(entity) })
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
    /// let entity = world.spawn(Position { x: 0.0, y: 0.0 }).id();
    /// let entity_ref = world.get_entity(entity).unwrap();
    /// let position = entity_ref.get::<Position>().unwrap();
    /// assert_eq!(position.x, 0.0);
    /// ```
    #[inline]
    pub fn get_entity(&self, entity: Entity) -> Option<EntityRef> {
        let location = self.entities.get(entity)?;
        Some(EntityRef::new(self, entity, location))
    }

    /// Returns an [`Entity`] iterator of current entities.
    ///
    /// This is useful in contexts where you only have read-only access to the [`World`].
    #[inline]
    pub fn iter_entities(&self) -> impl Iterator<Item = Entity> + '_ {
        self.archetypes
            .iter()
            .flat_map(|archetype| archetype.entities().iter())
            .map(|archetype_entity| archetype_entity.entity)
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
    /// let entity = world.spawn(Position { x: 0.0, y: 0.0 }).id();
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
    /// let entity = world.spawn_empty()
    ///     .insert(Position { x: 0.0, y: 0.0 }) // add a single component
    ///     .insert((Num(1), Label("hello"))) // add a bundle of components
    ///     .id();
    ///
    /// let position = world.entity(entity).get::<Position>().unwrap();
    /// assert_eq!(position.x, 0.0);
    /// ```
    pub fn spawn_empty(&mut self) -> EntityMut {
        self.flush();
        let entity = self.entities.alloc();
        // SAFETY: entity was just allocated
        unsafe { self.spawn_at_empty_internal(entity) }
    }

    /// Spawns a new [`Entity`] with a given [`Bundle`] of [components](`Component`) and returns
    /// a corresponding [`EntityMut`], which can be used to add components to the entity or
    /// retrieve its id.
    ///
    /// ```
    /// use bevy_ecs::{bundle::Bundle, component::Component, world::World};
    ///
    /// #[derive(Component)]
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// #[derive(Component)]
    /// struct Velocity {
    ///     x: f32,
    ///     y: f32,
    /// };
    ///
    /// #[derive(Component)]
    /// struct Name(&'static str);
    ///
    /// #[derive(Bundle)]
    /// struct PhysicsBundle {
    ///     position: Position,
    ///     velocity: Velocity,
    /// }
    ///
    /// let mut world = World::new();
    ///
    /// // `spawn` can accept a single component:
    /// world.spawn(Position { x: 0.0, y: 0.0 });

    /// // It can also accept a tuple of components:
    /// world.spawn((
    ///     Position { x: 0.0, y: 0.0 },
    ///     Velocity { x: 1.0, y: 1.0 },
    /// ));

    /// // Or it can accept a pre-defined Bundle of components:
    /// world.spawn(PhysicsBundle {
    ///     position: Position { x: 2.0, y: 2.0 },
    ///     velocity: Velocity { x: 0.0, y: 4.0 },
    /// });
    ///
    /// let entity = world
    ///     // Tuples can also mix Bundles and Components
    ///     .spawn((
    ///         PhysicsBundle {
    ///             position: Position { x: 2.0, y: 2.0 },
    ///             velocity: Velocity { x: 0.0, y: 4.0 },
    ///         },
    ///         Name("Elaina Proctor"),
    ///     ))
    ///     // Calling id() will return the unique identifier for the spawned entity
    ///     .id();
    /// let position = world.entity(entity).get::<Position>().unwrap();
    /// assert_eq!(position.x, 2.0);
    /// ```
    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> EntityMut {
        self.flush();
        let entity = self.entities.alloc();
        let entity_location = {
            let bundle_info = self
                .bundles
                .init_info::<B>(&mut self.components, &mut self.storages);
            let mut spawner = bundle_info.get_bundle_spawner(
                &mut self.entities,
                &mut self.archetypes,
                &mut self.components,
                &mut self.storages,
                *self.change_tick.get_mut(),
            );

            // SAFETY: bundle's type matches `bundle_info`, entity is allocated but non-existent
            unsafe { spawner.spawn_non_existent(entity, bundle) }
        };

        // SAFETY: entity and location are valid, as they were just created above
        unsafe { EntityMut::new(self, entity, entity_location) }
    }

    /// # Safety
    /// must be called on an entity that was just allocated
    unsafe fn spawn_at_empty_internal(&mut self, entity: Entity) -> EntityMut {
        let archetype = self.archetypes.empty_mut();
        // PERF: consider avoiding allocating entities in the empty archetype unless needed
        let table_row = self.storages.tables[archetype.table_id()].allocate(entity);
        // SAFETY: no components are allocated by archetype.allocate() because the archetype is
        // empty
        let location = archetype.allocate(entity, table_row);
        // SAFETY: entity index was just allocated
        self.entities
            .meta
            .get_unchecked_mut(entity.index() as usize)
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
    /// let entity = world.spawn(Position { x: 0.0, y: 0.0 }).id();
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
    /// let entity = world.spawn(Position { x: 0.0, y: 0.0 }).id();
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
    /// let entity = world.spawn(Position { x: 0.0, y: 0.0 }).id();
    /// assert!(world.despawn(entity));
    /// assert!(world.get_entity(entity).is_none());
    /// assert!(world.get::<Position>(entity).is_none());
    /// ```
    #[inline]
    pub fn despawn(&mut self, entity: Entity) -> bool {
        if let Some(entity) = self.get_entity_mut(entity) {
            entity.despawn();
            true
        } else {
            warn!("error[B0003]: Could not despawn entity {:?} because it doesn't exist in this World.", entity);
            false
        }
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
    /// let a = world.spawn((Order(2), Label("second"))).id();
    /// let b = world.spawn((Order(3), Label("third"))).id();
    /// let c = world.spawn((Order(1), Label("first"))).id();
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
    /// let e1 = world.spawn(A).id();
    /// let e2 = world.spawn((A, B)).id();
    ///
    /// let mut query = world.query_filtered::<Entity, With<B>>();
    /// let matching_entities = query.iter(&world).collect::<Vec<Entity>>();
    ///
    /// assert_eq!(matching_entities, vec![e2]);
    /// ```
    #[inline]
    pub fn query_filtered<Q: WorldQuery, F: ReadOnlyWorldQuery>(&mut self) -> QueryState<Q, F> {
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
        OwningPtr::make(value, |ptr| {
            // SAFETY: component_id was just initialized and corresponds to resource of type R
            unsafe {
                self.insert_resource_by_id(component_id, ptr);
            }
        });
    }

    /// Inserts a new non-send resource with standard starting values.
    ///
    /// If the resource already exists, nothing happens.
    ///
    /// The value given by the [`FromWorld::from_world`] method will be used.
    /// Note that any resource with the `Default` trait automatically implements `FromWorld`,
    /// and those default values will be here instead.
    ///
    /// # Panics
    ///
    /// Panics if called from a thread other than the main thread.
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
    ///
    /// # Panics
    ///
    /// Panics if called from a thread other than the main thread.
    #[inline]
    pub fn insert_non_send_resource<R: 'static>(&mut self, value: R) {
        self.validate_non_send_access::<R>();
        let component_id = self.components.init_non_send::<R>();
        OwningPtr::make(value, |ptr| {
            // SAFETY: component_id was just initialized and corresponds to resource of type R
            unsafe {
                self.insert_resource_by_id(component_id, ptr);
            }
        });
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
    /// as they cannot be sent across threads
    #[allow(unused_unsafe)]
    pub unsafe fn remove_resource_unchecked<R: 'static>(&mut self) -> Option<R> {
        let component_id = self.components.get_resource_id(TypeId::of::<R>())?;
        // SAFETY: the resource is of type R and the value is returned back to the caller.
        unsafe {
            let (ptr, _) = self.storages.resources.get_mut(component_id)?.remove()?;
            Some(ptr.read::<R>())
        }
    }

    /// Returns `true` if a resource of type `R` exists. Otherwise returns `false`.
    #[inline]
    pub fn contains_resource<R: 'static>(&self) -> bool {
        self.components
            .get_resource_id(TypeId::of::<R>())
            .and_then(|component_id| self.storages.resources.get(component_id))
            .map(|info| info.is_present())
            .unwrap_or(false)
    }

    pub fn is_resource_added<R: Resource>(&self) -> bool {
        self.components
            .get_resource_id(TypeId::of::<R>())
            .and_then(|component_id| self.storages.resources.get(component_id)?.get_ticks())
            .map(|ticks| ticks.is_added(self.last_change_tick(), self.read_change_tick()))
            .unwrap_or(false)
    }

    pub fn is_resource_changed<R: Resource>(&self) -> bool {
        self.components
            .get_resource_id(TypeId::of::<R>())
            .and_then(|component_id| self.storages.resources.get(component_id)?.get_ticks())
            .map(|ticks| ticks.is_changed(self.last_change_tick(), self.read_change_tick()))
            .unwrap_or(false)
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

    // Shorthand helper function for getting the data and change ticks for a resource.
    #[inline]
    pub(crate) fn get_resource_with_ticks(
        &self,
        component_id: ComponentId,
    ) -> Option<(Ptr<'_>, TickCells<'_>)> {
        self.storages.resources.get(component_id)?.get_with_ticks()
    }

    // Shorthand helper function for getting the [`ArchetypeComponentId`] for a resource.
    #[inline]
    pub(crate) fn get_resource_archetype_component_id(
        &self,
        component_id: ComponentId,
    ) -> Option<ArchetypeComponentId> {
        let resource = self.storages.resources.get(component_id)?;
        Some(resource.id())
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
    /// let e0 = world.spawn_empty().id();
    /// let e1 = world.spawn_empty().id();
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
    /// use bevy_ecs::prelude::*;
    /// #[derive(Resource)]
    /// struct A(u32);
    /// #[derive(Component)]
    /// struct B(u32);
    /// let mut world = World::new();
    /// world.insert_resource(A(1));
    /// let entity = world.spawn(B(1)).id();
    ///
    /// world.resource_scope(|world, mut a: Mut<A>| {
    ///     let b = world.get_mut::<B>(entity).unwrap();
    ///     a.0 += b.0;
    /// });
    /// assert_eq!(world.get_resource::<A>().unwrap().0, 2);
    /// ```
    pub fn resource_scope<
        R: 'static, /* The resource doesn't need to be Send nor Sync. */
        U,
    >(
        &mut self,
        f: impl FnOnce(&mut World, Mut<R>) -> U,
    ) -> U {
        let last_change_tick = self.last_change_tick();
        let change_tick = self.change_tick();

        let component_id = self
            .components
            .get_resource_id(TypeId::of::<R>())
            .unwrap_or_else(|| panic!("resource does not exist: {}", std::any::type_name::<R>()));
        // If the resource isn't send and sync, validate that we are on the main thread, so that we can access it.
        let component_info = self.components().get_info(component_id).unwrap();
        if !component_info.is_send_and_sync() {
            self.validate_non_send_access::<R>();
        }

        let (ptr, mut ticks) = self
            .storages
            .resources
            .get_mut(component_id)
            // SAFETY: The type R is Send and Sync or we've already validated that we're on the main thread.
            .and_then(|info| unsafe { info.remove() })
            .unwrap_or_else(|| panic!("resource does not exist: {}", std::any::type_name::<R>()));
        // Read the value onto the stack to avoid potential mut aliasing.
        // SAFETY: pointer is of type R
        let mut value = unsafe { ptr.read::<R>() };
        let value_mut = Mut {
            value: &mut value,
            ticks: Ticks {
                added: &mut ticks.added,
                changed: &mut ticks.changed,
                last_change_tick,
                change_tick,
            },
        };
        let result = f(self, value_mut);
        assert!(!self.contains_resource::<R>(),
            "Resource `{}` was inserted during a call to World::resource_scope.\n\
            This is not allowed as the original resource is reinserted to the world after the FnOnce param is invoked.",
            std::any::type_name::<R>());

        OwningPtr::make(value, |ptr| {
            // SAFETY: pointer is of type R
            unsafe {
                self.storages
                    .resources
                    .get_mut(component_id)
                    .map(|info| info.insert_with_ticks(ptr, ticks))
                    .unwrap_or_else(|| {
                        panic!(
                            "No resource of type {} exists in the World.",
                            std::any::type_name::<R>()
                        )
                    });
            }
        });

        result
    }

    /// Sends an [`Event`](crate::event::Event).
    #[inline]
    pub fn send_event<E: crate::event::Event>(&mut self, event: E) {
        self.send_event_batch(std::iter::once(event));
    }

    /// Sends the default value of the [`Event`](crate::event::Event) of type `E`.
    #[inline]
    pub fn send_event_default<E: crate::event::Event + Default>(&mut self) {
        self.send_event_batch(std::iter::once(E::default()));
    }

    /// Sends a batch of [`Event`](crate::event::Event)s from an iterator.
    #[inline]
    pub fn send_event_batch<E: crate::event::Event>(&mut self, events: impl Iterator<Item = E>) {
        match self.get_resource_mut::<crate::event::Events<E>>() {
            Some(mut events_resource) => events_resource.extend(events),
            None => bevy_utils::tracing::error!(
                    "Unable to send event `{}`\n\tEvent must be added to the app with `add_event()`\n\thttps://docs.rs/bevy/*/bevy/app/struct.App.html#method.add_event ",
                    std::any::type_name::<E>()
                ),
        }
    }

    /// # Safety
    /// `component_id` must be assigned to a component of type `R`
    #[inline]
    pub(crate) unsafe fn get_resource_with_id<R: 'static>(
        &self,
        component_id: ComponentId,
    ) -> Option<&R> {
        self.storages
            .resources
            .get(component_id)?
            .get_data()
            .map(|ptr| ptr.deref())
    }

    /// # Safety
    /// `component_id` must be assigned to a component of type `R`
    /// Caller must ensure this doesn't violate Rust mutability rules for the given resource.
    #[inline]
    pub(crate) unsafe fn get_resource_unchecked_mut_with_id<R>(
        &self,
        component_id: ComponentId,
    ) -> Option<Mut<'_, R>> {
        let (ptr, ticks) = self.get_resource_with_ticks(component_id)?;
        Some(Mut {
            value: ptr.assert_unique().deref_mut(),
            ticks: Ticks::from_tick_cells(ticks, self.last_change_tick(), self.read_change_tick()),
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

    /// Inserts a new resource with the given `value`. Will replace the value if it already existed.
    ///
    /// **You should prefer to use the typed API [`World::insert_resource`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    ///
    /// # Safety
    /// The value referenced by `value` must be valid for the given [`ComponentId`] of this world
    /// `component_id` must exist in this [`World`]
    #[inline]
    pub unsafe fn insert_resource_by_id(
        &mut self,
        component_id: ComponentId,
        value: OwningPtr<'_>,
    ) {
        let change_tick = self.change_tick();

        // SAFETY: component_id is valid, ensured by caller
        self.initialize_resource_internal(component_id)
            .insert(value, change_tick);
    }

    /// # Safety
    /// `component_id` must be valid for this world
    #[inline]
    unsafe fn initialize_resource_internal(
        &mut self,
        component_id: ComponentId,
    ) -> &mut ResourceData {
        let archetype_component_count = &mut self.archetypes.archetype_component_count;
        self.storages
            .resources
            .initialize_with(component_id, &self.components, || {
                let id = ArchetypeComponentId::new(*archetype_component_count);
                *archetype_component_count += 1;
                id
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
        self.storages.resources.check_change_ticks(change_tick);
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
        self.storages.resources.get(component_id)?.get_data()
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

        let (ptr, ticks) = self.get_resource_with_ticks(component_id)?;

        // SAFETY: This function has exclusive access to the world so nothing aliases `ticks`.
        // - index is in-bounds because the column is initialized and non-empty
        // - no other reference to the ticks of the same row can exist at the same time
        let ticks = unsafe {
            Ticks::from_tick_cells(ticks, self.last_change_tick(), self.read_change_tick())
        };

        Some(MutUntyped {
            // SAFETY: This function has exclusive access to the world so nothing aliases `ptr`.
            value: unsafe { ptr.assert_unique() },
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
        // SAFETY: The underlying type is Send and Sync or we've already validated we're on the main thread
        unsafe {
            self.storages
                .resources
                .get_mut(component_id)?
                .remove_and_drop();
        }
        Some(())
    }

    /// Retrieves an immutable untyped reference to the given `entity`'s [Component] of the given [`ComponentId`].
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
            .field("resource_count", &self.storages.resources.len())
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
        component::{ComponentDescriptor, ComponentInfo, StorageType},
        ptr::OwningPtr,
        system::Resource,
    };
    use bevy_ecs_macros::Component;
    use bevy_utils::{HashMap, HashSet};
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

    #[derive(Resource, Component)]
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
            println!("creating component with id {id}");
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
                .spawn_empty()
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

    #[derive(Resource)]
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

    #[derive(Component)]
    struct Foo;

    #[derive(Component)]
    struct Bar;

    #[derive(Component)]
    struct Baz;

    #[test]
    fn inspect_entity_components() {
        let mut world = World::new();
        let ent0 = world.spawn((Foo, Bar, Baz)).id();
        let ent1 = world.spawn((Foo, Bar)).id();
        let ent2 = world.spawn((Bar, Baz)).id();
        let ent3 = world.spawn((Foo, Baz)).id();
        let ent4 = world.spawn(Foo).id();
        let ent5 = world.spawn(Bar).id();
        let ent6 = world.spawn(Baz).id();

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

    #[test]
    fn iterate_entities() {
        let mut world = World::new();
        let mut entity_counters = HashMap::new();

        let iterate_and_count_entities = |world: &World, entity_counters: &mut HashMap<_, _>| {
            entity_counters.clear();
            for entity in world.iter_entities() {
                let counter = entity_counters.entry(entity).or_insert(0);
                *counter += 1;
            }
        };

        // Adding one entity and validating iteration
        let ent0 = world.spawn((Foo, Bar, Baz)).id();

        iterate_and_count_entities(&world, &mut entity_counters);
        assert_eq!(entity_counters[&ent0], 1);
        assert_eq!(entity_counters.len(), 1);

        // Spawning three more entities and then validating iteration
        let ent1 = world.spawn((Foo, Bar)).id();
        let ent2 = world.spawn((Bar, Baz)).id();
        let ent3 = world.spawn((Foo, Baz)).id();

        iterate_and_count_entities(&world, &mut entity_counters);

        assert_eq!(entity_counters[&ent0], 1);
        assert_eq!(entity_counters[&ent1], 1);
        assert_eq!(entity_counters[&ent2], 1);
        assert_eq!(entity_counters[&ent3], 1);
        assert_eq!(entity_counters.len(), 4);

        // Despawning first entity and then validating the iteration
        assert!(world.despawn(ent0));

        iterate_and_count_entities(&world, &mut entity_counters);

        assert_eq!(entity_counters[&ent1], 1);
        assert_eq!(entity_counters[&ent2], 1);
        assert_eq!(entity_counters[&ent3], 1);
        assert_eq!(entity_counters.len(), 3);

        // Spawning three more entities, despawning three and then validating the iteration
        let ent4 = world.spawn(Foo).id();
        let ent5 = world.spawn(Bar).id();
        let ent6 = world.spawn(Baz).id();

        assert!(world.despawn(ent2));
        assert!(world.despawn(ent3));
        assert!(world.despawn(ent4));

        iterate_and_count_entities(&world, &mut entity_counters);

        assert_eq!(entity_counters[&ent1], 1);
        assert_eq!(entity_counters[&ent5], 1);
        assert_eq!(entity_counters[&ent6], 1);
        assert_eq!(entity_counters.len(), 3);

        // Despawning remaining entities and then validating the iteration
        assert!(world.despawn(ent1));
        assert!(world.despawn(ent5));
        assert!(world.despawn(ent6));

        iterate_and_count_entities(&world, &mut entity_counters);

        assert_eq!(entity_counters.len(), 0);
    }

    #[test]
    fn spawn_empty_bundle() {
        let mut world = World::new();
        world.spawn(());
    }
}
