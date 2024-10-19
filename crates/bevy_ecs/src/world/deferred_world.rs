use core::ops::Deref;

use crate::{
    archetype::Archetype,
    change_detection::MutUntyped,
    component::ComponentId,
    entity::Entity,
    event::{Event, EventId, Events, SendBatchIds},
    observer::{Observers, TriggerTargets},
    prelude::{Component, QueryState},
    query::{QueryData, QueryFilter},
    system::{Commands, Query, Resource},
    traversal::Traversal,
    world::{error::EntityFetchError, WorldEntityFetch},
};

use super::{unsafe_world_cell::UnsafeWorldCell, Mut, World};

/// A [`World`] reference that disallows structural ECS changes.
/// This includes initializing resources, registering components or spawning entities.
pub struct DeferredWorld<'w> {
    // SAFETY: Implementors must not use this reference to make structural changes
    world: UnsafeWorldCell<'w>,
}

impl<'w> Deref for DeferredWorld<'w> {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        // SAFETY: Structural changes cannot be made through &World
        unsafe { self.world.world() }
    }
}

impl<'w> UnsafeWorldCell<'w> {
    /// Turn self into a [`DeferredWorld`]
    ///
    /// # Safety
    /// Caller must ensure there are no outstanding mutable references to world and no
    /// outstanding references to the world's command queue, resource or component data
    #[inline]
    pub unsafe fn into_deferred(self) -> DeferredWorld<'w> {
        DeferredWorld { world: self }
    }
}

impl<'w> From<&'w mut World> for DeferredWorld<'w> {
    fn from(world: &'w mut World) -> DeferredWorld<'w> {
        DeferredWorld {
            world: world.as_unsafe_world_cell(),
        }
    }
}

impl<'w> DeferredWorld<'w> {
    /// Reborrow self as a new instance of [`DeferredWorld`]
    #[inline]
    pub fn reborrow(&mut self) -> DeferredWorld {
        DeferredWorld { world: self.world }
    }

    /// Creates a [`Commands`] instance that pushes to the world's command queue
    #[inline]
    pub fn commands(&mut self) -> Commands {
        // SAFETY: &mut self ensure that there are no outstanding accesses to the queue
        let command_queue = unsafe { self.world.get_raw_command_queue() };
        // SAFETY: command_queue is stored on world and always valid while the world exists
        unsafe { Commands::new_raw_from_entities(command_queue, self.world.entities()) }
    }

    /// Retrieves a mutable reference to the given `entity`'s [`Component`] of the given type.
    /// Returns `None` if the `entity` does not have a [`Component`] of the given type.
    #[inline]
    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<Mut<T>> {
        // SAFETY:
        // - `as_unsafe_world_cell` is the only thing that is borrowing world
        // - `as_unsafe_world_cell` provides mutable permission to everything
        // - `&mut self` ensures no other borrows on world data
        unsafe { self.world.get_entity(entity)?.get_mut() }
    }

