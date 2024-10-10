//! Defines the [`World`] and APIs for accessing it directly.

pub(crate) mod command_queue;
mod component_constants;
mod deferred_world;
mod entity_fetch;
mod entity_ref;
pub mod error;
mod filtered_resource;
mod identifier;
mod spawn_batch;
pub mod unsafe_world_cell;

#[cfg(feature = "bevy_reflect")]
pub mod reflect;

pub use crate::{
    change_detection::{Mut, Ref, CHECK_TICK_THRESHOLD},
    world::command_queue::CommandQueue,
};
pub use component_constants::*;
pub use deferred_world::DeferredWorld;
pub use entity_fetch::WorldEntityFetch;
pub use entity_ref::{
    EntityMut, EntityMutExcept, EntityRef, EntityRefExcept, EntityWorldMut, Entry,
    FilteredEntityMut, FilteredEntityRef, OccupiedEntry, VacantEntry,
};
pub use filtered_resource::*;
pub use identifier::WorldId;
pub use spawn_batch::*;

use crate::{
    archetype::{ArchetypeId, ArchetypeRow, Archetypes},
    bundle::{Bundle, BundleInfo, BundleInserter, BundleSpawner, Bundles, InsertMode},
    change_detection::{MutUntyped, TicksMut},
    component::{
        Component, ComponentDescriptor, ComponentHooks, ComponentId, ComponentInfo, ComponentTicks,
        Components, RequiredComponents, RequiredComponentsError, Tick,
    },
    entity::{AllocAtWithoutReplacement, Entities, Entity, EntityHashSet, EntityLocation},
    event::{Event, EventId, Events, SendBatchIds},
    observer::Observers,
    query::{DebugCheckedUnwrap, QueryData, QueryEntityError, QueryFilter, QueryState},
    removal_detection::RemovedComponentEvents,
    schedule::{Schedule, ScheduleLabel, Schedules},
    storage::{ResourceData, Storages},
    system::{Commands, Resource},
    world::{
        command_queue::RawCommandQueue,
        error::{EntityFetchError, TryRunScheduleError},
    },
};
use bevy_ptr::{OwningPtr, Ptr};
use bevy_utils::tracing::warn;
use core::{
    any::TypeId,
    fmt,
    sync::atomic::{AtomicU32, Ordering},
};

#[cfg(feature = "track_change_detection")]
use bevy_ptr::UnsafeCellDeref;

use core::panic::Location;

use unsafe_world_cell::{UnsafeEntityCell, UnsafeWorldCell};

/// A [`World`] mutation.
///
/// Should be used with [`Commands::queue`].
///
/// # Usage
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::world::Command;
/// // Our world resource
/// #[derive(Resource, Default)]
/// struct Counter(u64);
///
/// // Our custom command
/// struct AddToCounter(u64);
///
/// impl Command for AddToCounter {
///     fn apply(self, world: &mut World) {
///         let mut counter = world.get_resource_or_insert_with(Counter::default);
///         counter.0 += self.0;
///     }
/// }
///
/// fn some_system(mut commands: Commands) {
///     commands.queue(AddToCounter(42));
/// }
/// ```
pub trait Command: Send + 'static {
    /// Applies this command, causing it to mutate the provided `world`.
    ///
    /// This method is used to define what a command "does" when it is ultimately applied.
    /// Because this method takes `self`, you can store data or settings on the type that implements this trait.
    /// This data is set by the system or other source of the command, and then ultimately read in this method.
    fn apply(self, world: &mut World);
}

/// Stores and exposes operations on [entities](Entity), [components](Component), resources,
/// and their associated metadata.
///
/// Each [`Entity`] has a set of components. Each component can have up to one instance of each
/// component type. Entity components can be created, updated, removed, and queried using a given
/// [`World`].
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
    pub(crate) observers: Observers,
    pub(crate) removed_components: RemovedComponentEvents,
    pub(crate) change_tick: AtomicU32,
    pub(crate) last_change_tick: Tick,
    pub(crate) last_check_tick: Tick,
    pub(crate) last_trigger_id: u32,
    pub(crate) command_queue: RawCommandQueue,
}

impl Default for World {
    fn default() -> Self {
        let mut world = Self {
            id: WorldId::new().expect("More `bevy` `World`s have been created than is supported"),
            entities: Entities::new(),
            components: Default::default(),
            archetypes: Archetypes::new(),
            storages: Default::default(),
            bundles: Default::default(),
            observers: Observers::default(),
            removed_components: Default::default(),
            // Default value is `1`, and `last_change_tick`s default to `0`, such that changes
            // are detected on first system runs and for direct world queries.
            change_tick: AtomicU32::new(1),
            last_change_tick: Tick::new(0),
            last_check_tick: Tick::new(0),
            last_trigger_id: 0,
            command_queue: RawCommandQueue::new(),
        };
        world.bootstrap();
        world
    }
}

impl Drop for World {
    fn drop(&mut self) {
        // SAFETY: Not passing a pointer so the argument is always valid
        unsafe { self.command_queue.apply_or_drop_queued(None) };
        // SAFETY: Pointers in internal command queue are only invalidated here
        drop(unsafe { Box::from_raw(self.command_queue.bytes.as_ptr()) });
        // SAFETY: Pointers in internal command queue are only invalidated here
        drop(unsafe { Box::from_raw(self.command_queue.cursor.as_ptr()) });
        // SAFETY: Pointers in internal command queue are only invalidated here
        drop(unsafe { Box::from_raw(self.command_queue.panic_recovery.as_ptr()) });
    }
}

impl World {
    /// This performs initialization that _must_ happen for every [`World`] immediately upon creation (such as claiming specific component ids).
    /// This _must_ be run as part of constructing a [`World`], before it is returned to the caller.
    #[inline]
    fn bootstrap(&mut self) {
        assert_eq!(ON_ADD, self.register_component::<OnAdd>());
        assert_eq!(ON_INSERT, self.register_component::<OnInsert>());
        assert_eq!(ON_REPLACE, self.register_component::<OnReplace>());
        assert_eq!(ON_REMOVE, self.register_component::<OnRemove>());
    }
    /// Creates a new empty [`World`].
    ///
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

