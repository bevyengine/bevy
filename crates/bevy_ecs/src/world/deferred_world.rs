use std::ops::Deref;

use crate::{
    change_detection::MutUntyped,
    component::ComponentId,
    entity::Entity,
    event::{Event, EventId, Events, SendBatchIds},
    prelude::{Component, QueryState},
    query::{QueryData, QueryFilter},
    system::{Commands, Query, Resource},
};

use super::{
    unsafe_world_cell::{UnsafeEntityCell, UnsafeWorldCell},
    EntityMut, Mut, World,
};

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
    /// Creates a [`Commands`] instance that pushes to the world's command queue
    #[inline]
    pub fn commands(&mut self) -> Commands {
        // SAFETY: &mut self ensure that there are no outstanding accesses to the queue
        let queue = unsafe { self.world.get_command_queue() };
        Commands::new_from_entities(queue, self.world.entities())
    }

    /// Retrieves a mutable reference to the given `entity`'s [`Component`] of the given type.
    /// Returns `None` if the `entity` does not have a [`Component`] of the given type.
    #[inline]
    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<Mut<T>> {
        // SAFETY: &mut self ensure that there are no outstanding accesses to the component
        unsafe { self.world.get_entity(entity)?.get_mut() }
    }

    /// Retrieves an [`EntityMut`] that exposes read and write operations for the given `entity`.
    /// This will panic if the `entity` does not exist. Use [`Self::get_entity_mut`] if you want
    /// to check for entity existence instead of implicitly panic-ing.
    #[inline]
    #[track_caller]
    pub fn entity_mut(&mut self, entity: Entity) -> EntityMut {
        #[inline(never)]
        #[cold]
        #[track_caller]
        fn panic_no_entity(entity: Entity) -> ! {
            panic!("Entity {entity:?} does not exist");
        }

        match self.get_entity_mut(entity) {
            Some(entity) => entity,
            None => panic_no_entity(entity),
        }
    }

    /// Retrieves an [`EntityMut`] that exposes read and write operations for the given `entity`.
    /// Returns [`None`] if the `entity` does not exist.
    /// Instead of unwrapping the value returned from this function, prefer [`Self::entity_mut`].
    #[inline]
    pub fn get_entity_mut(&mut self, entity: Entity) -> Option<EntityMut> {
        let location = self.entities.get(entity)?;
        // SAFETY: `entity` exists and `location` is that entity's location
        Some(unsafe { EntityMut::new(UnsafeEntityCell::new(self.world, entity, location)) })
    }

    /// Returns [`Query`] for the given [`QueryState`], which is used to efficiently
    /// run queries on the [`World`] by storing and reusing the [`QueryState`].
    ///
    /// # Panics
    /// If state is from a different world then self
    #[inline]
    pub fn query<'s, D: QueryData, F: QueryFilter>(
        &'w mut self,
        state: &'s mut QueryState<D, F>,
    ) -> Query<'w, 's, D, F> {
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
                std::any::type_name::<R>()
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
                Non-send resources can also be be added by plugins.",
                std::any::type_name::<R>()
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
        self.send_event_batch(std::iter::once(event))?.next()
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
                std::any::type_name::<E>()
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
        entity: Entity,
        targets: impl Iterator<Item = ComponentId>,
    ) {
        for component_id in targets {
            // SAFETY: Caller ensures that these components exist
            let hooks = unsafe { self.components().get_info_unchecked(component_id) }.hooks();
            if let Some(hook) = hooks.on_add {
                hook(DeferredWorld { world: self.world }, entity, component_id);
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
        entity: Entity,
        targets: impl Iterator<Item = ComponentId>,
    ) {
        for component_id in targets {
            // SAFETY: Caller ensures that these components exist
            let hooks = unsafe { self.world.components().get_info_unchecked(component_id) }.hooks();
            if let Some(hook) = hooks.on_insert {
                hook(DeferredWorld { world: self.world }, entity, component_id);
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
        entity: Entity,
        targets: impl Iterator<Item = ComponentId>,
    ) {
        for component_id in targets {
            // SAFETY: Caller ensures that these components exist
            let hooks = unsafe { self.world.components().get_info_unchecked(component_id) }.hooks();
            if let Some(hook) = hooks.on_remove {
                hook(DeferredWorld { world: self.world }, entity, component_id);
            }
        }
    }
}