    /// Returns [`EntityMut`]s that expose read and write operations for the
    /// given `entities`, returning [`Err`] if any of the given entities do not
    /// exist. Instead of immediately unwrapping the value returned from this
    /// function, prefer [`World::entity_mut`].
    ///
    /// This function supports fetching a single entity or multiple entities:
    /// - Pass an [`Entity`] to receive a single [`EntityMut`].
    /// - Pass a slice of [`Entity`]s to receive a [`Vec<EntityMut>`].
    /// - Pass an array of [`Entity`]s to receive an equally-sized array of [`EntityMut`]s.
    /// - Pass an [`&EntityHashSet`] to receive an [`EntityHashMap<EntityMut>`].
    ///
    /// **As [`DeferredWorld`] does not allow structural changes, all returned
    /// references are [`EntityMut`]s, which do not allow structural changes
    /// (i.e. adding/removing components or despawning the entity).**
    ///
    /// # Errors
    ///
    /// - Returns [`EntityFetchError::NoSuchEntity`] if any of the given `entities` do not exist in the world.
    ///     - Only the first entity found to be missing will be returned.
    /// - Returns [`EntityFetchError::AliasedMutability`] if the same entity is requested multiple times.
    ///
    /// # Examples
    ///
    /// For examples, see [`DeferredWorld::entity_mut`].
    ///
    /// [`EntityMut`]: crate::world::EntityMut
    /// [`&EntityHashSet`]: crate::entity::EntityHashSet
    /// [`EntityHashMap<EntityMut>`]: crate::entity::EntityHashMap
    #[inline]
    pub fn get_entity_mut<F: WorldEntityFetch>(
        &mut self,
        entities: F,
    ) -> Result<F::DeferredMut<'_>, EntityFetchError> {
        let cell = self.as_unsafe_world_cell();
        // SAFETY: `&mut self` gives mutable access to the entire world,
        // and prevents any other access to the world.
        unsafe { entities.fetch_deferred_mut(cell) }
    }

    /// Returns [`EntityMut`]s that expose read and write operations for the
    /// given `entities`. This will panic if any of the given entities do not
    /// exist. Use [`DeferredWorld::get_entity_mut`] if you want to check for
    /// entity existence instead of implicitly panicking.
    ///
    /// This function supports fetching a single entity or multiple entities:
    /// - Pass an [`Entity`] to receive a single [`EntityMut`].
    /// - Pass a slice of [`Entity`]s to receive a [`Vec<EntityMut>`].
    /// - Pass an array of [`Entity`]s to receive an equally-sized array of [`EntityMut`]s.
    /// - Pass an [`&EntityHashSet`] to receive an [`EntityHashMap<EntityMut>`].
    ///
    /// **As [`DeferredWorld`] does not allow structural changes, all returned
    /// references are [`EntityMut`]s, which do not allow structural changes
    /// (i.e. adding/removing components or despawning the entity).**
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
    /// # use bevy_ecs::{prelude::*, world::DeferredWorld};
    /// #[derive(Component)]
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// # let mut world = World::new();
    /// # let entity = world.spawn(Position { x: 0.0, y: 0.0 }).id();
    /// let mut world: DeferredWorld = // ...
    /// #   DeferredWorld::from(&mut world);
    ///
    /// let mut entity_mut = world.entity_mut(entity);
    /// let mut position = entity_mut.get_mut::<Position>().unwrap();
    /// position.y = 1.0;
    /// assert_eq!(position.x, 0.0);
    /// ```
    ///
    /// ## Array of [`Entity`]s
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, world::DeferredWorld};
    /// #[derive(Component)]
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// # let mut world = World::new();
    /// # let e1 = world.spawn(Position { x: 0.0, y: 0.0 }).id();
    /// # let e2 = world.spawn(Position { x: 1.0, y: 1.0 }).id();
    /// let mut world: DeferredWorld = // ...
    /// #   DeferredWorld::from(&mut world);
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
    /// # use bevy_ecs::{prelude::*, world::DeferredWorld};
    /// #[derive(Component)]
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// # let mut world = World::new();
    /// # let e1 = world.spawn(Position { x: 0.0, y: 1.0 }).id();
    /// # let e2 = world.spawn(Position { x: 0.0, y: 1.0 }).id();
    /// # let e3 = world.spawn(Position { x: 0.0, y: 1.0 }).id();
    /// let mut world: DeferredWorld = // ...
    /// #   DeferredWorld::from(&mut world);
    ///
    /// let ids = vec![e1, e2, e3];
    /// for mut eref in world.entity_mut(&ids[..]) {
    ///     let mut pos = eref.get_mut::<Position>().unwrap();
    ///     pos.y = 2.0;
    ///     assert_eq!(pos.y, 2.0);
    /// }
    /// ```
    ///
    /// ## [`&EntityHashSet`]
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, entity::EntityHashSet, world::DeferredWorld};
    /// #[derive(Component)]
    /// struct Position {
    ///   x: f32,
    ///   y: f32,
    /// }
    ///
    /// # let mut world = World::new();
    /// # let e1 = world.spawn(Position { x: 0.0, y: 1.0 }).id();
    /// # let e2 = world.spawn(Position { x: 0.0, y: 1.0 }).id();
    /// # let e3 = world.spawn(Position { x: 0.0, y: 1.0 }).id();
    /// let mut world: DeferredWorld = // ...
    /// #   DeferredWorld::from(&mut world);
    ///
    /// let ids = EntityHashSet::from_iter([e1, e2, e3]);
    /// for (_id, mut eref) in world.entity_mut(&ids) {
    ///     let mut pos = eref.get_mut::<Position>().unwrap();
    ///     pos.y = 2.0;
    ///     assert_eq!(pos.y, 2.0);
    /// }
    /// ```
    ///
    /// [`EntityMut`]: crate::world::EntityMut
    /// [`&EntityHashSet`]: crate::entity::EntityHashSet
    /// [`EntityHashMap<EntityMut>`]: crate::entity::EntityHashMap
    #[inline]
    pub fn entity_mut<F: WorldEntityFetch>(&mut self, entities: F) -> F::DeferredMut<'_> {
        self.get_entity_mut(entities).unwrap()
    }

    /// Returns [`Query`] for the given [`QueryState`], which is used to efficiently
    /// run queries on the [`World`] by storing and reusing the [`QueryState`].
    ///
    /// # Panics
    /// If state is from a different world then self
    #[inline]
    pub fn query<'s, D: QueryData, F: QueryFilter>(
        &mut self,
        state: &'s mut QueryState<D, F>,
    ) -> Query<'_, 's, D, F> {
        state.validate_world(self.world.id());
        state.update_archetypes(self);
        // SAFETY: We ran validate_world to ensure our state matches
        unsafe {
            let world_cell = self.world;
            Query::new(
                world_cell,
                state,
                world_cell.last_change_tick(),
                world_cell.change_tick(),
            )
        }
    }

    /// Gets a mutable reference to the resource of the given type
    ///
    /// # Panics
    ///
    /// Panics if the resource does not exist.
    /// Use [`get_resource_mut`](DeferredWorld::get_resource_mut) instead if you want to handle this case.
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

    /// Gets a mutable reference to the resource of the given type if it exists
    #[inline]
    pub fn get_resource_mut<R: Resource>(&mut self) -> Option<Mut<'_, R>> {
        // SAFETY: &mut self ensure that there are no outstanding accesses to the resource
        unsafe { self.world.get_resource_mut() }
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

    /// Gets a mutable reference to the non-send resource of the given type, if it exists.
    /// Otherwise returns `None`.
    ///
    /// # Panics
    /// This function will panic if it isn't called from the same thread that the resource was inserted from.
    #[inline]
    pub fn get_non_send_resource_mut<R: 'static>(&mut self) -> Option<Mut<'_, R>> {
        // SAFETY: &mut self ensure that there are no outstanding accesses to the resource
        unsafe { self.world.get_non_send_resource_mut() }
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

    /// Gets a pointer to the resource with the id [`ComponentId`] if it exists.
    /// The returned pointer may be used to modify the resource, as long as the mutable borrow
    /// of the [`World`] is still valid.
    ///
    /// **You should prefer to use the typed API [`World::get_resource_mut`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    #[inline]
    pub fn get_resource_mut_by_id(&mut self, component_id: ComponentId) -> Option<MutUntyped<'_>> {
        // SAFETY: &mut self ensure that there are no outstanding accesses to the resource
        unsafe { self.world.get_resource_mut_by_id(component_id) }
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
        // SAFETY: &mut self ensure that there are no outstanding accesses to the resource
        unsafe { self.world.get_non_send_resource_mut_by_id(component_id) }
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
        // SAFETY: &mut self ensure that there are no outstanding accesses to the resource
        unsafe { self.world.get_entity(entity)?.get_mut_by_id(component_id) }
    }

    /// Triggers all `on_add` hooks for [`ComponentId`] in target.
    ///
    /// # Safety
    /// Caller must ensure [`ComponentId`] in target exist in self.
    #[inline]
    pub(crate) unsafe fn trigger_on_add(
        &mut self,
        archetype: &Archetype,
        entity: Entity,
        targets: impl Iterator<Item = ComponentId>,
    ) {
        if archetype.has_add_hook() {
            for component_id in targets {
                // SAFETY: Caller ensures that these components exist
                let hooks = unsafe { self.components().get_info_unchecked(component_id) }.hooks();
                if let Some(hook) = hooks.on_add {
                    hook(DeferredWorld { world: self.world }, entity, component_id);
                }
            }
        }
    }

    /// Triggers all `on_insert` hooks for [`ComponentId`] in target.
    ///
    /// # Safety
    /// Caller must ensure [`ComponentId`] in target exist in self.
    #[inline]
    pub(crate) unsafe fn trigger_on_insert(
        &mut self,
        archetype: &Archetype,
        entity: Entity,
        targets: impl Iterator<Item = ComponentId>,
    ) {
        if archetype.has_insert_hook() {
            for component_id in targets {
                // SAFETY: Caller ensures that these components exist
                let hooks = unsafe { self.components().get_info_unchecked(component_id) }.hooks();
                if let Some(hook) = hooks.on_insert {
                    hook(DeferredWorld { world: self.world }, entity, component_id);
                }
            }
        }
    }

    /// Triggers all `on_replace` hooks for [`ComponentId`] in target.
    ///
    /// # Safety
    /// Caller must ensure [`ComponentId`] in target exist in self.
    #[inline]
    pub(crate) unsafe fn trigger_on_replace(
        &mut self,
        archetype: &Archetype,
        entity: Entity,
        targets: impl Iterator<Item = ComponentId>,
    ) {
        if archetype.has_replace_hook() {
            for component_id in targets {
                // SAFETY: Caller ensures that these components exist
                let hooks = unsafe { self.components().get_info_unchecked(component_id) }.hooks();
                if let Some(hook) = hooks.on_replace {
                    hook(DeferredWorld { world: self.world }, entity, component_id);
                }
            }
        }
    }

    /// Triggers all `on_remove` hooks for [`ComponentId`] in target.
    ///
    /// # Safety
    /// Caller must ensure [`ComponentId`] in target exist in self.
    #[inline]
    pub(crate) unsafe fn trigger_on_remove(
        &mut self,
        archetype: &Archetype,
        entity: Entity,
        targets: impl Iterator<Item = ComponentId>,
    ) {
        if archetype.has_remove_hook() {
            for component_id in targets {
                // SAFETY: Caller ensures that these components exist
                let hooks = unsafe { self.components().get_info_unchecked(component_id) }.hooks();
                if let Some(hook) = hooks.on_remove {
                    hook(DeferredWorld { world: self.world }, entity, component_id);
                }
            }
        }
    }

    /// Triggers all event observers for [`ComponentId`] in target.
    ///
    /// # Safety
    /// Caller must ensure observers listening for `event` can accept ZST pointers
    #[inline]
    pub(crate) unsafe fn trigger_observers(
        &mut self,
        event: ComponentId,
        entity: Entity,
        components: impl Iterator<Item = ComponentId> + Clone,
    ) {
        Observers::invoke::<_>(
            self.reborrow(),
            event,
            entity,
            components,
            &mut (),
            &mut false,
        );
    }

    /// Triggers all event observers for [`ComponentId`] in target.
    ///
    /// # Safety
    /// Caller must ensure `E` is accessible as the type represented by `event`
    #[inline]
    pub(crate) unsafe fn trigger_observers_with_data<E, T>(
        &mut self,
        event: ComponentId,
        mut entity: Entity,
        components: &[ComponentId],
        data: &mut E,
        mut propagate: bool,
    ) where
        T: Traversal,
    {
        loop {
            Observers::invoke::<_>(
                self.reborrow(),
                event,
                entity,
                components.iter().copied(),
                data,
                &mut propagate,
            );
            if !propagate {
                break;
            }
            if let Some(traverse_to) = self
                .get_entity(entity)
                .ok()
                .and_then(|entity| entity.get_components::<T>())
                .and_then(T::traverse)
            {
                entity = traverse_to;
            } else {
                break;
            }
        }
    }

    /// Sends a "global" [`Trigger`](crate::observer::Trigger) without any targets.
    pub fn trigger<T: Event>(&mut self, trigger: impl Event) {
        self.commands().trigger(trigger);
    }

    /// Sends a [`Trigger`](crate::observer::Trigger) with the given `targets`.
    pub fn trigger_targets(
        &mut self,
        trigger: impl Event,
        targets: impl TriggerTargets + Send + Sync + 'static,
    ) {
        self.commands().trigger_targets(trigger, targets);
    }

    /// Gets an [`UnsafeWorldCell`] containing the underlying world.
    ///
    /// # Safety
    /// - must only be used to make non-structural ECS changes
    #[inline]
    pub(crate) fn as_unsafe_world_cell(&mut self) -> UnsafeWorldCell {
        self.world
    }
}