    /// Creates a new [`UnsafeWorldCell`] view with complete read+write access.
    #[inline]
    pub fn as_unsafe_world_cell(&mut self) -> UnsafeWorldCell<'_> {
        UnsafeWorldCell::new_mutable(self)
    }

    /// Creates a new [`UnsafeWorldCell`] view with only read access to everything.
    #[inline]
    pub fn as_unsafe_world_cell_readonly(&self) -> UnsafeWorldCell<'_> {
        UnsafeWorldCell::new_readonly(self)
    }

    /// Retrieves this world's [`Entities`] collection.
    #[inline]
    pub fn entities(&self) -> &Entities {
        &self.entities
    }

    /// Retrieves this world's [`Entities`] collection mutably.
    ///
    /// # Safety
    /// Mutable reference must not be used to put the [`Entities`] data
    /// in an invalid state for this [`World`]
    #[inline]
    pub unsafe fn entities_mut(&mut self) -> &mut Entities {
        &mut self.entities
    }

    /// Retrieves this world's [`Archetypes`] collection.
    #[inline]
    pub fn archetypes(&self) -> &Archetypes {
        &self.archetypes
    }

    /// Retrieves this world's [`Components`] collection.
    #[inline]
    pub fn components(&self) -> &Components {
        &self.components
    }

    /// Retrieves this world's [`Storages`] collection.
    #[inline]
    pub fn storages(&self) -> &Storages {
        &self.storages
    }

    /// Retrieves this world's [`Bundles`] collection.
    #[inline]
    pub fn bundles(&self) -> &Bundles {
        &self.bundles
    }

    /// Retrieves this world's [`RemovedComponentEvents`] collection
    #[inline]
    pub fn removed_components(&self) -> &RemovedComponentEvents {
        &self.removed_components
    }

    /// Creates a new [`Commands`] instance that writes to the world's command queue
    /// Use [`World::flush`] to apply all queued commands
    #[inline]
    pub fn commands(&mut self) -> Commands {
        // SAFETY: command_queue is stored on world and always valid while the world exists
        unsafe { Commands::new_raw_from_entities(self.command_queue.clone(), &self.entities) }
    }

    /// Registers a new [`Component`] type and returns the [`ComponentId`] created for it.
    pub fn register_component<T: Component>(&mut self) -> ComponentId {
        self.components.register_component::<T>(&mut self.storages)
    }

    /// Returns a mutable reference to the [`ComponentHooks`] for a [`Component`] type.
    ///
    /// Will panic if `T` exists in any archetypes.
    pub fn register_component_hooks<T: Component>(&mut self) -> &mut ComponentHooks {
        let index = self.register_component::<T>();
        assert!(!self.archetypes.archetypes.iter().any(|a| a.contains(index)), "Components hooks cannot be modified if the component already exists in an archetype, use register_component if {} may already be in use", core::any::type_name::<T>());
        // SAFETY: We just created this component
        unsafe { self.components.get_hooks_mut(index).debug_checked_unwrap() }
    }

    /// Returns a mutable reference to the [`ComponentHooks`] for a [`Component`] with the given id if it exists.
    ///
    /// Will panic if `id` exists in any archetypes.
    pub fn register_component_hooks_by_id(
        &mut self,
        id: ComponentId,
    ) -> Option<&mut ComponentHooks> {
        assert!(!self.archetypes.archetypes.iter().any(|a| a.contains(id)), "Components hooks cannot be modified if the component already exists in an archetype, use register_component if the component with id {:?} may already be in use", id);
        self.components.get_hooks_mut(id)
    }

    /// Registers the given component `R` as a [required component] for `T`.
    ///
    /// When `T` is added to an entity, `R` and its own required components will also be added
    /// if `R` was not already provided. The [`Default`] `constructor` will be used for the creation of `R`.
    /// If a custom constructor is desired, use [`World::register_required_components_with`] instead.
    ///
    /// For the non-panicking version, see [`World::try_register_required_components`].
    ///
    /// Note that requirements must currently be registered before `T` is inserted into the world
    /// for the first time. This limitation may be fixed in the future.
    ///
    /// [required component]: Component#required-components
    ///
    /// # Panics
    ///
    /// Panics if `R` is already a directly required component for `T`, or if `T` has ever been added
    /// on an entity before the registration.
    ///
    /// Indirect requirements through other components are allowed. In those cases, any existing requirements
    /// will only be overwritten if the new requirement is more specific.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component)]
    /// struct A;
    ///
    /// #[derive(Component, Default, PartialEq, Eq, Debug)]
    /// struct B(usize);
    ///
    /// #[derive(Component, Default, PartialEq, Eq, Debug)]
    /// struct C(u32);
    ///
    /// # let mut world = World::default();
    /// // Register B as required by A and C as required by B.
    /// world.register_required_components::<A, B>();
    /// world.register_required_components::<B, C>();
    ///
    /// // This will implicitly also insert B and C with their Default constructors.
    /// let id = world.spawn(A).id();
    /// assert_eq!(&B(0), world.entity(id).get::<B>().unwrap());
    /// assert_eq!(&C(0), world.entity(id).get::<C>().unwrap());
    /// ```
    pub fn register_required_components<T: Component, R: Component + Default>(&mut self) {
        self.try_register_required_components::<T, R>().unwrap();
    }

    /// Registers the given component `R` as a [required component] for `T`.
    ///
    /// When `T` is added to an entity, `R` and its own required components will also be added
    /// if `R` was not already provided. The given `constructor` will be used for the creation of `R`.
    /// If a [`Default`] constructor is desired, use [`World::register_required_components`] instead.
    ///
    /// For the non-panicking version, see [`World::try_register_required_components_with`].
    ///
    /// Note that requirements must currently be registered before `T` is inserted into the world
    /// for the first time. This limitation may be fixed in the future.
    ///
    /// [required component]: Component#required-components
    ///
    /// # Panics
    ///
    /// Panics if `R` is already a directly required component for `T`, or if `T` has ever been added
    /// on an entity before the registration.
    ///
    /// Indirect requirements through other components are allowed. In those cases, any existing requirements
    /// will only be overwritten if the new requirement is more specific.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component)]
    /// struct A;
    ///
    /// #[derive(Component, Default, PartialEq, Eq, Debug)]
    /// struct B(usize);
    ///
    /// #[derive(Component, PartialEq, Eq, Debug)]
    /// struct C(u32);
    ///
    /// # let mut world = World::default();
    /// // Register B and C as required by A and C as required by B.
    /// // A requiring C directly will overwrite the indirect requirement through B.
    /// world.register_required_components::<A, B>();
    /// world.register_required_components_with::<B, C>(|| C(1));
    /// world.register_required_components_with::<A, C>(|| C(2));
    ///
    /// // This will implicitly also insert B with its Default constructor and C
    /// // with the custom constructor defined by A.
    /// let id = world.spawn(A).id();
    /// assert_eq!(&B(0), world.entity(id).get::<B>().unwrap());
    /// assert_eq!(&C(2), world.entity(id).get::<C>().unwrap());
    /// ```
    pub fn register_required_components_with<T: Component, R: Component>(
        &mut self,
        constructor: fn() -> R,
    ) {
        self.try_register_required_components_with::<T, R>(constructor)
            .unwrap();
    }

    /// Tries to register the given component `R` as a [required component] for `T`.
    ///
    /// When `T` is added to an entity, `R` and its own required components will also be added
    /// if `R` was not already provided. The [`Default`] `constructor` will be used for the creation of `R`.
    /// If a custom constructor is desired, use [`World::register_required_components_with`] instead.
    ///
    /// For the panicking version, see [`World::register_required_components`].
    ///
    /// Note that requirements must currently be registered before `T` is inserted into the world
    /// for the first time. This limitation may be fixed in the future.
    ///
    /// [required component]: Component#required-components
    ///
    /// # Errors
    ///
    /// Returns a [`RequiredComponentsError`] if `R` is already a directly required component for `T`, or if `T` has ever been added
    /// on an entity before the registration.
    ///
    /// Indirect requirements through other components are allowed. In those cases, any existing requirements
    /// will only be overwritten if the new requirement is more specific.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component)]
    /// struct A;
    ///
    /// #[derive(Component, Default, PartialEq, Eq, Debug)]
    /// struct B(usize);
    ///
    /// #[derive(Component, Default, PartialEq, Eq, Debug)]
    /// struct C(u32);
    ///
    /// # let mut world = World::default();
    /// // Register B as required by A and C as required by B.
    /// world.register_required_components::<A, B>();
    /// world.register_required_components::<B, C>();
    ///
    /// // Duplicate registration! This will fail.
    /// assert!(world.try_register_required_components::<A, B>().is_err());
    ///
    /// // This will implicitly also insert B and C with their Default constructors.
    /// let id = world.spawn(A).id();
    /// assert_eq!(&B(0), world.entity(id).get::<B>().unwrap());
    /// assert_eq!(&C(0), world.entity(id).get::<C>().unwrap());
    /// ```
    pub fn try_register_required_components<T: Component, R: Component + Default>(
        &mut self,
    ) -> Result<(), RequiredComponentsError> {
        self.try_register_required_components_with::<T, R>(R::default)
    }

    /// Tries to register the given component `R` as a [required component] for `T`.
    ///
    /// When `T` is added to an entity, `R` and its own required components will also be added
    /// if `R` was not already provided. The given `constructor` will be used for the creation of `R`.
    /// If a [`Default`] constructor is desired, use [`World::register_required_components`] instead.
    ///
    /// For the panicking version, see [`World::register_required_components_with`].
    ///
    /// Note that requirements must currently be registered before `T` is inserted into the world
    /// for the first time. This limitation may be fixed in the future.
    ///
    /// [required component]: Component#required-components
    ///
    /// # Errors
    ///
    /// Returns a [`RequiredComponentsError`] if `R` is already a directly required component for `T`, or if `T` has ever been added
    /// on an entity before the registration.
    ///
    /// Indirect requirements through other components are allowed. In those cases, any existing requirements
    /// will only be overwritten if the new requirement is more specific.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component)]
    /// struct A;
    ///
    /// #[derive(Component, Default, PartialEq, Eq, Debug)]
    /// struct B(usize);
    ///
    /// #[derive(Component, PartialEq, Eq, Debug)]
    /// struct C(u32);
    ///
    /// # let mut world = World::default();
    /// // Register B and C as required by A and C as required by B.
    /// // A requiring C directly will overwrite the indirect requirement through B.
    /// world.register_required_components::<A, B>();
    /// world.register_required_components_with::<B, C>(|| C(1));
    /// world.register_required_components_with::<A, C>(|| C(2));
    ///
    /// // Duplicate registration! Even if the constructors were different, this would fail.
    /// assert!(world.try_register_required_components_with::<B, C>(|| C(1)).is_err());
    ///
    /// // This will implicitly also insert B with its Default constructor and C
    /// // with the custom constructor defined by A.
    /// let id = world.spawn(A).id();
    /// assert_eq!(&B(0), world.entity(id).get::<B>().unwrap());
    /// assert_eq!(&C(2), world.entity(id).get::<C>().unwrap());
    /// ```
    pub fn try_register_required_components_with<T: Component, R: Component>(
        &mut self,
        constructor: fn() -> R,
    ) -> Result<(), RequiredComponentsError> {
        let requiree = self.register_component::<T>();

        // TODO: Remove this panic and update archetype edges accordingly when required components are added
        if self.archetypes().component_index().contains_key(&requiree) {
            return Err(RequiredComponentsError::ArchetypeExists(requiree));
        }

        let required = self.register_component::<R>();

        // SAFETY: We just created the `required` and `requiree` components.
        unsafe {
            self.components
                .register_required_components::<R>(required, requiree, constructor)
        }
    }

    /// Retrieves the [required components](RequiredComponents) for the given component type, if it exists.
    pub fn get_required_components<C: Component>(&self) -> Option<&RequiredComponents> {
        let id = self.components().component_id::<C>()?;
        let component_info = self.components().get_info(id)?;
        Some(component_info.required_components())
    }

    /// Retrieves the [required components](RequiredComponents) for the component of the given [`ComponentId`], if it exists.
    pub fn get_required_components_by_id(&self, id: ComponentId) -> Option<&RequiredComponents> {
        let component_info = self.components().get_info(id)?;
        Some(component_info.required_components())
    }

    /// Registers a new [`Component`] type and returns the [`ComponentId`] created for it.
    ///
    /// This method differs from [`World::register_component`] in that it uses a [`ComponentDescriptor`]
    /// to register the new component type instead of statically available type information. This
    /// enables the dynamic registration of new component definitions at runtime for advanced use cases.
    ///
    /// While the option to register a component from a descriptor is useful in type-erased
    /// contexts, the standard [`World::register_component`] function should always be used instead
    /// when type information is available at compile time.
    pub fn register_component_with_descriptor(
        &mut self,
        descriptor: ComponentDescriptor,
    ) -> ComponentId {
        self.components
            .register_component_with_descriptor(&mut self.storages, descriptor)
    }

    /// Returns the [`ComponentId`] of the given [`Component`] type `T`.
    ///
    /// The returned `ComponentId` is specific to the `World` instance
    /// it was retrieved from and should not be used with another `World` instance.
    ///
    /// Returns [`None`] if the `Component` type has not yet been initialized within
    /// the `World` using [`World::register_component`].
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// let mut world = World::new();
    ///
    /// #[derive(Component)]
    /// struct ComponentA;
    ///
    /// let component_a_id = world.register_component::<ComponentA>();
    ///
    /// assert_eq!(component_a_id, world.component_id::<ComponentA>().unwrap())
    /// ```
    ///
    /// # See also
    ///
    /// * [`Components::component_id()`]
    /// * [`Components::get_id()`]
    #[inline]
    pub fn component_id<T: Component>(&self) -> Option<ComponentId> {
        self.components.component_id::<T>()
    }

    /// Registers a new [`Resource`] type and returns the [`ComponentId`] created for it.
    ///
    /// The [`Resource`] doesn't have a value in the [`World`], it's only registered. If you want
    /// to insert the [`Resource`] in the [`World`], use [`World::init_resource`] or
    /// [`World::insert_resource`] instead.
    pub fn register_resource<R: Resource>(&mut self) -> ComponentId {
        self.components.register_resource::<R>()
    }

    /// Returns the [`ComponentId`] of the given [`Resource`] type `T`.
    ///
    /// The returned [`ComponentId`] is specific to the [`World`] instance it was retrieved from
    /// and should not be used with another [`World`] instance.
    ///
    /// Returns [`None`] if the [`Resource`] type has not yet been initialized within the
    /// [`World`] using [`World::register_resource`], [`World::init_resource`] or [`World::insert_resource`].
    pub fn resource_id<T: Resource>(&self) -> Option<ComponentId> {
        self.components.get_resource_id(TypeId::of::<T>())
    }

    /// Returns [`EntityRef`]s that expose read-only operations for the given
    /// `entities`. This will panic if any of the given entities do not exist. Use
    /// [`World::get_entity`] if you want to check for entity existence instead
    /// of implicitly panicking.
    ///
    /// This function supports fetching a single entity or multiple entities:
    /// - Pass an [`Entity`] to receive a single [`EntityRef`].
    /// - Pass a slice of [`Entity`]s to receive a [`Vec<EntityRef>`].
    /// - Pass an array of [`Entity`]s to receive an equally-sized array of [`EntityRef`]s.
    /// - Pass a reference to a [`EntityHashSet`] to receive an
    ///   [`EntityHashMap<EntityRef>`](crate::entity::EntityHashMap).
    ///
    /// # Panics
    ///
    /// If any of the given `entities` do not exist in the world.
    ///
    /// # Examples
    ///
    /// ## Single [`Entity`]
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component)]
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn(Position { x: 0.0, y: 0.0 }).id();
    ///
    /// let position = world.entity(entity).get::<Position>().unwrap();
    /// assert_eq!(position.x, 0.0);
    /// ```
    ///
    /// ## Array of [`Entity`]s
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component)]
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// let mut world = World::new();
    /// let e1 = world.spawn(Position { x: 0.0, y: 0.0 }).id();
    /// let e2 = world.spawn(Position { x: 1.0, y: 1.0 }).id();
    ///
    /// let [e1_ref, e2_ref] = world.entity([e1, e2]);
    /// let e1_position = e1_ref.get::<Position>().unwrap();
    /// assert_eq!(e1_position.x, 0.0);
    /// let e2_position = e2_ref.get::<Position>().unwrap();
    /// assert_eq!(e2_position.x, 1.0);
    /// ```
    ///
    /// ## Slice of [`Entity`]s
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component)]
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// let mut world = World::new();
    /// let e1 = world.spawn(Position { x: 0.0, y: 1.0 }).id();
    /// let e2 = world.spawn(Position { x: 0.0, y: 1.0 }).id();
    /// let e3 = world.spawn(Position { x: 0.0, y: 1.0 }).id();
    ///
    /// let ids = vec![e1, e2, e3];
    /// for eref in world.entity(&ids[..]) {
    ///     assert_eq!(eref.get::<Position>().unwrap().y, 1.0);
    /// }
    /// ```
    ///
    /// ## [`EntityHashSet`]
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, entity::EntityHashSet};
    /// #[derive(Component)]
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// let mut world = World::new();
    /// let e1 = world.spawn(Position { x: 0.0, y: 1.0 }).id();
    /// let e2 = world.spawn(Position { x: 0.0, y: 1.0 }).id();
    /// let e3 = world.spawn(Position { x: 0.0, y: 1.0 }).id();
    ///
    /// let ids = EntityHashSet::from_iter([e1, e2, e3]);
    /// for (_id, eref) in world.entity(&ids) {
    ///     assert_eq!(eref.get::<Position>().unwrap().y, 1.0);
    /// }
    /// ```
    #[inline]
    #[track_caller]
    pub fn entity<F: WorldEntityFetch>(&self, entities: F) -> F::Ref<'_> {
        #[inline(never)]
        #[cold]
        #[track_caller]
        fn panic_no_entity(entity: Entity) -> ! {
            panic!("Entity {entity:?} does not exist");
        }

        match self.get_entity(entities) {
            Ok(fetched) => fetched,
            Err(entity) => panic_no_entity(entity),
        }
    }

    /// Returns [`EntityMut`]s that expose read and write operations for the
    /// given `entities`. This will panic if any of the given entities do not
    /// exist. Use [`World::get_entity_mut`] if you want to check for entity
    /// existence instead of implicitly panicking.
    ///
    /// This function supports fetching a single entity or multiple entities:
    /// - Pass an [`Entity`] to receive a single [`EntityWorldMut`].
    ///    - This reference type allows for structural changes to the entity,
    ///      such as adding or removing components, or despawning the entity.
    /// - Pass a slice of [`Entity`]s to receive a [`Vec<EntityMut>`].
    /// - Pass an array of [`Entity`]s to receive an equally-sized array of [`EntityMut`]s.
    /// - Pass a reference to a [`EntityHashSet`] to receive an
    ///   [`EntityHashMap<EntityMut>`](crate::entity::EntityHashMap).
    ///
    /// In order to perform structural changes on the returned entity reference,
    /// such as adding or removing components, or despawning the entity, only a
    /// single [`Entity`] can be passed to this function. Allowing multiple
    /// entities at the same time with structural access would lead to undefined
    /// behavior, so [`EntityMut`] is returned when requesting multiple entities.
    ///
    /// # Panics
    ///
    /// If any of the given `entities` do not exist in the world.
    ///
    /// # Examples
    ///
    /// ## Single [`Entity`]
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component)]
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn(Position { x: 0.0, y: 0.0 }).id();
    ///
    /// let mut entity_mut = world.entity_mut(entity);
    /// let mut position = entity_mut.get_mut::<Position>().unwrap();
    /// position.y = 1.0;
    /// assert_eq!(position.x, 0.0);
    /// entity_mut.despawn();
    /// # assert!(world.get_entity_mut(entity).is_err());
    /// ```
    ///
    /// ## Array of [`Entity`]s
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component)]
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// let mut world = World::new();
    /// let e1 = world.spawn(Position { x: 0.0, y: 0.0 }).id();
    /// let e2 = world.spawn(Position { x: 1.0, y: 1.0 }).id();
    ///
    /// let [mut e1_ref, mut e2_ref] = world.entity_mut([e1, e2]);
    /// let mut e1_position = e1_ref.get_mut::<Position>().unwrap();
    /// e1_position.x = 1.0;
    /// assert_eq!(e1_position.x, 1.0);
    /// let mut e2_position = e2_ref.get_mut::<Position>().unwrap();
    /// e2_position.x = 2.0;
    /// assert_eq!(e2_position.x, 2.0);
    /// ```
    ///
    /// ## Slice of [`Entity`]s
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component)]
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// let mut world = World::new();
    /// let e1 = world.spawn(Position { x: 0.0, y: 1.0 }).id();
    /// let e2 = world.spawn(Position { x: 0.0, y: 1.0 }).id();
    /// let e3 = world.spawn(Position { x: 0.0, y: 1.0 }).id();
    ///
    /// let ids = vec![e1, e2, e3];
    /// for mut eref in world.entity_mut(&ids[..]) {
    ///     let mut pos = eref.get_mut::<Position>().unwrap();
    ///     pos.y = 2.0;
    ///     assert_eq!(pos.y, 2.0);
    /// }
    /// ```
    ///
    /// ## [`EntityHashSet`]
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, entity::EntityHashSet};
    /// #[derive(Component)]
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// let mut world = World::new();
    /// let e1 = world.spawn(Position { x: 0.0, y: 1.0 }).id();
    /// let e2 = world.spawn(Position { x: 0.0, y: 1.0 }).id();
    /// let e3 = world.spawn(Position { x: 0.0, y: 1.0 }).id();
    ///
    /// let ids = EntityHashSet::from_iter([e1, e2, e3]);
    /// for (_id, mut eref) in world.entity_mut(&ids) {
    ///     let mut pos = eref.get_mut::<Position>().unwrap();
    ///     pos.y = 2.0;
    ///     assert_eq!(pos.y, 2.0);
    /// }
    /// ```
    #[inline]
    #[track_caller]
    pub fn entity_mut<F: WorldEntityFetch>(&mut self, entities: F) -> F::Mut<'_> {
        #[inline(never)]
        #[cold]
        #[track_caller]
        fn panic_on_err(e: EntityFetchError) -> ! {
            panic!("{e}");
        }

        match self.get_entity_mut(entities) {
            Ok(fetched) => fetched,
            Err(e) => panic_on_err(e),
        }
    }

    /// Gets an [`EntityRef`] for multiple entities at once.
    ///
    /// # Panics
    ///
    /// If any entity does not exist in the world.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # let mut world = World::new();
    /// # let id1 = world.spawn_empty().id();
    /// # let id2 = world.spawn_empty().id();
    /// // Getting multiple entities.
    /// let [entity1, entity2] = world.many_entities([id1, id2]);
    /// ```
    ///
    /// ```should_panic
    /// # use bevy_ecs::prelude::*;
    /// # let mut world = World::new();
    /// # let id1 = world.spawn_empty().id();
    /// # let id2 = world.spawn_empty().id();
    /// // Trying to get a despawned entity will fail.
    /// world.despawn(id2);
    /// world.many_entities([id1, id2]);
    /// ```
    #[deprecated(since = "0.15.0", note = "Use `World::entity::<[Entity; N]>` instead")]
    pub fn many_entities<const N: usize>(&mut self, entities: [Entity; N]) -> [EntityRef<'_>; N] {
        self.entity(entities)
    }

    /// Gets mutable access to multiple entities at once.
    ///
    /// # Panics
    ///
    /// If any entities do not exist in the world,
    /// or if the same entity is specified multiple times.
    ///
    /// # Examples
    ///
    /// Disjoint mutable access.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # let mut world = World::new();
    /// # let id1 = world.spawn_empty().id();
    /// # let id2 = world.spawn_empty().id();
    /// // Disjoint mutable access.
    /// let [entity1, entity2] = world.many_entities_mut([id1, id2]);
    /// ```
    ///
    /// Trying to access the same entity multiple times will fail.
    ///
    /// ```should_panic
    /// # use bevy_ecs::prelude::*;
    /// # let mut world = World::new();
    /// # let id = world.spawn_empty().id();
    /// world.many_entities_mut([id, id]);
    /// ```
    #[deprecated(
        since = "0.15.0",
        note = "Use `World::entity_mut::<[Entity; N]>` instead"
    )]
    pub fn many_entities_mut<const N: usize>(
        &mut self,
        entities: [Entity; N],
    ) -> [EntityMut<'_>; N] {
        self.entity_mut(entities)
    }

    /// Returns the components of an [`Entity`] through [`ComponentInfo`].
    #[inline]
    pub fn inspect_entity(&self, entity: Entity) -> impl Iterator<Item = &ComponentInfo> {
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
    }

    /// Returns an [`EntityWorldMut`] for the given `entity` (if it exists) or spawns one if it doesn't exist.
    /// This will return [`None`] if the `entity` exists with a different generation.
    ///
    /// # Note
    /// Spawning a specific `entity` value is rarely the right choice. Most apps should favor [`World::spawn`].
    /// This method should generally only be used for sharing entities across apps, and only when they have a
    /// scheme worked out to share an ID space (which doesn't happen by default).
    #[inline]
    #[deprecated(since = "0.15.0", note = "use `World::spawn` instead")]
    pub fn get_or_spawn(&mut self, entity: Entity) -> Option<EntityWorldMut> {
        self.flush();
        match self.entities.alloc_at_without_replacement(entity) {
            AllocAtWithoutReplacement::Exists(location) => {
                // SAFETY: `entity` exists and `location` is that entity's location
                Some(unsafe { EntityWorldMut::new(self, entity, location) })
            }
            AllocAtWithoutReplacement::DidNotExist => {
                // SAFETY: entity was just allocated
                Some(unsafe { self.spawn_at_empty_internal(entity) })
            }
            AllocAtWithoutReplacement::ExistsWithWrongGeneration => None,
        }
    }

    /// Returns [`EntityRef`]s that expose read-only operations for the given
    /// `entities`, returning [`Err`] if any of the given entities do not exist.
    /// Instead of immediately unwrapping the value returned from this function,
    /// prefer [`World::entity`].
    ///
    /// This function supports fetching a single entity or multiple entities:
    /// - Pass an [`Entity`] to receive a single [`EntityRef`].
    /// - Pass a slice of [`Entity`]s to receive a [`Vec<EntityRef>`].
    /// - Pass an array of [`Entity`]s to receive an equally-sized array of [`EntityRef`]s.
    /// - Pass a reference to a [`EntityHashSet`] to receive an
    ///   [`EntityHashMap<EntityRef>`](crate::entity::EntityHashMap).
    ///
    /// # Errors
    ///
    /// If any of the given `entities` do not exist in the world, the first
    /// [`Entity`] found to be missing will be returned in the [`Err`].
    ///
    /// # Examples
    ///
    /// For examples, see [`World::entity`].
    #[inline]
    pub fn get_entity<F: WorldEntityFetch>(&self, entities: F) -> Result<F::Ref<'_>, Entity> {
        let cell = self.as_unsafe_world_cell_readonly();
        // SAFETY: `&self` gives read access to the entire world, and prevents mutable access.
        unsafe { entities.fetch_ref(cell) }
    }

    /// Gets an [`EntityRef`] for multiple entities at once.
    ///
    /// # Errors
    ///
    /// If any entity does not exist in the world.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # let mut world = World::new();
    /// # let id1 = world.spawn_empty().id();
    /// # let id2 = world.spawn_empty().id();
    /// // Getting multiple entities.
    /// let [entity1, entity2] = world.get_many_entities([id1, id2]).unwrap();
    ///
    /// // Trying to get a despawned entity will fail.
    /// world.despawn(id2);
    /// assert!(world.get_many_entities([id1, id2]).is_err());
    /// ```
    #[deprecated(
        since = "0.15.0",
        note = "Use `World::get_entity::<[Entity; N]>` instead"
    )]
    pub fn get_many_entities<const N: usize>(
        &self,
        entities: [Entity; N],
    ) -> Result<[EntityRef<'_>; N], Entity> {
        self.get_entity(entities)
    }

    /// Gets an [`EntityRef`] for multiple entities at once, whose number is determined at runtime.
    ///
    /// # Errors
    ///
    /// If any entity does not exist in the world.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # let mut world = World::new();
    /// # let id1 = world.spawn_empty().id();
    /// # let id2 = world.spawn_empty().id();
    /// // Getting multiple entities.
    /// let entities = world.get_many_entities_dynamic(&[id1, id2]).unwrap();
    /// let entity1 = entities.get(0).unwrap();
    /// let entity2 = entities.get(1).unwrap();
    ///
    /// // Trying to get a despawned entity will fail.
    /// world.despawn(id2);
    /// assert!(world.get_many_entities_dynamic(&[id1, id2]).is_err());
    /// ```
    #[deprecated(
        since = "0.15.0",
        note = "Use `World::get_entity::<&[Entity]>` instead"
    )]
    pub fn get_many_entities_dynamic<'w>(
        &'w self,
        entities: &[Entity],
    ) -> Result<Vec<EntityRef<'w>>, Entity> {
        self.get_entity(entities)
    }

    /// Returns [`EntityMut`]s that expose read and write operations for the
    /// given `entities`, returning [`Err`] if any of the given entities do not
    /// exist. Instead of immediately unwrapping the value returned from this
    /// function, prefer [`World::entity_mut`].
    ///
    /// This function supports fetching a single entity or multiple entities:
    /// - Pass an [`Entity`] to receive a single [`EntityWorldMut`].
    ///    - This reference type allows for structural changes to the entity,
    ///      such as adding or removing components, or despawning the entity.
    /// - Pass a slice of [`Entity`]s to receive a [`Vec<EntityMut>`].
    /// - Pass an array of [`Entity`]s to receive an equally-sized array of [`EntityMut`]s.
    /// - Pass a reference to a [`EntityHashSet`] to receive an
    ///   [`EntityHashMap<EntityMut>`](crate::entity::EntityHashMap).
    ///
    /// In order to perform structural changes on the returned entity reference,
    /// such as adding or removing components, or despawning the entity, only a
    /// single [`Entity`] can be passed to this function. Allowing multiple
    /// entities at the same time with structural access would lead to undefined
    /// behavior, so [`EntityMut`] is returned when requesting multiple entities.
    ///
    /// # Errors
    ///
    /// - Returns [`EntityFetchError::NoSuchEntity`] if any of the given `entities` do not exist in the world.
    ///     - Only the first entity found to be missing will be returned.
    /// - Returns [`EntityFetchError::AliasedMutability`] if the same entity is requested multiple times.
    ///
    /// # Examples
    ///
    /// For examples, see [`World::entity_mut`].
    #[inline]
    pub fn get_entity_mut<F: WorldEntityFetch>(
        &mut self,
        entities: F,
    ) -> Result<F::Mut<'_>, EntityFetchError> {
        let cell = self.as_unsafe_world_cell();
        // SAFETY: `&mut self` gives mutable access to the entire world,
        // and prevents any other access to the world.
        unsafe { entities.fetch_mut(cell) }
    }

    /// Returns an [`Entity`] iterator of current entities.
    ///
    /// This is useful in contexts where you only have read-only access to the [`World`].
    #[inline]
    pub fn iter_entities(&self) -> impl Iterator<Item = EntityRef<'_>> + '_ {
        self.archetypes.iter().flat_map(|archetype| {
            archetype
                .entities()
                .iter()
                .enumerate()
                .map(|(archetype_row, archetype_entity)| {
                    let entity = archetype_entity.id();
                    let location = EntityLocation {
                        archetype_id: archetype.id(),
                        archetype_row: ArchetypeRow::new(archetype_row),
                        table_id: archetype.table_id(),
                        table_row: archetype_entity.table_row(),
                    };

                    // SAFETY: entity exists and location accurately specifies the archetype where the entity is stored.
                    let cell = UnsafeEntityCell::new(
                        self.as_unsafe_world_cell_readonly(),
                        entity,
                        location,
                    );
                    // SAFETY: `&self` gives read access to the entire world.
                    unsafe { EntityRef::new(cell) }
                })
        })
    }

    /// Returns a mutable iterator over all entities in the `World`.
    pub fn iter_entities_mut(&mut self) -> impl Iterator<Item = EntityMut<'_>> + '_ {
        let world_cell = self.as_unsafe_world_cell();
        world_cell.archetypes().iter().flat_map(move |archetype| {
            archetype
                .entities()
                .iter()
                .enumerate()
                .map(move |(archetype_row, archetype_entity)| {
                    let entity = archetype_entity.id();
                    let location = EntityLocation {
                        archetype_id: archetype.id(),
                        archetype_row: ArchetypeRow::new(archetype_row),
                        table_id: archetype.table_id(),
                        table_row: archetype_entity.table_row(),
                    };

                    // SAFETY: entity exists and location accurately specifies the archetype where the entity is stored.
                    let cell = UnsafeEntityCell::new(world_cell, entity, location);
                    // SAFETY: We have exclusive access to the entire world. We only create one borrow for each entity,
                    // so none will conflict with one another.
                    unsafe { EntityMut::new(cell) }
                })
        })
    }

    /// Gets mutable access to multiple entities.
    ///
    /// # Errors
    ///
    /// If any entities do not exist in the world,
    /// or if the same entity is specified multiple times.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # let mut world = World::new();
    /// # let id1 = world.spawn_empty().id();
    /// # let id2 = world.spawn_empty().id();
    /// // Disjoint mutable access.
    /// let [entity1, entity2] = world.get_many_entities_mut([id1, id2]).unwrap();
    ///
    /// // Trying to access the same entity multiple times will fail.
    /// assert!(world.get_many_entities_mut([id1, id1]).is_err());
    /// ```
    #[deprecated(
        since = "0.15.0",
        note = "Use `World::get_entity_mut::<[Entity; N]>` instead"
    )]
    pub fn get_many_entities_mut<const N: usize>(
        &mut self,
        entities: [Entity; N],
    ) -> Result<[EntityMut<'_>; N], QueryEntityError> {
        self.get_entity_mut(entities).map_err(|e| match e {
            EntityFetchError::NoSuchEntity(entity) => QueryEntityError::NoSuchEntity(entity),
            EntityFetchError::AliasedMutability(entity) => {
                QueryEntityError::AliasedMutability(entity)
            }
        })
    }

    /// Gets mutable access to multiple entities, whose number is determined at runtime.
    ///
    /// # Errors
    ///
    /// If any entities do not exist in the world,
    /// or if the same entity is specified multiple times.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # let mut world = World::new();
    /// # let id1 = world.spawn_empty().id();
    /// # let id2 = world.spawn_empty().id();
    /// // Disjoint mutable access.
    /// let mut entities = world.get_many_entities_dynamic_mut(&[id1, id2]).unwrap();
    /// let entity1 = entities.get_mut(0).unwrap();
    ///
    /// // Trying to access the same entity multiple times will fail.
    /// assert!(world.get_many_entities_dynamic_mut(&[id1, id1]).is_err());
    /// ```
    #[deprecated(
        since = "0.15.0",
        note = "Use `World::get_entity_mut::<&[Entity]>` instead"
    )]
    pub fn get_many_entities_dynamic_mut<'w>(
        &'w mut self,
        entities: &[Entity],
    ) -> Result<Vec<EntityMut<'w>>, QueryEntityError> {
        self.get_entity_mut(entities).map_err(|e| match e {
            EntityFetchError::NoSuchEntity(entity) => QueryEntityError::NoSuchEntity(entity),
            EntityFetchError::AliasedMutability(entity) => {
                QueryEntityError::AliasedMutability(entity)
            }
        })
    }

    /// Gets mutable access to multiple entities, contained in a [`EntityHashSet`].
    /// The uniqueness of items in a [`EntityHashSet`] allows us to avoid checking for duplicates.
    ///
    /// # Errors
    ///
    /// If any entities do not exist in the world.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_ecs::entity::EntityHash;
    /// # use bevy_ecs::entity::EntityHashSet;
    /// # use bevy_utils::hashbrown::HashSet;
    /// # use bevy_utils::hashbrown::hash_map::DefaultHashBuilder;
    /// # let mut world = World::new();
    /// # let id1 = world.spawn_empty().id();
    /// # let id2 = world.spawn_empty().id();
    /// let s = EntityHash::default();
    /// let mut set = EntityHashSet::with_hasher(s);
    /// set.insert(id1);
    /// set.insert(id2);
    ///
    /// // Disjoint mutable access.
    /// let mut entities = world.get_many_entities_from_set_mut(&set).unwrap();
    /// let entity1 = entities.get_mut(0).unwrap();
    /// ```
    #[deprecated(
        since = "0.15.0",
        note = "Use `World::get_entity_mut::<&EntityHashSet>` instead."
    )]
    pub fn get_many_entities_from_set_mut<'w>(
        &'w mut self,
        entities: &EntityHashSet,
    ) -> Result<Vec<EntityMut<'w>>, QueryEntityError> {
        self.get_entity_mut(entities)
            .map(|fetched| fetched.into_values().collect())
            .map_err(|e| match e {
                EntityFetchError::NoSuchEntity(entity) => QueryEntityError::NoSuchEntity(entity),
                EntityFetchError::AliasedMutability(entity) => {
                    QueryEntityError::AliasedMutability(entity)
                }
            })
    }

    /// Spawns a new [`Entity`] and returns a corresponding [`EntityWorldMut`], which can be used
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
    pub fn spawn_empty(&mut self) -> EntityWorldMut {
        self.flush();
        let entity = self.entities.alloc();
        // SAFETY: entity was just allocated
        unsafe { self.spawn_at_empty_internal(entity) }
    }

    /// Spawns a new [`Entity`] with a given [`Bundle`] of [components](`Component`) and returns
    /// a corresponding [`EntityWorldMut`], which can be used to add components to the entity or
    /// retrieve its id. In case large batches of entities need to be spawned, consider using
    /// [`World::spawn_batch`] instead.
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
    #[track_caller]
    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> EntityWorldMut {
        self.flush();
        let change_tick = self.change_tick();
        let entity = self.entities.alloc();
        let entity_location = {
            let mut bundle_spawner = BundleSpawner::new::<B>(self, change_tick);
            // SAFETY: bundle's type matches `bundle_info`, entity is allocated but non-existent
            unsafe {
                bundle_spawner.spawn_non_existent(
                    entity,
                    bundle,
                    #[cfg(feature = "track_change_detection")]
                    Location::caller(),
                )
            }
        };

        // SAFETY: entity and location are valid, as they were just created above
        unsafe { EntityWorldMut::new(self, entity, entity_location) }
    }

    /// # Safety
    /// must be called on an entity that was just allocated
    unsafe fn spawn_at_empty_internal(&mut self, entity: Entity) -> EntityWorldMut {
        let archetype = self.archetypes.empty_mut();
        // PERF: consider avoiding allocating entities in the empty archetype unless needed
        let table_row = self.storages.tables[archetype.table_id()].allocate(entity);
        // SAFETY: no components are allocated by archetype.allocate() because the archetype is
        // empty
        let location = unsafe { archetype.allocate(entity, table_row) };
        // SAFETY: entity index was just allocated
        unsafe {
            self.entities.set(entity.index(), location);
        }
        EntityWorldMut::new(self, entity, location)
    }

    /// Spawns a batch of entities with the same component [`Bundle`] type. Takes a given
    /// [`Bundle`] iterator and returns a corresponding [`Entity`] iterator.
    /// This is more efficient than spawning entities and adding components to them individually
    /// using [`World::spawn`], but it is limited to spawning entities with the same [`Bundle`]
    /// type, whereas spawning individually is more flexible.
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
    #[track_caller]
    pub fn spawn_batch<I>(&mut self, iter: I) -> SpawnBatchIter<'_, I::IntoIter>
    where
        I: IntoIterator,
        I::Item: Bundle,
    {
        SpawnBatchIter::new(
            self,
            iter.into_iter(),
            #[cfg(feature = "track_change_detection")]
            Location::caller(),
        )
    }

    /// Retrieves a reference to the given `entity`'s [`Component`] of the given type.
    /// Returns `None` if the `entity` does not have a [`Component`] of the given type.
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
        self.get_entity(entity).ok()?.get()
    }

    /// Retrieves a mutable reference to the given `entity`'s [`Component`] of the given type.
    /// Returns `None` if the `entity` does not have a [`Component`] of the given type.
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
        // SAFETY:
        // - `as_unsafe_world_cell` is the only thing that is borrowing world
        // - `as_unsafe_world_cell` provides mutable permission to everything
        // - `&mut self` ensures no other borrows on world data
        unsafe { self.as_unsafe_world_cell().get_entity(entity)?.get_mut() }
    }

    /// Despawns the given `entity`, if it exists. This will also remove all of the entity's
    /// [`Component`]s. Returns `true` if the `entity` is successfully despawned and `false` if
    /// the `entity` does not exist.
    ///
    /// # Note
    ///
    /// This won't clean up external references to the entity (such as parent-child relationships
    /// if you're using `bevy_hierarchy`), which may leave the world in an invalid state.
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
    /// assert!(world.despawn(entity));
    /// assert!(world.get_entity(entity).is_err());
    /// assert!(world.get::<Position>(entity).is_none());
    /// ```
    #[track_caller]
    #[inline]
    pub fn despawn(&mut self, entity: Entity) -> bool {
        self.despawn_with_caller(entity, Location::caller(), true)
    }

    /// Performs the same function as [`Self::despawn`] but does not emit a warning if
    /// the entity does not exist.
    #[track_caller]
    #[inline]
    pub fn try_despawn(&mut self, entity: Entity) -> bool {
        self.despawn_with_caller(entity, Location::caller(), false)
    }

    #[inline]
    pub(crate) fn despawn_with_caller(
        &mut self,
        entity: Entity,
        caller: &'static Location,
        log_warning: bool,
    ) -> bool {
        self.flush();
        if let Ok(entity) = self.get_entity_mut(entity) {
            entity.despawn();
            true
        } else {
            if log_warning {
                warn!("error[B0003]: {caller}: Could not despawn entity {:?} because it doesn't exist in this World. See: https://bevyengine.org/learn/errors/b0003", entity);
            }
            false
        }
    }

    /// Clears the internal component tracker state.
    ///
    /// The world maintains some internal state about changed and removed components. This state
    /// is used by [`RemovedComponents`] to provide access to the entities that had a specific type
    /// of component removed since last tick.
    ///
    /// The state is also used for change detection when accessing components and resources outside
    /// of a system, for example via [`World::get_mut()`] or [`World::get_resource_mut()`].
    ///
    /// By clearing this internal state, the world "forgets" about those changes, allowing a new round
    /// of detection to be recorded.
    ///
    /// When using `bevy_ecs` as part of the full Bevy engine, this method is called automatically
    /// by `bevy_app::App::update` and `bevy_app::SubApp::update`, so you don't need to call it manually.
    /// When using `bevy_ecs` as a separate standalone crate however, you do need to call this manually.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Component, Default)]
    /// # struct Transform;
    /// // a whole new world
    /// let mut world = World::new();
    ///
    /// // you changed it
    /// let entity = world.spawn(Transform::default()).id();
    ///
    /// // change is detected
    /// let transform = world.get_mut::<Transform>(entity).unwrap();
    /// assert!(transform.is_changed());
    ///
    /// // update the last change tick
    /// world.clear_trackers();
    ///
    /// // change is no longer detected
    /// let transform = world.get_mut::<Transform>(entity).unwrap();
    /// assert!(!transform.is_changed());
    /// ```
    ///
    /// [`RemovedComponents`]: crate::removal_detection::RemovedComponents
    pub fn clear_trackers(&mut self) {
        self.removed_components.update();
        self.last_change_tick = self.increment_change_tick();
    }

    /// Returns [`QueryState`] for the given [`QueryData`], which is used to efficiently
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
    /// and allocation of a [`Vec`] to store it.
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
    pub fn query<D: QueryData>(&mut self) -> QueryState<D, ()> {
        self.query_filtered::<D, ()>()
    }

    /// Returns [`QueryState`] for the given filtered [`QueryData`], which is used to efficiently
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
    pub fn query_filtered<D: QueryData, F: QueryFilter>(&mut self) -> QueryState<D, F> {
        QueryState::new(self)
    }

    /// Returns an iterator of entities that had components of type `T` removed
    /// since the last call to [`World::clear_trackers`].
    pub fn removed<T: Component>(&self) -> impl Iterator<Item = Entity> + '_ {
        self.components
            .get_id(TypeId::of::<T>())
            .map(|component_id| self.removed_with_id(component_id))
            .into_iter()
            .flatten()
    }

    /// Returns an iterator of entities that had components with the given `component_id` removed
    /// since the last call to [`World::clear_trackers`].
    pub fn removed_with_id(&self, component_id: ComponentId) -> impl Iterator<Item = Entity> + '_ {
        self.removed_components
            .get(component_id)
            .map(|removed| removed.iter_current_update_events().cloned())
            .into_iter()
            .flatten()
            .map(Into::into)
    }

    /// Registers a new [`Resource`] type and returns the [`ComponentId`] created for it.
    ///
    /// This enables the dynamic registration of new [`Resource`] definitions at runtime for
    /// advanced use cases.
    ///
    /// # Note
    ///
    /// Registering a [`Resource`] does not insert it into [`World`]. For insertion, you could use
    /// [`World::insert_resource_by_id`].
    pub fn register_resource_with_descriptor(
        &mut self,
        descriptor: ComponentDescriptor,
    ) -> ComponentId {
        self.components
            .register_resource_with_descriptor(descriptor)
    }

    /// Initializes a new resource and returns the [`ComponentId`] created for it.
    ///
    /// If the resource already exists, nothing happens.
    ///
    /// The value given by the [`FromWorld::from_world`] method will be used.
    /// Note that any resource with the [`Default`] trait automatically implements [`FromWorld`],
    /// and those default values will be here instead.
    #[inline]
    #[track_caller]
    pub fn init_resource<R: Resource + FromWorld>(&mut self) -> ComponentId {
        #[cfg(feature = "track_change_detection")]
        let caller = Location::caller();
        let component_id = self.components.register_resource::<R>();
        if self
            .storages
            .resources
            .get(component_id)
            .map_or(true, |data| !data.is_present())
        {
            let value = R::from_world(self);
            OwningPtr::make(value, |ptr| {
                // SAFETY: component_id was just initialized and corresponds to resource of type R.
                unsafe {
                    self.insert_resource_by_id(
                        component_id,
                        ptr,
                        #[cfg(feature = "track_change_detection")]
                        caller,
                    );
                }
            });
        }
        component_id
    }

    /// Inserts a new resource with the given `value`.
    ///
    /// Resources are "unique" data of a given type.
    /// If you insert a resource of a type that already exists,
    /// you will overwrite any existing data.
    #[inline]
    #[track_caller]
    pub fn insert_resource<R: Resource>(&mut self, value: R) {
        self.insert_resource_with_caller(
            value,
            #[cfg(feature = "track_change_detection")]
            Location::caller(),
        );
    }

    /// Split into a new function so we can pass the calling location into the function when using
    /// as a command.
    #[inline]
    pub(crate) fn insert_resource_with_caller<R: Resource>(
        &mut self,
        value: R,
        #[cfg(feature = "track_change_detection")] caller: &'static Location,
    ) {
        let component_id = self.components.register_resource::<R>();
        OwningPtr::make(value, |ptr| {
            // SAFETY: component_id was just initialized and corresponds to resource of type R.
            unsafe {
                self.insert_resource_by_id(
                    component_id,
                    ptr,
                    #[cfg(feature = "track_change_detection")]
                    caller,
                );
            }
        });
    }

    /// Initializes a new non-send resource and returns the [`ComponentId`] created for it.
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
    #[track_caller]
    pub fn init_non_send_resource<R: 'static + FromWorld>(&mut self) -> ComponentId {
        #[cfg(feature = "track_change_detection")]
        let caller = Location::caller();
        let component_id = self.components.register_non_send::<R>();
        if self
            .storages
            .non_send_resources
            .get(component_id)
            .map_or(true, |data| !data.is_present())
        {
            let value = R::from_world(self);
            OwningPtr::make(value, |ptr| {
                // SAFETY: component_id was just initialized and corresponds to resource of type R.
                unsafe {
                    self.insert_non_send_by_id(
                        component_id,
                        ptr,
                        #[cfg(feature = "track_change_detection")]
                        caller,
                    );
                }
            });
        }
        component_id
    }

    /// Inserts a new non-send resource with the given `value`.
    ///
    /// `NonSend` resources cannot be sent across threads,
    /// and do not need the `Send + Sync` bounds.
    /// Systems with `NonSend` resources are always scheduled on the main thread.
    ///
    /// # Panics
    /// If a value is already present, this function will panic if called
    /// from a different thread than where the original value was inserted from.
    #[inline]
    #[track_caller]
    pub fn insert_non_send_resource<R: 'static>(&mut self, value: R) {
        #[cfg(feature = "track_change_detection")]
        let caller = Location::caller();
        let component_id = self.components.register_non_send::<R>();
        OwningPtr::make(value, |ptr| {
            // SAFETY: component_id was just initialized and corresponds to resource of type R.
            unsafe {
                self.insert_non_send_by_id(
                    component_id,
                    ptr,
                    #[cfg(feature = "track_change_detection")]
                    caller,
                );
            }
        });
    }

    /// Removes the resource of a given type and returns it, if it exists. Otherwise returns `None`.
    #[inline]
    pub fn remove_resource<R: Resource>(&mut self) -> Option<R> {
        let component_id = self.components.get_resource_id(TypeId::of::<R>())?;
        let (ptr, _, _) = self.storages.resources.get_mut(component_id)?.remove()?;
        // SAFETY: `component_id` was gotten via looking up the `R` type
        unsafe { Some(ptr.read::<R>()) }
    }

    /// Removes a `!Send` resource from the world and returns it, if present.
    ///
    /// `NonSend` resources cannot be sent across threads,
    /// and do not need the `Send + Sync` bounds.
    /// Systems with `NonSend` resources are always scheduled on the main thread.
    ///
    /// Returns `None` if a value was not previously present.
    ///
    /// # Panics
    /// If a value is present, this function will panic if called from a different
    /// thread than where the value was inserted from.
    #[inline]
    pub fn remove_non_send_resource<R: 'static>(&mut self) -> Option<R> {
        let component_id = self.components.get_resource_id(TypeId::of::<R>())?;
        let (ptr, _, _) = self
            .storages
            .non_send_resources
            .get_mut(component_id)?
            .remove()?;
        // SAFETY: `component_id` was gotten via looking up the `R` type
        unsafe { Some(ptr.read::<R>()) }
    }

    /// Returns `true` if a resource of type `R` exists. Otherwise returns `false`.
    #[inline]
    pub fn contains_resource<R: Resource>(&self) -> bool {
        self.components
            .get_resource_id(TypeId::of::<R>())
            .and_then(|component_id| self.storages.resources.get(component_id))
            .is_some_and(ResourceData::is_present)
    }

    /// Returns `true` if a resource with provided `component_id` exists. Otherwise returns `false`.
    #[inline]
    pub fn contains_resource_by_id(&self, component_id: ComponentId) -> bool {
        self.storages
            .resources
            .get(component_id)
            .is_some_and(ResourceData::is_present)
    }

    /// Returns `true` if a resource of type `R` exists. Otherwise returns `false`.
    #[inline]
    pub fn contains_non_send<R: 'static>(&self) -> bool {
        self.components
            .get_resource_id(TypeId::of::<R>())
            .and_then(|component_id| self.storages.non_send_resources.get(component_id))
            .is_some_and(ResourceData::is_present)
    }

    /// Returns `true` if a resource with provided `component_id` exists. Otherwise returns `false`.
    #[inline]
    pub fn contains_non_send_by_id(&self, component_id: ComponentId) -> bool {
        self.storages
            .non_send_resources
            .get(component_id)
            .is_some_and(ResourceData::is_present)
    }

    /// Returns `true` if a resource of type `R` exists and was added since the world's
    /// [`last_change_tick`](World::last_change_tick()). Otherwise, this returns `false`.
    ///
    /// This means that:
    /// - When called from an exclusive system, this will check for additions since the system last ran.
    /// - When called elsewhere, this will check for additions since the last time that [`World::clear_trackers`]
    ///   was called.
    pub fn is_resource_added<R: Resource>(&self) -> bool {
        self.components
            .get_resource_id(TypeId::of::<R>())
            .is_some_and(|component_id| self.is_resource_added_by_id(component_id))
    }

    /// Returns `true` if a resource with id `component_id` exists and was added since the world's
    /// [`last_change_tick`](World::last_change_tick()). Otherwise, this returns `false`.
    ///
    /// This means that:
    /// - When called from an exclusive system, this will check for additions since the system last ran.
    /// - When called elsewhere, this will check for additions since the last time that [`World::clear_trackers`]
    ///   was called.
    pub fn is_resource_added_by_id(&self, component_id: ComponentId) -> bool {
        self.storages
            .resources
            .get(component_id)
            .and_then(|resource| {
                resource
                    .get_ticks()
                    .map(|ticks| ticks.is_added(self.last_change_tick(), self.read_change_tick()))
            })
            .unwrap_or(false)
    }

    /// Returns `true` if a resource of type `R` exists and was modified since the world's
    /// [`last_change_tick`](World::last_change_tick()). Otherwise, this returns `false`.
    ///
    /// This means that:
    /// - When called from an exclusive system, this will check for changes since the system last ran.
    /// - When called elsewhere, this will check for changes since the last time that [`World::clear_trackers`]
    ///   was called.
    pub fn is_resource_changed<R: Resource>(&self) -> bool {
        self.components
            .get_resource_id(TypeId::of::<R>())
            .map(|component_id| self.is_resource_changed_by_id(component_id))
            .unwrap_or(false)
    }

    /// Returns `true` if a resource with id `component_id` exists and was modified since the world's
    /// [`last_change_tick`](World::last_change_tick()). Otherwise, this returns `false`.
    ///
    /// This means that:
    /// - When called from an exclusive system, this will check for changes since the system last ran.
    /// - When called elsewhere, this will check for changes since the last time that [`World::clear_trackers`]
    ///   was called.
    pub fn is_resource_changed_by_id(&self, component_id: ComponentId) -> bool {
        self.storages
            .resources
            .get(component_id)
            .and_then(|resource| {
                resource
                    .get_ticks()
                    .map(|ticks| ticks.is_changed(self.last_change_tick(), self.read_change_tick()))
            })
            .unwrap_or(false)
    }

    /// Retrieves the change ticks for the given resource.
    pub fn get_resource_change_ticks<R: Resource>(&self) -> Option<ComponentTicks> {
        self.components
            .get_resource_id(TypeId::of::<R>())
            .and_then(|component_id| self.get_resource_change_ticks_by_id(component_id))
    }

    /// Retrieves the change ticks for the given [`ComponentId`].
    ///
    /// **You should prefer to use the typed API [`World::get_resource_change_ticks`] where possible.**
    pub fn get_resource_change_ticks_by_id(
        &self,
        component_id: ComponentId,
    ) -> Option<ComponentTicks> {
        self.storages
            .resources
            .get(component_id)
            .and_then(ResourceData::get_ticks)
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
                core::any::type_name::<R>()
            ),
        }
    }

    /// Gets a reference to the resource of the given type
    ///
    /// # Panics
    ///
    /// Panics if the resource does not exist.
    /// Use [`get_resource_ref`](World::get_resource_ref) instead if you want to handle this case.
    ///
    /// If you want to instead insert a value if the resource does not exist,
    /// use [`get_resource_or_insert_with`](World::get_resource_or_insert_with).
    #[inline]
    #[track_caller]
    pub fn resource_ref<R: Resource>(&self) -> Ref<R> {
        match self.get_resource_ref() {
            Some(x) => x,
            None => panic!(
                "Requested resource {} does not exist in the `World`.
                Did you forget to add it using `app.insert_resource` / `app.init_resource`?
                Resources are also implicitly added via `app.add_event`,
                and can be added by plugins.",
                core::any::type_name::<R>()
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
                core::any::type_name::<R>()
            ),
        }
    }

    /// Gets a reference to the resource of the given type if it exists
    #[inline]
    pub fn get_resource<R: Resource>(&self) -> Option<&R> {
        // SAFETY:
        // - `as_unsafe_world_cell_readonly` gives permission to access everything immutably
        // - `&self` ensures nothing in world is borrowed mutably
        unsafe { self.as_unsafe_world_cell_readonly().get_resource() }
    }

    /// Gets a reference including change detection to the resource of the given type if it exists.
    #[inline]
    pub fn get_resource_ref<R: Resource>(&self) -> Option<Ref<R>> {
        // SAFETY:
        // - `as_unsafe_world_cell_readonly` gives permission to access everything immutably
        // - `&self` ensures nothing in world is borrowed mutably
        unsafe { self.as_unsafe_world_cell_readonly().get_resource_ref() }
    }

    /// Gets a mutable reference to the resource of the given type if it exists
    #[inline]
    pub fn get_resource_mut<R: Resource>(&mut self) -> Option<Mut<'_, R>> {
        // SAFETY:
        // - `as_unsafe_world_cell` gives permission to access everything mutably
        // - `&mut self` ensures nothing in world is borrowed
        unsafe { self.as_unsafe_world_cell().get_resource_mut() }
    }

    /// Gets a mutable reference to the resource of type `T` if it exists,
    /// otherwise inserts the resource using the result of calling `func`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// #[derive(Resource)]
    /// struct MyResource(i32);
    ///
    /// # let mut world = World::new();
    /// let my_res = world.get_resource_or_insert_with(|| MyResource(10));
    /// assert_eq!(my_res.0, 10);
    /// ```
    #[inline]
    #[track_caller]
    pub fn get_resource_or_insert_with<R: Resource>(
        &mut self,
        func: impl FnOnce() -> R,
    ) -> Mut<'_, R> {
        #[cfg(feature = "track_change_detection")]
        let caller = Location::caller();
        let change_tick = self.change_tick();
        let last_change_tick = self.last_change_tick();

        let component_id = self.components.register_resource::<R>();
        let data = self.initialize_resource_internal(component_id);
        if !data.is_present() {
            OwningPtr::make(func(), |ptr| {
                // SAFETY: component_id was just initialized and corresponds to resource of type R.
                unsafe {
                    data.insert(
                        ptr,
                        change_tick,
                        #[cfg(feature = "track_change_detection")]
                        caller,
                    );
                }
            });
        }

        // SAFETY: The resource must be present, as we would have inserted it if it was empty.
        let data = unsafe {
            data.get_mut(last_change_tick, change_tick)
                .debug_checked_unwrap()
        };
        // SAFETY: The underlying type of the resource is `R`.
        unsafe { data.with_type::<R>() }
    }

    /// Gets a mutable reference to the resource of type `T` if it exists,
    /// otherwise initializes the resource by calling its [`FromWorld`]
    /// implementation.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// #[derive(Resource)]
    /// struct Foo(i32);
    ///
    /// impl Default for Foo {
    ///     fn default() -> Self {
    ///         Self(15)
    ///     }
    /// }
    ///
    /// #[derive(Resource)]
    /// struct MyResource(i32);
    ///
    /// impl FromWorld for MyResource {
    ///     fn from_world(world: &mut World) -> Self {
    ///         let foo = world.get_resource_or_init::<Foo>();
    ///         Self(foo.0 * 2)
    ///     }
    /// }
    ///
    /// # let mut world = World::new();
    /// let my_res = world.get_resource_or_init::<MyResource>();
    /// assert_eq!(my_res.0, 30);
    /// ```
    #[track_caller]
    pub fn get_resource_or_init<R: Resource + FromWorld>(&mut self) -> Mut<'_, R> {
        #[cfg(feature = "track_change_detection")]
        let caller = Location::caller();
        let change_tick = self.change_tick();
        let last_change_tick = self.last_change_tick();

        let component_id = self.components.register_resource::<R>();
        if self
            .storages
            .resources
            .get(component_id)
            .map_or(true, |data| !data.is_present())
        {
            let value = R::from_world(self);
            OwningPtr::make(value, |ptr| {
                // SAFETY: component_id was just initialized and corresponds to resource of type R.
                unsafe {
                    self.insert_resource_by_id(
                        component_id,
                        ptr,
                        #[cfg(feature = "track_change_detection")]
                        caller,
                    );
                }
            });
        }

        // SAFETY: The resource was just initialized if it was empty.
        let data = unsafe {
            self.storages
                .resources
                .get_mut(component_id)
                .debug_checked_unwrap()
        };
        // SAFETY: The resource must be present, as we would have inserted it if it was empty.
        let data = unsafe {
            data.get_mut(last_change_tick, change_tick)
                .debug_checked_unwrap()
        };
        // SAFETY: The underlying type of the resource is `R`.
        unsafe { data.with_type::<R>() }
    }

    /// Gets an immutable reference to the non-send resource of the given type, if it exists.
    ///
    /// # Panics
    ///
    /// Panics if the resource does not exist.
    /// Use [`get_non_send_resource`](World::get_non_send_resource) instead if you want to handle this case.
    ///
    /// This function will panic if it isn't called from the same thread that the resource was inserted from.
    #[inline]
    #[track_caller]
    pub fn non_send_resource<R: 'static>(&self) -> &R {
        match self.get_non_send_resource() {
            Some(x) => x,
            None => panic!(
                "Requested non-send resource {} does not exist in the `World`.
                Did you forget to add it using `app.insert_non_send_resource` / `app.init_non_send_resource`?
                Non-send resources can also be added by plugins.",
                core::any::type_name::<R>()
            ),
        }
    }

    /// Gets a mutable reference to the non-send resource of the given type, if it exists.
    ///
    /// # Panics
    ///
    /// Panics if the resource does not exist.
    /// Use [`get_non_send_resource_mut`](World::get_non_send_resource_mut) instead if you want to handle this case.
    ///
    /// This function will panic if it isn't called from the same thread that the resource was inserted from.
    #[inline]
    #[track_caller]
    pub fn non_send_resource_mut<R: 'static>(&mut self) -> Mut<'_, R> {
        match self.get_non_send_resource_mut() {
            Some(x) => x,
            None => panic!(
                "Requested non-send resource {} does not exist in the `World`.
                Did you forget to add it using `app.insert_non_send_resource` / `app.init_non_send_resource`?
                Non-send resources can also be added by plugins.",
                core::any::type_name::<R>()
            ),
        }
    }

    /// Gets a reference to the non-send resource of the given type, if it exists.
    /// Otherwise returns `None`.
    ///
    /// # Panics
    /// This function will panic if it isn't called from the same thread that the resource was inserted from.
    #[inline]
    pub fn get_non_send_resource<R: 'static>(&self) -> Option<&R> {
        // SAFETY:
        // - `as_unsafe_world_cell_readonly` gives permission to access the entire world immutably
        // - `&self` ensures that there are no mutable borrows of world data
        unsafe { self.as_unsafe_world_cell_readonly().get_non_send_resource() }
    }

    /// Gets a mutable reference to the non-send resource of the given type, if it exists.
    /// Otherwise returns `None`.
    ///
    /// # Panics
    /// This function will panic if it isn't called from the same thread that the resource was inserted from.
    #[inline]
    pub fn get_non_send_resource_mut<R: 'static>(&mut self) -> Option<Mut<'_, R>> {
        // SAFETY:
        // - `as_unsafe_world_cell` gives permission to access the entire world mutably
        // - `&mut self` ensures that there are no borrows of world data
        unsafe { self.as_unsafe_world_cell().get_non_send_resource_mut() }
    }

    /// For a given batch of ([`Entity`], [`Bundle`]) pairs, either spawns each [`Entity`] with the given
    /// bundle (if the entity does not exist), or inserts the [`Bundle`] (if the entity already exists).
    /// This is faster than doing equivalent operations one-by-one.
    /// Returns `Ok` if all entities were successfully inserted into or spawned. Otherwise it returns an `Err`
    /// with a list of entities that could not be spawned or inserted into. A "spawn or insert" operation can
    /// only fail if an [`Entity`] is passed in with an "invalid generation" that conflicts with an existing [`Entity`].
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
    #[track_caller]
    pub fn insert_or_spawn_batch<I, B>(&mut self, iter: I) -> Result<(), Vec<Entity>>
    where
        I: IntoIterator,
        I::IntoIter: Iterator<Item = (Entity, B)>,
        B: Bundle,
    {
        self.insert_or_spawn_batch_with_caller(
            iter,
            #[cfg(feature = "track_change_detection")]
            Location::caller(),
        )
    }

    /// Split into a new function so we can pass the calling location into the function when using
    /// as a command.
    #[inline]
    pub(crate) fn insert_or_spawn_batch_with_caller<I, B>(
        &mut self,
        iter: I,
        #[cfg(feature = "track_change_detection")] caller: &'static Location,
    ) -> Result<(), Vec<Entity>>
    where
        I: IntoIterator,
        I::IntoIter: Iterator<Item = (Entity, B)>,
        B: Bundle,
    {
        self.flush();

        let change_tick = self.change_tick();

        let bundle_id = self
            .bundles
            .register_info::<B>(&mut self.components, &mut self.storages);
        enum SpawnOrInsert<'w> {
            Spawn(BundleSpawner<'w>),
            Insert(BundleInserter<'w>, ArchetypeId),
        }

        impl<'w> SpawnOrInsert<'w> {
            fn entities(&mut self) -> &mut Entities {
                match self {
                    SpawnOrInsert::Spawn(spawner) => spawner.entities(),
                    SpawnOrInsert::Insert(inserter, _) => inserter.entities(),
                }
            }
        }
        // SAFETY: we initialized this bundle_id in `init_info`
        let mut spawn_or_insert = SpawnOrInsert::Spawn(unsafe {
            BundleSpawner::new_with_id(self, bundle_id, change_tick)
        });

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
                            unsafe {
                                inserter.insert(
                                    entity,
                                    location,
                                    bundle,
                                    InsertMode::Replace,
                                    #[cfg(feature = "track_change_detection")]
                                    caller,
                                )
                            };
                        }
                        _ => {
                            // SAFETY: we initialized this bundle_id in `init_info`
                            let mut inserter = unsafe {
                                BundleInserter::new_with_id(
                                    self,
                                    location.archetype_id,
                                    bundle_id,
                                    change_tick,
                                )
                            };
                            // SAFETY: `entity` is valid, `location` matches entity, bundle matches inserter
                            unsafe {
                                inserter.insert(
                                    entity,
                                    location,
                                    bundle,
                                    InsertMode::Replace,
                                    #[cfg(feature = "track_change_detection")]
                                    caller,
                                )
                            };
                            spawn_or_insert =
                                SpawnOrInsert::Insert(inserter, location.archetype_id);
                        }
                    };
                }
                AllocAtWithoutReplacement::DidNotExist => {
                    if let SpawnOrInsert::Spawn(ref mut spawner) = spawn_or_insert {
                        // SAFETY: `entity` is allocated (but non existent), bundle matches inserter
                        unsafe {
                            spawner.spawn_non_existent(
                                entity,
                                bundle,
                                #[cfg(feature = "track_change_detection")]
                                caller,
                            )
                        };
                    } else {
                        // SAFETY: we initialized this bundle_id in `init_info`
                        let mut spawner =
                            unsafe { BundleSpawner::new_with_id(self, bundle_id, change_tick) };
                        // SAFETY: `entity` is valid, `location` matches entity, bundle matches inserter
                        unsafe {
                            spawner.spawn_non_existent(
                                entity,
                                bundle,
                                #[cfg(feature = "track_change_detection")]
                                caller,
                            )
                        };
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

    /// Temporarily removes the requested resource from this [`World`], runs custom user code,
    /// then re-adds the resource before returning.
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
    #[track_caller]
    pub fn resource_scope<R: Resource, U>(&mut self, f: impl FnOnce(&mut World, Mut<R>) -> U) -> U {
        let last_change_tick = self.last_change_tick();
        let change_tick = self.change_tick();

        let component_id = self
            .components
            .get_resource_id(TypeId::of::<R>())
            .unwrap_or_else(|| panic!("resource does not exist: {}", core::any::type_name::<R>()));
        let (ptr, mut ticks, mut _caller) = self
            .storages
            .resources
            .get_mut(component_id)
            .and_then(ResourceData::remove)
            .unwrap_or_else(|| panic!("resource does not exist: {}", core::any::type_name::<R>()));
        // Read the value onto the stack to avoid potential mut aliasing.
        // SAFETY: `ptr` was obtained from the TypeId of `R`.
        let mut value = unsafe { ptr.read::<R>() };
        let value_mut = Mut {
            value: &mut value,
            ticks: TicksMut {
                added: &mut ticks.added,
                changed: &mut ticks.changed,
                last_run: last_change_tick,
                this_run: change_tick,
            },
            #[cfg(feature = "track_change_detection")]
            changed_by: &mut _caller,
        };
        let result = f(self, value_mut);
        assert!(!self.contains_resource::<R>(),
            "Resource `{}` was inserted during a call to World::resource_scope.\n\
            This is not allowed as the original resource is reinserted to the world after the closure is invoked.",
            core::any::type_name::<R>());

        OwningPtr::make(value, |ptr| {
            // SAFETY: pointer is of type R
            unsafe {
                self.storages
                    .resources
                    .get_mut(component_id)
                    .map(|info| {
                        info.insert_with_ticks(
                            ptr,
                            ticks,
                            #[cfg(feature = "track_change_detection")]
                            _caller,
                        );
                    })
                    .unwrap_or_else(|| {
                        panic!(
                            "No resource of type {} exists in the World.",
                            core::any::type_name::<R>()
                        )
                    });
            }
        });

        result
    }

    /// Sends an [`Event`].
    /// This method returns the [ID](`EventId`) of the sent `event`,
    /// or [`None`] if the `event` could not be sent.
    #[inline]
    pub fn send_event<E: Event>(&mut self, event: E) -> Option<EventId<E>> {
        self.send_event_batch(core::iter::once(event))?.next()
    }

    /// Sends the default value of the [`Event`] of type `E`.
    /// This method returns the [ID](`EventId`) of the sent `event`,
    /// or [`None`] if the `event` could not be sent.
    #[inline]
    pub fn send_event_default<E: Event + Default>(&mut self) -> Option<EventId<E>> {
        self.send_event(E::default())
    }

    /// Sends a batch of [`Event`]s from an iterator.
    /// This method returns the [IDs](`EventId`) of the sent `events`,
    /// or [`None`] if the `event` could not be sent.
    #[inline]
    pub fn send_event_batch<E: Event>(
        &mut self,
        events: impl IntoIterator<Item = E>,
    ) -> Option<SendBatchIds<E>> {
        let Some(mut events_resource) = self.get_resource_mut::<Events<E>>() else {
            bevy_utils::tracing::error!(
                "Unable to send event `{}`\n\tEvent must be added to the app with `add_event()`\n\thttps://docs.rs/bevy/*/bevy/app/struct.App.html#method.add_event ",
                core::any::type_name::<E>()
            );
            return None;
        };
        Some(events_resource.send_batch(events))
    }

    /// Inserts a new resource with the given `value`. Will replace the value if it already existed.
    ///
    /// **You should prefer to use the typed API [`World::insert_resource`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    ///
    /// # Safety
    /// The value referenced by `value` must be valid for the given [`ComponentId`] of this world.
    #[inline]
    #[track_caller]
    pub unsafe fn insert_resource_by_id(
        &mut self,
        component_id: ComponentId,
        value: OwningPtr<'_>,
        #[cfg(feature = "track_change_detection")] caller: &'static Location,
    ) {
        let change_tick = self.change_tick();

        let resource = self.initialize_resource_internal(component_id);
        // SAFETY: `value` is valid for `component_id`, ensured by caller
        unsafe {
            resource.insert(
                value,
                change_tick,
                #[cfg(feature = "track_change_detection")]
                caller,
            );
        }
    }

    /// Inserts a new `!Send` resource with the given `value`. Will replace the value if it already
    /// existed.
    ///
    /// **You should prefer to use the typed API [`World::insert_non_send_resource`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    ///
    /// # Panics
    /// If a value is already present, this function will panic if not called from the same
    /// thread that the original value was inserted from.
    ///
    /// # Safety
    /// The value referenced by `value` must be valid for the given [`ComponentId`] of this world.
    #[inline]
    #[track_caller]
    pub unsafe fn insert_non_send_by_id(
        &mut self,
        component_id: ComponentId,
        value: OwningPtr<'_>,
        #[cfg(feature = "track_change_detection")] caller: &'static Location,
    ) {
        let change_tick = self.change_tick();

        let resource = self.initialize_non_send_internal(component_id);
        // SAFETY: `value` is valid for `component_id`, ensured by caller
        unsafe {
            resource.insert(
                value,
                change_tick,
                #[cfg(feature = "track_change_detection")]
                caller,
            );
        }
    }

    /// # Panics
    /// Panics if `component_id` is not registered as a `Send` component type in this `World`
    #[inline]
    pub(crate) fn initialize_resource_internal(
        &mut self,
        component_id: ComponentId,
    ) -> &mut ResourceData<true> {
        let archetypes = &mut self.archetypes;
        self.storages
            .resources
            .initialize_with(component_id, &self.components, || {
                archetypes.new_archetype_component_id()
            })
    }

    /// # Panics
    /// Panics if `component_id` is not registered in this world
    #[inline]
    pub(crate) fn initialize_non_send_internal(
        &mut self,
        component_id: ComponentId,
    ) -> &mut ResourceData<false> {
        let archetypes = &mut self.archetypes;
        self.storages
            .non_send_resources
            .initialize_with(component_id, &self.components, || {
                archetypes.new_archetype_component_id()
            })
    }

    /// Empties queued entities and adds them to the empty [`Archetype`](crate::archetype::Archetype).
    /// This should be called before doing operations that might operate on queued entities,
    /// such as inserting a [`Component`].
    pub(crate) fn flush_entities(&mut self) {
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

    /// Applies any commands in the world's internal [`CommandQueue`].
    /// This does not apply commands from any systems, only those stored in the world.
    ///
    /// # Panics
    /// This will panic if any of the queued commands are [`spawn`](Commands::spawn).
    /// If this is possible, you should instead use [`flush`](Self::flush).
    pub(crate) fn flush_commands(&mut self) {
        // SAFETY: `self.command_queue` is only de-allocated in `World`'s `Drop`
        if !unsafe { self.command_queue.is_empty() } {
            // SAFETY: `self.command_queue` is only de-allocated in `World`'s `Drop`
            unsafe {
                self.command_queue
                    .clone()
                    .apply_or_drop_queued(Some(self.into()));
            };
        }
    }

    /// Flushes queued entities and commands.
    ///
    /// Queued entities will be spawned, and then commands will be applied.
    #[inline]
    pub fn flush(&mut self) {
        self.flush_entities();
        self.flush_commands();
    }

    /// Increments the world's current change tick and returns the old value.
    ///
    /// If you need to call this method, but do not have `&mut` access to the world,
    /// consider using [`as_unsafe_world_cell_readonly`](Self::as_unsafe_world_cell_readonly)
    /// to obtain an [`UnsafeWorldCell`] and calling [`increment_change_tick`](UnsafeWorldCell::increment_change_tick) on that.
    /// Note that this *can* be done in safe code, despite the name of the type.
    #[inline]
    pub fn increment_change_tick(&mut self) -> Tick {
        let change_tick = self.change_tick.get_mut();
        let prev_tick = *change_tick;
        *change_tick = change_tick.wrapping_add(1);
        Tick::new(prev_tick)
    }

    /// Reads the current change tick of this world.
    ///
    /// If you have exclusive (`&mut`) access to the world, consider using [`change_tick()`](Self::change_tick),
    /// which is more efficient since it does not require atomic synchronization.
    #[inline]
    pub fn read_change_tick(&self) -> Tick {
        let tick = self.change_tick.load(Ordering::Acquire);
        Tick::new(tick)
    }

    /// Reads the current change tick of this world.
    ///
    /// This does the same thing as [`read_change_tick()`](Self::read_change_tick), only this method
    /// is more efficient since it does not require atomic synchronization.
    #[inline]
    pub fn change_tick(&mut self) -> Tick {
        let tick = *self.change_tick.get_mut();
        Tick::new(tick)
    }

    /// When called from within an exclusive system (a [`System`] that takes `&mut World` as its first
    /// parameter), this method returns the [`Tick`] indicating the last time the exclusive system was run.
    ///
    /// Otherwise, this returns the `Tick` indicating the last time that [`World::clear_trackers`] was called.
    ///
    /// [`System`]: crate::system::System
    #[inline]
    pub fn last_change_tick(&self) -> Tick {
        self.last_change_tick
    }

    /// Returns the id of the last ECS event that was fired.
    /// Used internally to ensure observers don't trigger multiple times for the same event.
    #[inline]
    pub(crate) fn last_trigger_id(&self) -> u32 {
        self.last_trigger_id
    }

    /// Sets [`World::last_change_tick()`] to the specified value during a scope.
    /// When the scope terminates, it will return to its old value.
    ///
    /// This is useful if you need a region of code to be able to react to earlier changes made in the same system.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// // This function runs an update loop repeatedly, allowing each iteration of the loop
    /// // to react to changes made in the previous loop iteration.
    /// fn update_loop(
    ///     world: &mut World,
    ///     mut update_fn: impl FnMut(&mut World) -> std::ops::ControlFlow<()>,
    /// ) {
    ///     let mut last_change_tick = world.last_change_tick();
    ///
    ///     // Repeatedly run the update function until it requests a break.
    ///     loop {
    ///         let control_flow = world.last_change_tick_scope(last_change_tick, |world| {
    ///             // Increment the change tick so we can detect changes from the previous update.
    ///             last_change_tick = world.change_tick();
    ///             world.increment_change_tick();
    ///
    ///             // Update once.
    ///             update_fn(world)
    ///         });
    ///
    ///         // End the loop when the closure returns `ControlFlow::Break`.
    ///         if control_flow.is_break() {
    ///             break;
    ///         }
    ///     }
    /// }
    /// #
    /// # #[derive(Resource)] struct Count(u32);
    /// # let mut world = World::new();
    /// # world.insert_resource(Count(0));
    /// # let saved_last_tick = world.last_change_tick();
    /// # let mut num_updates = 0;
    /// # update_loop(&mut world, |world| {
    /// #     let mut c = world.resource_mut::<Count>();
    /// #     match c.0 {
    /// #         0 => {
    /// #             assert_eq!(num_updates, 0);
    /// #             assert!(c.is_added());
    /// #             c.0 = 1;
    /// #         }
    /// #         1 => {
    /// #             assert_eq!(num_updates, 1);
    /// #             assert!(!c.is_added());
    /// #             assert!(c.is_changed());
    /// #             c.0 = 2;
    /// #         }
    /// #         2 if c.is_changed() => {
    /// #             assert_eq!(num_updates, 2);
    /// #             assert!(!c.is_added());
    /// #         }
    /// #         2 => {
    /// #             assert_eq!(num_updates, 3);
    /// #             assert!(!c.is_changed());
    /// #             world.remove_resource::<Count>();
    /// #             world.insert_resource(Count(3));
    /// #         }
    /// #         3 if c.is_changed() => {
    /// #             assert_eq!(num_updates, 4);
    /// #             assert!(c.is_added());
    /// #         }
    /// #         3 => {
    /// #             assert_eq!(num_updates, 5);
    /// #             assert!(!c.is_added());
    /// #             c.0 = 4;
    /// #             return std::ops::ControlFlow::Break(());
    /// #         }
    /// #         _ => unreachable!(),
    /// #     }
    /// #     num_updates += 1;
    /// #     std::ops::ControlFlow::Continue(())
    /// # });
    /// # assert_eq!(num_updates, 5);
    /// # assert_eq!(world.resource::<Count>().0, 4);
    /// # assert_eq!(world.last_change_tick(), saved_last_tick);
    /// ```
    pub fn last_change_tick_scope<T>(
        &mut self,
        last_change_tick: Tick,
        f: impl FnOnce(&mut World) -> T,
    ) -> T {
        struct LastTickGuard<'a> {
            world: &'a mut World,
            last_tick: Tick,
        }

        // By setting the change tick in the drop impl, we ensure that
        // the change tick gets reset even if a panic occurs during the scope.
        impl Drop for LastTickGuard<'_> {
            fn drop(&mut self) {
                self.world.last_change_tick = self.last_tick;
            }
        }

        let guard = LastTickGuard {
            last_tick: self.last_change_tick,
            world: self,
        };

        guard.world.last_change_tick = last_change_tick;

        f(guard.world)
    }

    /// Iterates all component change ticks and clamps any older than [`MAX_CHANGE_AGE`](crate::change_detection::MAX_CHANGE_AGE).
    /// This prevents overflow and thus prevents false positives.
    ///
    /// **Note:** Does nothing if the [`World`] counter has not been incremented at least [`CHECK_TICK_THRESHOLD`]
    /// times since the previous pass.
    // TODO: benchmark and optimize
    pub fn check_change_ticks(&mut self) {
        let change_tick = self.change_tick();
        if change_tick.relative_to(self.last_check_tick).get() < CHECK_TICK_THRESHOLD {
            return;
        }

        let Storages {
            ref mut tables,
            ref mut sparse_sets,
            ref mut resources,
            ref mut non_send_resources,
        } = self.storages;

        #[cfg(feature = "trace")]
        let _span = bevy_utils::tracing::info_span!("check component ticks").entered();
        tables.check_change_ticks(change_tick);
        sparse_sets.check_change_ticks(change_tick);
        resources.check_change_ticks(change_tick);
        non_send_resources.check_change_ticks(change_tick);

        if let Some(mut schedules) = self.get_resource_mut::<Schedules>() {
            schedules.check_change_ticks(change_tick);
        }

        self.last_check_tick = change_tick;
    }

    /// Runs both [`clear_entities`](Self::clear_entities) and [`clear_resources`](Self::clear_resources),
    /// invalidating all [`Entity`] and resource fetches such as [`Res`](crate::system::Res), [`ResMut`](crate::system::ResMut)
    pub fn clear_all(&mut self) {
        self.clear_entities();
        self.clear_resources();
    }

    /// Despawns all entities in this [`World`].
    pub fn clear_entities(&mut self) {
        self.storages.tables.clear();
        self.storages.sparse_sets.clear_entities();
        self.archetypes.clear_entities();
        self.entities.clear();
    }

    /// Clears all resources in this [`World`].
    ///
    /// **Note:** Any resource fetch to this [`World`] will fail unless they are re-initialized,
    /// including engine-internal resources that are only initialized on app/world construction.
    ///
    /// This can easily cause systems expecting certain resources to immediately start panicking.
    /// Use with caution.
    pub fn clear_resources(&mut self) {
        self.storages.resources.clear();
        self.storages.non_send_resources.clear();
    }

    /// Registers all of the components in the given [`Bundle`] and returns both the component
    /// ids and the bundle id.
    ///
    /// This is largely equivalent to calling [`register_component`](Self::register_component) on each
    /// component in the bundle.
    #[inline]
    pub fn register_bundle<B: Bundle>(&mut self) -> &BundleInfo {
        let id = self
            .bundles
            .register_info::<B>(&mut self.components, &mut self.storages);
        // SAFETY: We just initialised the bundle so its id should definitely be valid.
        unsafe { self.bundles.get(id).debug_checked_unwrap() }
    }
}

impl World {
    /// Gets a pointer to the resource with the id [`ComponentId`] if it exists.
    /// The returned pointer must not be used to modify the resource, and must not be
    /// dereferenced after the immutable borrow of the [`World`] ends.
    ///
    /// **You should prefer to use the typed API [`World::get_resource`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    #[inline]
    pub fn get_resource_by_id(&self, component_id: ComponentId) -> Option<Ptr<'_>> {
        // SAFETY:
        // - `as_unsafe_world_cell_readonly` gives permission to access the whole world immutably
        // - `&self` ensures there are no mutable borrows on world data
        unsafe {
            self.as_unsafe_world_cell_readonly()
                .get_resource_by_id(component_id)
        }
    }

    /// Gets a pointer to the resource with the id [`ComponentId`] if it exists.
    /// The returned pointer may be used to modify the resource, as long as the mutable borrow
    /// of the [`World`] is still valid.
    ///
    /// **You should prefer to use the typed API [`World::get_resource_mut`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    #[inline]
    pub fn get_resource_mut_by_id(&mut self, component_id: ComponentId) -> Option<MutUntyped<'_>> {
        // SAFETY:
        // - `&mut self` ensures that all accessed data is unaliased
        // - `as_unsafe_world_cell` provides mutable permission to the whole world
        unsafe {
            self.as_unsafe_world_cell()
                .get_resource_mut_by_id(component_id)
        }
    }

    /// Iterates over all resources in the world.
    ///
    /// The returned iterator provides lifetimed, but type-unsafe pointers. Actually reading the contents
    /// of each resource will require the use of unsafe code.
    ///
    /// # Examples
    ///
    /// ## Printing the size of all resources
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Resource)]
    /// # struct A(u32);
    /// # #[derive(Resource)]
    /// # struct B(u32);
    /// #
    /// # let mut world = World::new();
    /// # world.insert_resource(A(1));
    /// # world.insert_resource(B(2));
    /// let mut total = 0;
    /// for (info, _) in world.iter_resources() {
    ///    println!("Resource: {}", info.name());
    ///    println!("Size: {} bytes", info.layout().size());
    ///    total += info.layout().size();
    /// }
    /// println!("Total size: {} bytes", total);
    /// # assert_eq!(total, size_of::<A>() + size_of::<B>());
    /// ```
    ///
    /// ## Dynamically running closures for resources matching specific `TypeId`s
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use std::collections::HashMap;
    /// # use std::any::TypeId;
    /// # use bevy_ptr::Ptr;
    /// # #[derive(Resource)]
    /// # struct A(u32);
    /// # #[derive(Resource)]
    /// # struct B(u32);
    /// #
    /// # let mut world = World::new();
    /// # world.insert_resource(A(1));
    /// # world.insert_resource(B(2));
    /// #
    /// // In this example, `A` and `B` are resources. We deliberately do not use the
    /// // `bevy_reflect` crate here to showcase the low-level [`Ptr`] usage. You should
    /// // probably use something like `ReflectFromPtr` in a real-world scenario.
    ///
    /// // Create the hash map that will store the closures for each resource type
    /// let mut closures: HashMap<TypeId, Box<dyn Fn(&Ptr<'_>)>> = HashMap::new();
    ///
    /// // Add closure for `A`
    /// closures.insert(TypeId::of::<A>(), Box::new(|ptr| {
    ///     // SAFETY: We assert ptr is the same type of A with TypeId of A
    ///     let a = unsafe { &ptr.deref::<A>() };
    /// #   assert_eq!(a.0, 1);
    ///     // ... do something with `a` here
    /// }));
    ///
    /// // Add closure for `B`
    /// closures.insert(TypeId::of::<B>(), Box::new(|ptr| {
    ///     // SAFETY: We assert ptr is the same type of B with TypeId of B
    ///     let b = unsafe { &ptr.deref::<B>() };
    /// #   assert_eq!(b.0, 2);
    ///     // ... do something with `b` here
    /// }));
    ///
    /// // Iterate all resources, in order to run the closures for each matching resource type
    /// for (info, ptr) in world.iter_resources() {
    ///     let Some(type_id) = info.type_id() else {
    ///        // It's possible for resources to not have a `TypeId` (e.g. non-Rust resources
    ///        // dynamically inserted via a scripting language) in which case we can't match them.
    ///        continue;
    ///     };
    ///
    ///     let Some(closure) = closures.get(&type_id) else {
    ///        // No closure for this resource type, skip it.
    ///        continue;
    ///     };
    ///
    ///     // Run the closure for the resource
    ///     closure(&ptr);
    /// }
    /// ```
    #[inline]
    pub fn iter_resources(&self) -> impl Iterator<Item = (&ComponentInfo, Ptr<'_>)> {
        self.storages
            .resources
            .iter()
            .filter_map(|(component_id, data)| {
                // SAFETY: If a resource has been initialized, a corresponding ComponentInfo must exist with its ID.
                let component_info = unsafe {
                    self.components
                        .get_info(component_id)
                        .debug_checked_unwrap()
                };
                Some((component_info, data.get_data()?))
            })
    }

    /// Mutably iterates over all resources in the world.
    ///
    /// The returned iterator provides lifetimed, but type-unsafe pointers. Actually reading from or writing
    /// to the contents of each resource will require the use of unsafe code.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_ecs::change_detection::MutUntyped;
    /// # use std::collections::HashMap;
    /// # use std::any::TypeId;
    /// # #[derive(Resource)]
    /// # struct A(u32);
    /// # #[derive(Resource)]
    /// # struct B(u32);
    /// #
    /// # let mut world = World::new();
    /// # world.insert_resource(A(1));
    /// # world.insert_resource(B(2));
    /// #
    /// // In this example, `A` and `B` are resources. We deliberately do not use the
    /// // `bevy_reflect` crate here to showcase the low-level `MutUntyped` usage. You should
    /// // probably use something like `ReflectFromPtr` in a real-world scenario.
    ///
    /// // Create the hash map that will store the mutator closures for each resource type
    /// let mut mutators: HashMap<TypeId, Box<dyn Fn(&mut MutUntyped<'_>)>> = HashMap::new();
    ///
    /// // Add mutator closure for `A`
    /// mutators.insert(TypeId::of::<A>(), Box::new(|mut_untyped| {
    ///     // Note: `MutUntyped::as_mut()` automatically marks the resource as changed
    ///     // for ECS change detection, and gives us a `PtrMut` we can use to mutate the resource.
    ///     // SAFETY: We assert ptr is the same type of A with TypeId of A
    ///     let a = unsafe { &mut mut_untyped.as_mut().deref_mut::<A>() };
    /// #   a.0 += 1;
    ///     // ... mutate `a` here
    /// }));
    ///
    /// // Add mutator closure for `B`
    /// mutators.insert(TypeId::of::<B>(), Box::new(|mut_untyped| {
    ///     // SAFETY: We assert ptr is the same type of B with TypeId of B
    ///     let b = unsafe { &mut mut_untyped.as_mut().deref_mut::<B>() };
    /// #   b.0 += 1;
    ///     // ... mutate `b` here
    /// }));
    ///
    /// // Iterate all resources, in order to run the mutator closures for each matching resource type
    /// for (info, mut mut_untyped) in world.iter_resources_mut() {
    ///     let Some(type_id) = info.type_id() else {
    ///        // It's possible for resources to not have a `TypeId` (e.g. non-Rust resources
    ///        // dynamically inserted via a scripting language) in which case we can't match them.
    ///        continue;
    ///     };
    ///
    ///     let Some(mutator) = mutators.get(&type_id) else {
    ///        // No mutator closure for this resource type, skip it.
    ///        continue;
    ///     };
    ///
    ///     // Run the mutator closure for the resource
    ///     mutator(&mut mut_untyped);
    /// }
    /// # assert_eq!(world.resource::<A>().0, 2);
    /// # assert_eq!(world.resource::<B>().0, 3);
    /// ```
    #[inline]
    pub fn iter_resources_mut(&mut self) -> impl Iterator<Item = (&ComponentInfo, MutUntyped<'_>)> {
        self.storages
            .resources
            .iter()
            .filter_map(|(component_id, data)| {
                // SAFETY: If a resource has been initialized, a corresponding ComponentInfo must exist with its ID.
                let component_info = unsafe {
                    self.components
                        .get_info(component_id)
                        .debug_checked_unwrap()
                };
                let (ptr, ticks, _caller) = data.get_with_ticks()?;

                // SAFETY:
                // - We have exclusive access to the world, so no other code can be aliasing the `TickCells`
                // - We only hold one `TicksMut` at a time, and we let go of it before getting the next one
                let ticks = unsafe {
                    TicksMut::from_tick_cells(
                        ticks,
                        self.last_change_tick(),
                        self.read_change_tick(),
                    )
                };

                let mut_untyped = MutUntyped {
                    // SAFETY:
                    // - We have exclusive access to the world, so no other code can be aliasing the `Ptr`
                    // - We iterate one resource at a time, and we let go of each `PtrMut` before getting the next one
                    value: unsafe { ptr.assert_unique() },
                    ticks,
                    #[cfg(feature = "track_change_detection")]
                    // SAFETY:
                    // - We have exclusive access to the world, so no other code can be aliasing the `Ptr`
                    // - We iterate one resource at a time, and we let go of each `PtrMut` before getting the next one
                    changed_by: unsafe { _caller.deref_mut() },
                };

                Some((component_info, mut_untyped))
            })
    }

    /// Gets a `!Send` resource to the resource with the id [`ComponentId`] if it exists.
    /// The returned pointer must not be used to modify the resource, and must not be
    /// dereferenced after the immutable borrow of the [`World`] ends.
    ///
    /// **You should prefer to use the typed API [`World::get_resource`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    ///
    /// # Panics
    /// This function will panic if it isn't called from the same thread that the resource was inserted from.
    #[inline]
    pub fn get_non_send_by_id(&self, component_id: ComponentId) -> Option<Ptr<'_>> {
        // SAFETY:
        // - `as_unsafe_world_cell_readonly` gives permission to access the whole world immutably
        // - `&self` ensures there are no mutable borrows on world data
        unsafe {
            self.as_unsafe_world_cell_readonly()
                .get_non_send_resource_by_id(component_id)
        }
    }

    /// Gets a `!Send` resource to the resource with the id [`ComponentId`] if it exists.
    /// The returned pointer may be used to modify the resource, as long as the mutable borrow
    /// of the [`World`] is still valid.
    ///
    /// **You should prefer to use the typed API [`World::get_resource_mut`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    ///
    /// # Panics
    /// This function will panic if it isn't called from the same thread that the resource was inserted from.
    #[inline]
    pub fn get_non_send_mut_by_id(&mut self, component_id: ComponentId) -> Option<MutUntyped<'_>> {
        // SAFETY:
        // - `&mut self` ensures that all accessed data is unaliased
        // - `as_unsafe_world_cell` provides mutable permission to the whole world
        unsafe {
            self.as_unsafe_world_cell()
                .get_non_send_resource_mut_by_id(component_id)
        }
    }

    /// Removes the resource of a given type, if it exists. Otherwise returns `None`.
    ///
    /// **You should prefer to use the typed API [`World::remove_resource`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    pub fn remove_resource_by_id(&mut self, component_id: ComponentId) -> Option<()> {
        self.storages
            .resources
            .get_mut(component_id)?
            .remove_and_drop();
        Some(())
    }

    /// Removes the resource of a given type, if it exists. Otherwise returns `None`.
    ///
    /// **You should prefer to use the typed API [`World::remove_resource`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    ///
    /// # Panics
    /// This function will panic if it isn't called from the same thread that the resource was inserted from.
    pub fn remove_non_send_by_id(&mut self, component_id: ComponentId) -> Option<()> {
        self.storages
            .non_send_resources
            .get_mut(component_id)?
            .remove_and_drop();
        Some(())
    }

    /// Retrieves an immutable untyped reference to the given `entity`'s [`Component`] of the given [`ComponentId`].
    /// Returns `None` if the `entity` does not have a [`Component`] of the given type.
    ///
    /// **You should prefer to use the typed API [`World::get_mut`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    ///
    /// # Panics
    /// This function will panic if it isn't called from the same thread that the resource was inserted from.
    #[inline]
    pub fn get_by_id(&self, entity: Entity, component_id: ComponentId) -> Option<Ptr<'_>> {
        // SAFETY:
        // - `&self` ensures that all accessed data is not mutably aliased
        // - `as_unsafe_world_cell_readonly` provides shared/readonly permission to the whole world
        unsafe {
            self.as_unsafe_world_cell_readonly()
                .get_entity(entity)?
                .get_by_id(component_id)
        }
    }

    /// Retrieves a mutable untyped reference to the given `entity`'s [`Component`] of the given [`ComponentId`].
    /// Returns `None` if the `entity` does not have a [`Component`] of the given type.
    ///
    /// **You should prefer to use the typed API [`World::get_mut`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    #[inline]
    pub fn get_mut_by_id(
        &mut self,
        entity: Entity,
        component_id: ComponentId,
    ) -> Option<MutUntyped<'_>> {
        // SAFETY:
        // - `&mut self` ensures that all accessed data is unaliased
        // - `as_unsafe_world_cell` provides mutable permission to the whole world
        unsafe {
            self.as_unsafe_world_cell()
                .get_entity(entity)?
                .get_mut_by_id(component_id)
        }
    }
}

// Schedule-related methods
impl World {
    /// Adds the specified [`Schedule`] to the world. The schedule can later be run
    /// by calling [`.run_schedule(label)`](Self::run_schedule) or by directly
    /// accessing the [`Schedules`] resource.
    ///
    /// The `Schedules` resource will be initialized if it does not already exist.
    pub fn add_schedule(&mut self, schedule: Schedule) {
        let mut schedules = self.get_resource_or_init::<Schedules>();
        schedules.insert(schedule);
    }

    /// Temporarily removes the schedule associated with `label` from the world,
    /// runs user code, and finally re-adds the schedule.
    /// This returns a [`TryRunScheduleError`] if there is no schedule
    /// associated with `label`.
    ///
    /// The [`Schedule`] is fetched from the [`Schedules`] resource of the world by its label,
    /// and system state is cached.
    ///
    /// For simple cases where you just need to call the schedule once,
    /// consider using [`World::try_run_schedule`] instead.
    /// For other use cases, see the example on [`World::schedule_scope`].
    pub fn try_schedule_scope<R>(
        &mut self,
        label: impl ScheduleLabel,
        f: impl FnOnce(&mut World, &mut Schedule) -> R,
    ) -> Result<R, TryRunScheduleError> {
        let label = label.intern();
        let Some(mut schedule) = self
            .get_resource_mut::<Schedules>()
            .and_then(|mut s| s.remove(label))
        else {
            return Err(TryRunScheduleError(label));
        };

        let value = f(self, &mut schedule);

        let old = self.resource_mut::<Schedules>().insert(schedule);
        if old.is_some() {
            warn!("Schedule `{label:?}` was inserted during a call to `World::schedule_scope`: its value has been overwritten");
        }

        Ok(value)
    }

    /// Temporarily removes the schedule associated with `label` from the world,
    /// runs user code, and finally re-adds the schedule.
    ///
    /// The [`Schedule`] is fetched from the [`Schedules`] resource of the world by its label,
    /// and system state is cached.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, schedule::ScheduleLabel};
    /// # #[derive(ScheduleLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    /// # pub struct MySchedule;
    /// # #[derive(Resource)]
    /// # struct Counter(usize);
    /// #
    /// # let mut world = World::new();
    /// # world.insert_resource(Counter(0));
    /// # let mut schedule = Schedule::new(MySchedule);
    /// # schedule.add_systems(tick_counter);
    /// # world.init_resource::<Schedules>();
    /// # world.add_schedule(schedule);
    /// # fn tick_counter(mut counter: ResMut<Counter>) { counter.0 += 1; }
    /// // Run the schedule five times.
    /// world.schedule_scope(MySchedule, |world, schedule| {
    ///     for _ in 0..5 {
    ///         schedule.run(world);
    ///     }
    /// });
    /// # assert_eq!(world.resource::<Counter>().0, 5);
    /// ```
    ///
    /// For simple cases where you just need to call the schedule once,
    /// consider using [`World::run_schedule`] instead.
    ///
    /// # Panics
    ///
    /// If the requested schedule does not exist.
    pub fn schedule_scope<R>(
        &mut self,
        label: impl ScheduleLabel,
        f: impl FnOnce(&mut World, &mut Schedule) -> R,
    ) -> R {
        self.try_schedule_scope(label, f)
            .unwrap_or_else(|e| panic!("{e}"))
    }

    /// Attempts to run the [`Schedule`] associated with the `label` a single time,
    /// and returns a [`TryRunScheduleError`] if the schedule does not exist.
    ///
    /// The [`Schedule`] is fetched from the [`Schedules`] resource of the world by its label,
    /// and system state is cached.
    ///
    /// For simple testing use cases, call [`Schedule::run(&mut world)`](Schedule::run) instead.
    pub fn try_run_schedule(
        &mut self,
        label: impl ScheduleLabel,
    ) -> Result<(), TryRunScheduleError> {
        self.try_schedule_scope(label, |world, sched| sched.run(world))
    }

    /// Runs the [`Schedule`] associated with the `label` a single time.
    ///
    /// The [`Schedule`] is fetched from the [`Schedules`] resource of the world by its label,
    /// and system state is cached.
    ///
    /// For simple testing use cases, call [`Schedule::run(&mut world)`](Schedule::run) instead.
    ///
    /// # Panics
    ///
    /// If the requested schedule does not exist.
    pub fn run_schedule(&mut self, label: impl ScheduleLabel) {
        self.schedule_scope(label, |world, sched| sched.run(world));
    }

    /// Ignore system order ambiguities caused by conflicts on [`Component`]s of type `T`.
    pub fn allow_ambiguous_component<T: Component>(&mut self) {
        let mut schedules = self.remove_resource::<Schedules>().unwrap_or_default();
        schedules.allow_ambiguous_component::<T>(self);
        self.insert_resource(schedules);
    }

    /// Ignore system order ambiguities caused by conflicts on [`Resource`]s of type `T`.
    pub fn allow_ambiguous_resource<T: Resource>(&mut self) {
        let mut schedules = self.remove_resource::<Schedules>().unwrap_or_default();
        schedules.allow_ambiguous_resource::<T>(self);
        self.insert_resource(schedules);
    }
}

impl fmt::Debug for World {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // SAFETY: `UnsafeWorldCell` requires that this must only access metadata.
        // Accessing any data stored in the world would be unsound.
        f.debug_struct("World")
            .field("id", &self.id)
            .field("entity_count", &self.entities.len())
            .field("archetype_count", &self.archetypes.len())
            .field("component_count", &self.components.len())
            .field("resource_count", &self.storages.resources.len())
            .finish()
    }
}

// SAFETY: all methods on the world ensure that non-send resources are only accessible on the main thread
unsafe impl Send for World {}
// SAFETY: all methods on the world ensure that non-send resources are only accessible on the main thread
unsafe impl Sync for World {}

/// Creates an instance of the type this trait is implemented for
/// using data from the supplied [`World`].
///
/// This can be helpful for complex initialization or context-aware defaults.
///
/// [`FromWorld`] is automatically implemented for any type implementing [`Default`].
pub trait FromWorld {
    /// Creates `Self` using data from the given [`World`].
    fn from_world(world: &mut World) -> Self;
}

impl<T: Default> FromWorld for T {
    /// Creates `Self` using [`default()`](`Default::default`).
    fn from_world(_world: &mut World) -> Self {
        T::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{FromWorld, World};
    use crate::{
        change_detection::DetectChangesMut,
        component::{ComponentDescriptor, ComponentInfo, StorageType},
        entity::EntityHashSet,
        ptr::OwningPtr,
        system::Resource,
        world::error::EntityFetchError,
    };
    use alloc::sync::Arc;
    use bevy_ecs_macros::Component;
    use bevy_utils::{HashMap, HashSet};
    use core::{
        any::TypeId,
        panic,
        sync::atomic::{AtomicBool, AtomicU32, Ordering},
    };
    use std::sync::Mutex;

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
                    Err(e) => std::panic::resume_unwind(e),
                }
            }

            drop_log.to_owned()
        }
    }

    #[test]
    fn panic_while_overwriting_component() {
        let helper = DropTestHelper::new();

        let res = std::panic::catch_unwind(|| {
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

    #[derive(Resource)]
    struct TestResource2(String);

    #[derive(Resource)]
    struct TestResource3;

    #[test]
    fn get_resource_by_id() {
        let mut world = World::new();
        world.insert_resource(TestResource(42));
        let component_id = world
            .components()
            .get_resource_id(TypeId::of::<TestResource>())
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
            .get_resource_id(TypeId::of::<TestResource>())
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
    fn iter_resources() {
        let mut world = World::new();
        world.insert_resource(TestResource(42));
        world.insert_resource(TestResource2("Hello, world!".to_string()));
        world.insert_resource(TestResource3);
        world.remove_resource::<TestResource3>();

        let mut iter = world.iter_resources();

        let (info, ptr) = iter.next().unwrap();
        assert_eq!(info.name(), core::any::type_name::<TestResource>());
        // SAFETY: We know that the resource is of type `TestResource`
        assert_eq!(unsafe { ptr.deref::<TestResource>().0 }, 42);

        let (info, ptr) = iter.next().unwrap();
        assert_eq!(info.name(), core::any::type_name::<TestResource2>());
        assert_eq!(
            // SAFETY: We know that the resource is of type `TestResource2`
            unsafe { &ptr.deref::<TestResource2>().0 },
            &"Hello, world!".to_string()
        );

        assert!(iter.next().is_none());
    }

    #[test]
    fn iter_resources_mut() {
        let mut world = World::new();
        world.insert_resource(TestResource(42));
        world.insert_resource(TestResource2("Hello, world!".to_string()));
        world.insert_resource(TestResource3);
        world.remove_resource::<TestResource3>();

        let mut iter = world.iter_resources_mut();

        let (info, mut mut_untyped) = iter.next().unwrap();
        assert_eq!(info.name(), core::any::type_name::<TestResource>());
        // SAFETY: We know that the resource is of type `TestResource`
        unsafe {
            mut_untyped.as_mut().deref_mut::<TestResource>().0 = 43;
        };

        let (info, mut mut_untyped) = iter.next().unwrap();
        assert_eq!(info.name(), core::any::type_name::<TestResource2>());
        // SAFETY: We know that the resource is of type `TestResource2`
        unsafe {
            mut_untyped.as_mut().deref_mut::<TestResource2>().0 = "Hello, world?".to_string();
        };

        assert!(iter.next().is_none());
        drop(iter);

        assert_eq!(world.resource::<TestResource>().0, 43);
        assert_eq!(
            world.resource::<TestResource2>().0,
            "Hello, world?".to_string()
        );
    }

    #[test]
    fn dynamic_resource() {
        let mut world = World::new();

        let descriptor = ComponentDescriptor::new_resource::<TestResource>();

        let component_id = world.register_resource_with_descriptor(descriptor);

        let value = 0;
        OwningPtr::make(value, |ptr| {
            // SAFETY: value is valid for the layout of `TestResource`
            unsafe {
                world.insert_resource_by_id(
                    component_id,
                    ptr,
                    #[cfg(feature = "track_change_detection")]
                    panic::Location::caller(),
                );
            }
        });

        // SAFETY: We know that the resource is of type `TestResource`
        let resource = unsafe {
            world
                .get_resource_by_id(component_id)
                .unwrap()
                .deref::<TestResource>()
        };
        assert_eq!(resource.0, 0);

        assert!(world.remove_resource_by_id(component_id).is_some());
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
                core::alloc::Layout::new::<[u8; 8]>(),
                Some(|ptr| {
                    let data = ptr.read::<[u8; 8]>();
                    assert_eq!(data, [0, 1, 2, 3, 4, 5, 6, 7]);
                    DROP_COUNT.fetch_add(1, Ordering::SeqCst);
                }),
            )
        };

        let component_id = world.register_resource_with_descriptor(descriptor);

        let value: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
        OwningPtr::make(value, |ptr| {
            // SAFETY: value is valid for the component layout
            unsafe {
                world.insert_resource_by_id(
                    component_id,
                    ptr,
                    #[cfg(feature = "track_change_detection")]
                    panic::Location::caller(),
                );
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

        assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 1);
    }

    #[derive(Resource)]
    struct TestFromWorld(u32);
    impl FromWorld for TestFromWorld {
        fn from_world(world: &mut World) -> Self {
            let b = world.resource::<TestResource>();
            Self(b.0)
        }
    }

    #[test]
    fn init_resource_does_not_overwrite() {
        let mut world = World::new();
        world.insert_resource(TestResource(0));
        world.init_resource::<TestFromWorld>();
        world.insert_resource(TestResource(1));
        world.init_resource::<TestFromWorld>();

        let resource = world.resource::<TestFromWorld>();

        assert_eq!(resource.0, 0);
    }

    #[test]
    fn init_non_send_resource_does_not_overwrite() {
        let mut world = World::new();
        world.insert_resource(TestResource(0));
        world.init_non_send_resource::<TestFromWorld>();
        world.insert_resource(TestResource(1));
        world.init_non_send_resource::<TestFromWorld>();

        let resource = world.non_send_resource::<TestFromWorld>();

        assert_eq!(resource.0, 0);
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
                .map(ComponentInfo::type_id)
                .collect()
        }

        let foo_id = TypeId::of::<Foo>();
        let bar_id = TypeId::of::<Bar>();
        let baz_id = TypeId::of::<Baz>();
        assert_eq!(
            to_type_ids(world.inspect_entity(ent0).collect()),
            [Some(foo_id), Some(bar_id), Some(baz_id)].into()
        );
        assert_eq!(
            to_type_ids(world.inspect_entity(ent1).collect()),
            [Some(foo_id), Some(bar_id)].into()
        );
        assert_eq!(
            to_type_ids(world.inspect_entity(ent2).collect()),
            [Some(bar_id), Some(baz_id)].into()
        );
        assert_eq!(
            to_type_ids(world.inspect_entity(ent3).collect()),
            [Some(foo_id), Some(baz_id)].into()
        );
        assert_eq!(
            to_type_ids(world.inspect_entity(ent4).collect()),
            [Some(foo_id)].into()
        );
        assert_eq!(
            to_type_ids(world.inspect_entity(ent5).collect()),
            [Some(bar_id)].into()
        );
        assert_eq!(
            to_type_ids(world.inspect_entity(ent6).collect()),
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
                let counter = entity_counters.entry(entity.id()).or_insert(0);
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
    fn iterate_entities_mut() {
        #[derive(Component, PartialEq, Debug)]
        struct A(i32);

        #[derive(Component, PartialEq, Debug)]
        struct B(i32);

        let mut world = World::new();

        let a1 = world.spawn(A(1)).id();
        let a2 = world.spawn(A(2)).id();
        let b1 = world.spawn(B(1)).id();
        let b2 = world.spawn(B(2)).id();

        for mut entity in world.iter_entities_mut() {
            if let Some(mut a) = entity.get_mut::<A>() {
                a.0 -= 1;
            }
        }
        assert_eq!(world.entity(a1).get(), Some(&A(0)));
        assert_eq!(world.entity(a2).get(), Some(&A(1)));
        assert_eq!(world.entity(b1).get(), Some(&B(1)));
        assert_eq!(world.entity(b2).get(), Some(&B(2)));

        for mut entity in world.iter_entities_mut() {
            if let Some(mut b) = entity.get_mut::<B>() {
                b.0 *= 2;
            }
        }
        assert_eq!(world.entity(a1).get(), Some(&A(0)));
        assert_eq!(world.entity(a2).get(), Some(&A(1)));
        assert_eq!(world.entity(b1).get(), Some(&B(2)));
        assert_eq!(world.entity(b2).get(), Some(&B(4)));

        let mut entities = world.iter_entities_mut().collect::<Vec<_>>();
        entities.sort_by_key(|e| e.get::<A>().map(|a| a.0).or(e.get::<B>().map(|b| b.0)));
        let (a, b) = entities.split_at_mut(2);
        core::mem::swap(
            &mut a[1].get_mut::<A>().unwrap().0,
            &mut b[0].get_mut::<B>().unwrap().0,
        );
        assert_eq!(world.entity(a1).get(), Some(&A(0)));
        assert_eq!(world.entity(a2).get(), Some(&A(2)));
        assert_eq!(world.entity(b1).get(), Some(&B(1)));
        assert_eq!(world.entity(b2).get(), Some(&B(4)));
    }

    #[test]
    fn spawn_empty_bundle() {
        let mut world = World::new();
        world.spawn(());
    }

    #[test]
    fn get_entity() {
        let mut world = World::new();

        let e1 = world.spawn_empty().id();
        let e2 = world.spawn_empty().id();

        assert!(world.get_entity(e1).is_ok());
        assert!(world.get_entity([e1, e2]).is_ok());
        assert!(world
            .get_entity(&[e1, e2] /* this is an array not a slice */)
            .is_ok());
        assert!(world.get_entity(&vec![e1, e2][..]).is_ok());
        assert!(world
            .get_entity(&EntityHashSet::from_iter([e1, e2]))
            .is_ok());

        world.entity_mut(e1).despawn();

        assert_eq!(Err(e1), world.get_entity(e1).map(|_| {}));
        assert_eq!(Err(e1), world.get_entity([e1, e2]).map(|_| {}));
        assert_eq!(
            Err(e1),
            world
                .get_entity(&[e1, e2] /* this is an array not a slice */)
                .map(|_| {})
        );
        assert_eq!(Err(e1), world.get_entity(&vec![e1, e2][..]).map(|_| {}));
        assert_eq!(
            Err(e1),
            world
                .get_entity(&EntityHashSet::from_iter([e1, e2]))
                .map(|_| {})
        );
    }

    #[test]
    fn get_entity_mut() {
        let mut world = World::new();

        let e1 = world.spawn_empty().id();
        let e2 = world.spawn_empty().id();

        assert!(world.get_entity_mut(e1).is_ok());
        assert!(world.get_entity_mut([e1, e2]).is_ok());
        assert!(world
            .get_entity_mut(&[e1, e2] /* this is an array not a slice */)
            .is_ok());
        assert!(world.get_entity_mut(&vec![e1, e2][..]).is_ok());
        assert!(world
            .get_entity_mut(&EntityHashSet::from_iter([e1, e2]))
            .is_ok());

        assert_eq!(
            Err(EntityFetchError::AliasedMutability(e1)),
            world.get_entity_mut([e1, e2, e1]).map(|_| {})
        );
        assert_eq!(
            Err(EntityFetchError::AliasedMutability(e1)),
            world
                .get_entity_mut(&[e1, e2, e1] /* this is an array not a slice */)
                .map(|_| {})
        );
        assert_eq!(
            Err(EntityFetchError::AliasedMutability(e1)),
            world.get_entity_mut(&vec![e1, e2, e1][..]).map(|_| {})
        );
        // Aliased mutability isn't allowed by HashSets
        assert!(world
            .get_entity_mut(&EntityHashSet::from_iter([e1, e2, e1]))
            .is_ok());

        world.entity_mut(e1).despawn();

        assert_eq!(
            Err(EntityFetchError::NoSuchEntity(e1)),
            world.get_entity_mut(e1).map(|_| {})
        );
        assert_eq!(
            Err(EntityFetchError::NoSuchEntity(e1)),
            world.get_entity_mut([e1, e2]).map(|_| {})
        );
        assert_eq!(
            Err(EntityFetchError::NoSuchEntity(e1)),
            world
                .get_entity_mut(&[e1, e2] /* this is an array not a slice */)
                .map(|_| {})
        );
        assert_eq!(
            Err(EntityFetchError::NoSuchEntity(e1)),
            world.get_entity_mut(&vec![e1, e2][..]).map(|_| {})
        );
        assert_eq!(
            Err(EntityFetchError::NoSuchEntity(e1)),
            world
                .get_entity_mut(&EntityHashSet::from_iter([e1, e2]))
                .map(|_| {})
        );
    }
}
