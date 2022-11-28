use crate::{
    archetype::{Archetype, ArchetypeId, Archetypes},
    bundle::{Bundle, BundleInfo},
    change_detection::{MutUntyped, Ticks},
    component::{Component, ComponentId, ComponentTicks, Components, StorageType, TickCells},
    entity::{Entities, Entity, EntityLocation},
    storage::{SparseSet, Storages},
    world::{Mut, World},
};
use bevy_ptr::{OwningPtr, Ptr};
use bevy_utils::tracing::debug;
use std::any::TypeId;

/// A read-only reference to a particular [`Entity`] and all of its components
#[derive(Copy, Clone)]
pub struct EntityRef<'w> {
    world: &'w World,
    entity: Entity,
    location: EntityLocation,
}

impl<'w> EntityRef<'w> {
    #[inline]
    pub(crate) fn new(world: &'w World, entity: Entity, location: EntityLocation) -> Self {
        Self {
            world,
            entity,
            location,
        }
    }

    #[inline]
    #[must_use = "Omit the .id() call if you do not need to store the `Entity` identifier."]
    pub fn id(&self) -> Entity {
        self.entity
    }

    #[inline]
    pub fn location(&self) -> EntityLocation {
        self.location
    }

    #[inline]
    pub fn archetype(&self) -> &Archetype {
        &self.world.archetypes[self.location.archetype_id]
    }

    #[inline]
    pub fn world(&self) -> &'w World {
        self.world
    }

    #[inline]
    pub fn contains<T: Component>(&self) -> bool {
        self.contains_type_id(TypeId::of::<T>())
    }

    #[inline]
    pub fn contains_id(&self, component_id: ComponentId) -> bool {
        contains_component_with_id(self.world, component_id, self.location)
    }

    #[inline]
    pub fn contains_type_id(&self, type_id: TypeId) -> bool {
        contains_component_with_type(self.world, type_id, self.location)
    }

    #[inline]
    pub fn get<T: Component>(&self) -> Option<&'w T> {
        // SAFETY: entity location is valid and returned component is of type T
        unsafe {
            get_component_with_type(self.world, TypeId::of::<T>(), self.entity, self.location)
                .map(|value| value.deref::<T>())
        }
    }

    /// Retrieves the change ticks for the given component. This can be useful for implementing change
    /// detection in custom runtimes.
    #[inline]
    pub fn get_change_ticks<T: Component>(&self) -> Option<ComponentTicks> {
        // SAFETY: entity location is valid
        unsafe { get_ticks_with_type(self.world, TypeId::of::<T>(), self.entity, self.location) }
    }

    /// Gets a mutable reference to the component of type `T` associated with
    /// this entity without ensuring there are no other borrows active and without
    /// ensuring that the returned reference will stay valid.
    ///
    /// # Safety
    ///
    /// - The returned reference must never alias a mutable borrow of this component.
    /// - The returned reference must not be used after this component is moved which
    ///   may happen from **any** `insert_component`, `remove_component` or `despawn`
    ///   operation on this world (non-exhaustive list).
    #[inline]
    pub unsafe fn get_unchecked_mut<T: Component>(
        &self,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Option<Mut<'w, T>> {
        get_component_and_ticks_with_type(self.world, TypeId::of::<T>(), self.entity, self.location)
            .map(|(value, ticks)| Mut {
                value: value.assert_unique().deref_mut::<T>(),
                ticks: Ticks::from_tick_cells(ticks, last_change_tick, change_tick),
            })
    }
}

impl<'w> EntityRef<'w> {
    /// Gets the component of the given [`ComponentId`] from the entity.
    ///
    /// **You should prefer to use the typed API where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    ///
    /// Unlike [`EntityRef::get`], this returns a raw pointer to the component,
    /// which is only valid while the `'w` borrow of the lifetime is active.
    #[inline]
    pub fn get_by_id(&self, component_id: ComponentId) -> Option<Ptr<'w>> {
        self.world.components().get_info(component_id)?;
        // SAFETY: entity_location is valid, component_id is valid as checked by the line above
        unsafe { get_component(self.world, component_id, self.entity, self.location) }
    }
}

impl<'w> From<EntityMut<'w>> for EntityRef<'w> {
    fn from(entity_mut: EntityMut<'w>) -> EntityRef<'w> {
        EntityRef::new(entity_mut.world, entity_mut.entity, entity_mut.location)
    }
}

/// A mutable reference to a particular [`Entity`] and all of its components
pub struct EntityMut<'w> {
    world: &'w mut World,
    entity: Entity,
    location: EntityLocation,
}

impl<'w> EntityMut<'w> {
    /// # Safety
    /// entity and location _must_ be valid
    #[inline]
    pub(crate) unsafe fn new(
        world: &'w mut World,
        entity: Entity,
        location: EntityLocation,
    ) -> Self {
        EntityMut {
            world,
            entity,
            location,
        }
    }

    #[inline]
    #[must_use = "Omit the .id() call if you do not need to store the `Entity` identifier."]
    pub fn id(&self) -> Entity {
        self.entity
    }

    #[inline]
    pub fn location(&self) -> EntityLocation {
        self.location
    }

    #[inline]
    pub fn archetype(&self) -> &Archetype {
        &self.world.archetypes[self.location.archetype_id]
    }

    #[inline]
    pub fn contains<T: Component>(&self) -> bool {
        self.contains_type_id(TypeId::of::<T>())
    }

    #[inline]
    pub fn contains_id(&self, component_id: ComponentId) -> bool {
        contains_component_with_id(self.world, component_id, self.location)
    }

    #[inline]
    pub fn contains_type_id(&self, type_id: TypeId) -> bool {
        contains_component_with_type(self.world, type_id, self.location)
    }

    #[inline]
    pub fn get<T: Component>(&self) -> Option<&'_ T> {
        // SAFETY: lifetimes enforce correct usage of returned borrow
        unsafe {
            get_component_with_type(self.world, TypeId::of::<T>(), self.entity, self.location)
                .map(|value| value.deref::<T>())
        }
    }

    #[inline]
    pub fn get_mut<T: Component>(&mut self) -> Option<Mut<'_, T>> {
        // SAFETY: world access is unique, and lifetimes enforce correct usage of returned borrow
        unsafe { self.get_unchecked_mut::<T>() }
    }

    /// Retrieves the change ticks for the given component. This can be useful for implementing change
    /// detection in custom runtimes.
    #[inline]
    pub fn get_change_ticks<T: Component>(&self) -> Option<ComponentTicks> {
        // SAFETY: entity location is valid
        unsafe { get_ticks_with_type(self.world, TypeId::of::<T>(), self.entity, self.location) }
    }

    /// Gets a mutable reference to the component of type `T` associated with
    /// this entity without ensuring there are no other borrows active and without
    /// ensuring that the returned reference will stay valid.
    ///
    /// # Safety
    ///
    /// - The returned reference must never alias a mutable borrow of this component.
    /// - The returned reference must not be used after this component is moved which
    ///   may happen from **any** `insert_component`, `remove_component` or `despawn`
    ///   operation on this world (non-exhaustive list).
    #[inline]
    pub unsafe fn get_unchecked_mut<T: Component>(&self) -> Option<Mut<'_, T>> {
        get_component_and_ticks_with_type(self.world, TypeId::of::<T>(), self.entity, self.location)
            .map(|(value, ticks)| Mut {
                value: value.assert_unique().deref_mut::<T>(),
                ticks: Ticks::from_tick_cells(
                    ticks,
                    self.world.last_change_tick(),
                    self.world.read_change_tick(),
                ),
            })
    }

    #[deprecated(
        since = "0.9.0",
        note = "Use `insert` instead, which now accepts bundles, components, and tuples of bundles and components."
    )]
    pub fn insert_bundle<T: Bundle>(&mut self, bundle: T) -> &mut Self {
        self.insert(bundle)
    }

    /// Adds a [`Bundle`] of components to the entity.
    ///
    /// This will overwrite any previous value(s) of the same component type.
    pub fn insert<T: Bundle>(&mut self, bundle: T) -> &mut Self {
        let change_tick = self.world.change_tick();
        let bundle_info = self
            .world
            .bundles
            .init_info::<T>(&mut self.world.components, &mut self.world.storages);
        let mut bundle_inserter = bundle_info.get_bundle_inserter(
            &mut self.world.entities,
            &mut self.world.archetypes,
            &mut self.world.components,
            &mut self.world.storages,
            self.location.archetype_id,
            change_tick,
        );
        // SAFETY: location matches current entity. `T` matches `bundle_info`
        unsafe {
            self.location = bundle_inserter.insert(self.entity, self.location.index, bundle);
        }

        self
    }

    #[deprecated(
        since = "0.9.0",
        note = "Use `remove` instead, which now accepts bundles, components, and tuples of bundles and components."
    )]
    pub fn remove_bundle<T: Bundle>(&mut self) -> Option<T> {
        self.remove::<T>()
    }

    // TODO: move to BundleInfo
    /// Removes a [`Bundle`] of components from the entity and returns the bundle.
    ///
    /// Returns `None` if the entity does not contain the bundle.
    pub fn remove<T: Bundle>(&mut self) -> Option<T> {
        let archetypes = &mut self.world.archetypes;
        let storages = &mut self.world.storages;
        let components = &mut self.world.components;
        let entities = &mut self.world.entities;
        let removed_components = &mut self.world.removed_components;

        let bundle_info = self.world.bundles.init_info::<T>(components, storages);
        let old_location = self.location;
        // SAFETY: `archetype_id` exists because it is referenced in the old `EntityLocation` which is valid,
        // components exist in `bundle_info` because `Bundles::init_info` initializes a `BundleInfo` containing all components of the bundle type `T`
        let new_archetype_id = unsafe {
            remove_bundle_from_archetype(
                archetypes,
                storages,
                components,
                old_location.archetype_id,
                bundle_info,
                false,
            )?
        };

        if new_archetype_id == old_location.archetype_id {
            return None;
        }

        let old_archetype = &mut archetypes[old_location.archetype_id];
        let mut bundle_components = bundle_info.component_ids.iter().cloned();
        let entity = self.entity;
        // SAFETY: bundle components are iterated in order, which guarantees that the component type
        // matches
        let result = unsafe {
            T::from_components(storages, &mut |storages| {
                let component_id = bundle_components.next().unwrap();
                // SAFETY: entity location is valid and table row is removed below
                take_component(
                    components,
                    storages,
                    old_archetype,
                    removed_components,
                    component_id,
                    entity,
                    old_location,
                )
            })
        };

        #[allow(clippy::undocumented_unsafe_blocks)] // TODO: document why this is safe
        unsafe {
            Self::move_entity_from_remove::<false>(
                entity,
                &mut self.location,
                old_location.archetype_id,
                old_location,
                entities,
                archetypes,
                storages,
                new_archetype_id,
            );
        }

        Some(result)
    }

    /// Safety: `new_archetype_id` must have the same or a subset of the components
    /// in `old_archetype_id`. Probably more safety stuff too, audit a call to
    /// this fn as if the code here was written inline
    ///
    /// when DROP is true removed components will be dropped otherwise they will be forgotten
    ///
    // We use a const generic here so that we are less reliant on
    // inlining for rustc to optimize out the `match DROP`
    #[allow(clippy::too_many_arguments)]
    unsafe fn move_entity_from_remove<const DROP: bool>(
        entity: Entity,
        self_location: &mut EntityLocation,
        old_archetype_id: ArchetypeId,
        old_location: EntityLocation,
        entities: &mut Entities,
        archetypes: &mut Archetypes,
        storages: &mut Storages,
        new_archetype_id: ArchetypeId,
    ) {
        let old_archetype = &mut archetypes[old_archetype_id];
        let remove_result = old_archetype.swap_remove(old_location.index);
        if let Some(swapped_entity) = remove_result.swapped_entity {
            entities.meta[swapped_entity.index as usize].location = old_location;
        }
        let old_table_row = remove_result.table_row;
        let old_table_id = old_archetype.table_id();
        let new_archetype = &mut archetypes[new_archetype_id];

        let new_location = if old_table_id == new_archetype.table_id() {
            new_archetype.allocate(entity, old_table_row)
        } else {
            let (old_table, new_table) = storages
                .tables
                .get_2_mut(old_table_id, new_archetype.table_id());

            // SAFETY: old_table_row exists
            let move_result = if DROP {
                old_table.move_to_and_drop_missing_unchecked(old_table_row, new_table)
            } else {
                old_table.move_to_and_forget_missing_unchecked(old_table_row, new_table)
            };

            // SAFETY: move_result.new_row is a valid position in new_archetype's table
            let new_location = new_archetype.allocate(entity, move_result.new_row);

            // if an entity was moved into this entity's table spot, update its table row
            if let Some(swapped_entity) = move_result.swapped_entity {
                let swapped_location = entities.get(swapped_entity).unwrap();
                archetypes[swapped_location.archetype_id]
                    .set_entity_table_row(swapped_location.index, old_table_row);
            }

            new_location
        };

        *self_location = new_location;
        entities.meta[entity.index as usize].location = new_location;
    }

    #[deprecated(
        since = "0.9.0",
        note = "Use `remove_intersection` instead, which now accepts bundles, components, and tuples of bundles and components."
    )]
    pub fn remove_bundle_intersection<T: Bundle>(&mut self) {
        self.remove_intersection::<T>();
    }

    // TODO: move to BundleInfo
    /// Remove any components in the bundle that the entity has.
    pub fn remove_intersection<T: Bundle>(&mut self) {
        let archetypes = &mut self.world.archetypes;
        let storages = &mut self.world.storages;
        let components = &mut self.world.components;
        let entities = &mut self.world.entities;
        let removed_components = &mut self.world.removed_components;

        let bundle_info = self.world.bundles.init_info::<T>(components, storages);
        let old_location = self.location;

        // SAFETY: `archetype_id` exists because it is referenced in the old `EntityLocation` which is valid,
        // components exist in `bundle_info` because `Bundles::init_info` initializes a `BundleInfo` containing all components of the bundle type `T`
        let new_archetype_id = unsafe {
            remove_bundle_from_archetype(
                archetypes,
                storages,
                components,
                old_location.archetype_id,
                bundle_info,
                true,
            )
            .expect("intersections should always return a result")
        };

        if new_archetype_id == old_location.archetype_id {
            return;
        }

        let old_archetype = &mut archetypes[old_location.archetype_id];
        let entity = self.entity;
        for component_id in bundle_info.component_ids.iter().cloned() {
            if old_archetype.contains(component_id) {
                removed_components
                    .get_or_insert_with(component_id, Vec::new)
                    .push(entity);

                // Make sure to drop components stored in sparse sets.
                // Dense components are dropped later in `move_to_and_drop_missing_unchecked`.
                if let Some(StorageType::SparseSet) = old_archetype.get_storage_type(component_id) {
                    storages
                        .sparse_sets
                        .get_mut(component_id)
                        .unwrap()
                        .remove(entity);
                }
            }
        }

        #[allow(clippy::undocumented_unsafe_blocks)] // TODO: document why this is safe
        unsafe {
            Self::move_entity_from_remove::<true>(
                entity,
                &mut self.location,
                old_location.archetype_id,
                old_location,
                entities,
                archetypes,
                storages,
                new_archetype_id,
            );
        }
    }

    pub fn despawn(self) {
        debug!("Despawning entity {:?}", self.entity);
        let world = self.world;
        world.flush();
        let location = world
            .entities
            .free(self.entity)
            .expect("entity should exist at this point.");
        let table_row;
        let moved_entity;
        {
            let archetype = &mut world.archetypes[location.archetype_id];
            for component_id in archetype.components() {
                let removed_components = world
                    .removed_components
                    .get_or_insert_with(component_id, Vec::new);
                removed_components.push(self.entity);
            }
            let remove_result = archetype.swap_remove(location.index);
            if let Some(swapped_entity) = remove_result.swapped_entity {
                world.entities.meta[swapped_entity.index as usize].location = location;
            }
            table_row = remove_result.table_row;

            for component_id in archetype.sparse_set_components() {
                let sparse_set = world.storages.sparse_sets.get_mut(component_id).unwrap();
                sparse_set.remove(self.entity);
            }
            // SAFETY: table rows stored in archetypes always exist
            moved_entity = unsafe {
                world.storages.tables[archetype.table_id()].swap_remove_unchecked(table_row)
            };
        };

        if let Some(moved_entity) = moved_entity {
            let moved_location = world.entities.get(moved_entity).unwrap();
            world.archetypes[moved_location.archetype_id]
                .set_entity_table_row(moved_location.index, table_row);
        }
    }

    #[inline]
    pub fn world(&self) -> &World {
        self.world
    }

    /// Returns this `EntityMut`'s world.
    ///
    /// See [`EntityMut::world_scope`] or [`EntityMut::into_world_mut`] for a safe alternative.
    ///
    /// # Safety
    /// Caller must not modify the world in a way that changes the current entity's location
    /// If the caller _does_ do something that could change the location, `self.update_location()`
    /// must be called before using any other methods on this [`EntityMut`].
    #[inline]
    pub unsafe fn world_mut(&mut self) -> &mut World {
        self.world
    }

    /// Return this `EntityMut`'s [`World`], consuming itself.
    #[inline]
    pub fn into_world_mut(self) -> &'w mut World {
        self.world
    }

    /// Gives mutable access to this `EntityMut`'s [`World`] in a temporary scope.
    pub fn world_scope(&mut self, f: impl FnOnce(&mut World)) {
        f(self.world);
        self.update_location();
    }

    /// Updates the internal entity location to match the current location in the internal
    /// [`World`]. This is only needed if the user called [`EntityMut::world`], which enables the
    /// location to change.
    pub fn update_location(&mut self) {
        self.location = self.world.entities().get(self.entity).unwrap();
    }
}

impl<'w> EntityMut<'w> {
    /// Gets the component of the given [`ComponentId`] from the entity.
    ///
    /// **You should prefer to use the typed API [`EntityMut::get`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    ///
    /// Unlike [`EntityMut::get`], this returns a raw pointer to the component,
    /// which is only valid while the [`EntityMut`] is alive.
    #[inline]
    pub fn get_by_id(&self, component_id: ComponentId) -> Option<Ptr<'_>> {
        self.world.components().get_info(component_id)?;
        // SAFETY: entity_location is valid, component_id is valid as checked by the line above
        unsafe { get_component(self.world, component_id, self.entity, self.location) }
    }

    /// Gets a [`MutUntyped`] of the component of the given [`ComponentId`] from the entity.
    ///
    /// **You should prefer to use the typed API [`EntityMut::get_mut`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    ///
    /// Unlike [`EntityMut::get_mut`], this returns a raw pointer to the component,
    /// which is only valid while the [`EntityMut`] is alive.
    #[inline]
    pub fn get_mut_by_id(&mut self, component_id: ComponentId) -> Option<MutUntyped<'_>> {
        self.world.components().get_info(component_id)?;
        // SAFETY: entity_location is valid, component_id is valid as checked by the line above
        unsafe { get_mut_by_id(self.world, self.entity, self.location, component_id) }
    }
}

// TODO: move to Storages?
/// Get a raw pointer to a particular [`Component`] on a particular [`Entity`] in the provided [`World`].
///
/// # Safety
/// - `entity_location` must be within bounds of the given archetype and `entity` must exist inside
/// the archetype
/// - `component_id` must be valid
#[inline]
pub(crate) unsafe fn get_component(
    world: &World,
    component_id: ComponentId,
    entity: Entity,
    location: EntityLocation,
) -> Option<Ptr<'_>> {
    let archetype = &world.archetypes[location.archetype_id];
    // SAFETY: component_id exists and is therefore valid
    let component_info = world.components.get_info_unchecked(component_id);
    match component_info.storage_type() {
        StorageType::Table => {
            let table = &world.storages.tables[archetype.table_id()];
            let components = table.get_column(component_id)?;
            let table_row = archetype.entity_table_row(location.index);
            // SAFETY: archetypes only store valid table_rows and the stored component type is T
            Some(components.get_data_unchecked(table_row))
        }
        StorageType::SparseSet => world
            .storages
            .sparse_sets
            .get(component_id)
            .and_then(|sparse_set| sparse_set.get(entity)),
    }
}

// TODO: move to Storages?
/// Get a raw pointer to the [`ComponentTicks`] of a particular [`Component`] on a particular [`Entity`] in the provided [World].
///
/// # Safety
/// Caller must ensure that `component_id` is valid
#[inline]
unsafe fn get_component_and_ticks(
    world: &World,
    component_id: ComponentId,
    entity: Entity,
    location: EntityLocation,
) -> Option<(Ptr<'_>, TickCells<'_>)> {
    let archetype = &world.archetypes[location.archetype_id];
    let component_info = world.components.get_info_unchecked(component_id);
    match component_info.storage_type() {
        StorageType::Table => {
            let table = &world.storages.tables[archetype.table_id()];
            let components = table.get_column(component_id)?;
            let table_row = archetype.entity_table_row(location.index);
            // SAFETY: archetypes only store valid table_rows and the stored component type is T
            Some((
                components.get_data_unchecked(table_row),
                TickCells {
                    added: components.get_added_ticks_unchecked(table_row),
                    changed: components.get_changed_ticks_unchecked(table_row),
                },
            ))
        }
        StorageType::SparseSet => world
            .storages
            .sparse_sets
            .get(component_id)
            .and_then(|sparse_set| sparse_set.get_with_ticks(entity)),
    }
}

#[inline]
unsafe fn get_ticks(
    world: &World,
    component_id: ComponentId,
    entity: Entity,
    location: EntityLocation,
) -> Option<ComponentTicks> {
    let archetype = &world.archetypes[location.archetype_id];
    let component_info = world.components.get_info_unchecked(component_id);
    match component_info.storage_type() {
        StorageType::Table => {
            let table = &world.storages.tables[archetype.table_id()];
            let components = table.get_column(component_id)?;
            let table_row = archetype.entity_table_row(location.index);
            // SAFETY: archetypes only store valid table_rows and the stored component type is T
            Some(components.get_ticks_unchecked(table_row))
        }
        StorageType::SparseSet => world
            .storages
            .sparse_sets
            .get(component_id)
            .and_then(|sparse_set| sparse_set.get_ticks(entity)),
    }
}

// TODO: move to Storages?
/// Moves component data out of storage.
///
/// This function leaves the underlying memory unchanged, but the component behind
/// returned pointer is semantically owned by the caller and will not be dropped in its original location.
/// Caller is responsible to drop component data behind returned pointer.
///
/// # Safety
/// - `entity_location` must be within bounds of the given archetype and `entity` must exist inside the archetype
/// - `component_id` must be valid
/// - The relevant table row **must be removed** by the caller once all components are taken
#[inline]
unsafe fn take_component<'a>(
    components: &Components,
    storages: &'a mut Storages,
    archetype: &Archetype,
    removed_components: &mut SparseSet<ComponentId, Vec<Entity>>,
    component_id: ComponentId,
    entity: Entity,
    location: EntityLocation,
) -> OwningPtr<'a> {
    let component_info = components.get_info_unchecked(component_id);
    let removed_components = removed_components.get_or_insert_with(component_id, Vec::new);
    removed_components.push(entity);
    match component_info.storage_type() {
        StorageType::Table => {
            let table = &mut storages.tables[archetype.table_id()];
            // SAFETY: archetypes will always point to valid columns
            let components = table.get_column_mut(component_id).unwrap();
            let table_row = archetype.entity_table_row(location.index);
            // SAFETY: archetypes only store valid table_rows and the stored component type is T
            components.get_data_unchecked_mut(table_row).promote()
        }
        StorageType::SparseSet => storages
            .sparse_sets
            .get_mut(component_id)
            .unwrap()
            .remove_and_forget(entity)
            .unwrap(),
    }
}

/// Get a raw pointer to a particular [`Component`] by [`TypeId`] on a particular [`Entity`] in the provided [`World`].
///
/// # Safety
/// `entity_location` must be within bounds of an archetype that exists.
unsafe fn get_component_with_type(
    world: &World,
    type_id: TypeId,
    entity: Entity,
    location: EntityLocation,
) -> Option<Ptr<'_>> {
    let component_id = world.components.get_id(type_id)?;
    get_component(world, component_id, entity, location)
}

/// Get a raw pointer to the [`ComponentTicks`] of a particular [`Component`] by [`TypeId`] on a particular [`Entity`] in the provided [`World`].
///
/// # Safety
/// `entity_location` must be within bounds of an archetype that exists.
pub(crate) unsafe fn get_component_and_ticks_with_type(
    world: &World,
    type_id: TypeId,
    entity: Entity,
    location: EntityLocation,
) -> Option<(Ptr<'_>, TickCells<'_>)> {
    let component_id = world.components.get_id(type_id)?;
    get_component_and_ticks(world, component_id, entity, location)
}

/// # Safety
/// `entity_location` must be within bounds of an archetype that exists.
pub(crate) unsafe fn get_ticks_with_type(
    world: &World,
    type_id: TypeId,
    entity: Entity,
    location: EntityLocation,
) -> Option<ComponentTicks> {
    let component_id = world.components.get_id(type_id)?;
    get_ticks(world, component_id, entity, location)
}

fn contains_component_with_type(world: &World, type_id: TypeId, location: EntityLocation) -> bool {
    if let Some(component_id) = world.components.get_id(type_id) {
        contains_component_with_id(world, component_id, location)
    } else {
        false
    }
}

fn contains_component_with_id(
    world: &World,
    component_id: ComponentId,
    location: EntityLocation,
) -> bool {
    world.archetypes[location.archetype_id].contains(component_id)
}

/// Removes a bundle from the given archetype and returns the resulting archetype (or None if the
/// removal was invalid). in the event that adding the given bundle does not result in an Archetype
/// change. Results are cached in the Archetype Graph to avoid redundant work.
/// if `intersection` is false, attempting to remove a bundle with components _not_ contained in the
/// current archetype will fail, returning None. if `intersection` is true, components in the bundle
/// but not in the current archetype will be ignored
///
/// # Safety
/// `archetype_id` must exist and components in `bundle_info` must exist
unsafe fn remove_bundle_from_archetype(
    archetypes: &mut Archetypes,
    storages: &mut Storages,
    components: &mut Components,
    archetype_id: ArchetypeId,
    bundle_info: &BundleInfo,
    intersection: bool,
) -> Option<ArchetypeId> {
    // check the archetype graph to see if the Bundle has been removed from this archetype in the
    // past
    let remove_bundle_result = {
        let current_archetype = &mut archetypes[archetype_id];
        if intersection {
            current_archetype
                .edges()
                .get_remove_bundle_intersection(bundle_info.id)
        } else {
            current_archetype.edges().get_remove_bundle(bundle_info.id)
        }
    };
    let result = if let Some(result) = remove_bundle_result {
        // this Bundle removal result is cached. just return that!
        result
    } else {
        let mut next_table_components;
        let mut next_sparse_set_components;
        let next_table_id;
        {
            let current_archetype = &mut archetypes[archetype_id];
            let mut removed_table_components = Vec::new();
            let mut removed_sparse_set_components = Vec::new();
            for component_id in bundle_info.component_ids.iter().cloned() {
                if current_archetype.contains(component_id) {
                    // SAFETY: bundle components were already initialized by bundles.get_info
                    let component_info = components.get_info_unchecked(component_id);
                    match component_info.storage_type() {
                        StorageType::Table => removed_table_components.push(component_id),
                        StorageType::SparseSet => removed_sparse_set_components.push(component_id),
                    }
                } else if !intersection {
                    // a component in the bundle was not present in the entity's archetype, so this
                    // removal is invalid cache the result in the archetype
                    // graph
                    current_archetype
                        .edges_mut()
                        .insert_remove_bundle(bundle_info.id, None);
                    return None;
                }
            }

            // sort removed components so we can do an efficient "sorted remove". archetype
            // components are already sorted
            removed_table_components.sort();
            removed_sparse_set_components.sort();
            next_table_components = current_archetype.table_components().collect();
            next_sparse_set_components = current_archetype.sparse_set_components().collect();
            sorted_remove(&mut next_table_components, &removed_table_components);
            sorted_remove(
                &mut next_sparse_set_components,
                &removed_sparse_set_components,
            );

            next_table_id = if removed_table_components.is_empty() {
                current_archetype.table_id()
            } else {
                // SAFETY: all components in next_table_components exist
                storages
                    .tables
                    .get_id_or_insert(&next_table_components, components)
            };
        }

        let new_archetype_id = archetypes.get_id_or_insert(
            next_table_id,
            next_table_components,
            next_sparse_set_components,
        );
        Some(new_archetype_id)
    };
    let current_archetype = &mut archetypes[archetype_id];
    // cache the result in an edge
    if intersection {
        current_archetype
            .edges_mut()
            .insert_remove_bundle_intersection(bundle_info.id, result);
    } else {
        current_archetype
            .edges_mut()
            .insert_remove_bundle(bundle_info.id, result);
    }
    result
}

fn sorted_remove<T: Eq + Ord + Copy>(source: &mut Vec<T>, remove: &[T]) {
    let mut remove_index = 0;
    source.retain(|value| {
        while remove_index < remove.len() && *value > remove[remove_index] {
            remove_index += 1;
        }

        if remove_index < remove.len() {
            *value != remove[remove_index]
        } else {
            true
        }
    });
}

// SAFETY: EntityLocation must be valid
#[inline]
pub(crate) unsafe fn get_mut<T: Component>(
    world: &mut World,
    entity: Entity,
    location: EntityLocation,
) -> Option<Mut<'_, T>> {
    // SAFETY: world access is unique, entity location is valid, and returned component is of type
    // T
    let change_tick = world.change_tick();
    let last_change_tick = world.last_change_tick();
    get_component_and_ticks_with_type(world, TypeId::of::<T>(), entity, location).map(
        |(value, ticks)| Mut {
            value: value.assert_unique().deref_mut::<T>(),
            ticks: Ticks::from_tick_cells(ticks, last_change_tick, change_tick),
        },
    )
}

// SAFETY: EntityLocation must be valid, component_id must be valid
#[inline]
pub(crate) unsafe fn get_mut_by_id(
    world: &mut World,
    entity: Entity,
    location: EntityLocation,
    component_id: ComponentId,
) -> Option<MutUntyped> {
    // SAFETY: world access is unique, entity location and component_id required to be valid
    get_component_and_ticks(world, component_id, entity, location).map(|(value, ticks)| {
        MutUntyped {
            value: value.assert_unique(),
            ticks: Ticks::from_tick_cells(
                ticks,
                world.last_change_tick(),
                world.read_change_tick(),
            ),
        }
    })
}

#[cfg(test)]
mod tests {
    use crate as bevy_ecs;
    use crate::component::ComponentId;
    use crate::prelude::*; // for the `#[derive(Component)]`

    #[test]
    fn sorted_remove() {
        let mut a = vec![1, 2, 3, 4, 5, 6, 7];
        let b = vec![1, 2, 3, 5, 7];
        super::sorted_remove(&mut a, &b);

        assert_eq!(a, vec![4, 6]);

        let mut a = vec![1];
        let b = vec![1];
        super::sorted_remove(&mut a, &b);

        assert_eq!(a, vec![]);

        let mut a = vec![1];
        let b = vec![2];
        super::sorted_remove(&mut a, &b);

        assert_eq!(a, vec![1]);
    }

    #[derive(Component)]
    struct TestComponent(u32);

    #[test]
    fn entity_ref_get_by_id() {
        let mut world = World::new();
        let entity = world.spawn(TestComponent(42)).id();
        let component_id = world
            .components()
            .get_id(std::any::TypeId::of::<TestComponent>())
            .unwrap();

        let entity = world.entity(entity);
        let test_component = entity.get_by_id(component_id).unwrap();
        // SAFETY: points to a valid `TestComponent`
        let test_component = unsafe { test_component.deref::<TestComponent>() };

        assert_eq!(test_component.0, 42);
    }

    #[test]
    fn entity_mut_get_by_id() {
        let mut world = World::new();
        let entity = world.spawn(TestComponent(42)).id();
        let component_id = world
            .components()
            .get_id(std::any::TypeId::of::<TestComponent>())
            .unwrap();

        let mut entity_mut = world.entity_mut(entity);
        let mut test_component = entity_mut.get_mut_by_id(component_id).unwrap();
        {
            test_component.set_changed();
            let test_component =
                // SAFETY: `test_component` has unique access of the `EntityMut` and is not used afterwards
                unsafe { test_component.into_inner().deref_mut::<TestComponent>() };
            test_component.0 = 43;
        }

        let entity = world.entity(entity);
        let test_component = entity.get_by_id(component_id).unwrap();
        // SAFETY: `TestComponent` is the correct component type
        let test_component = unsafe { test_component.deref::<TestComponent>() };

        assert_eq!(test_component.0, 43);
    }

    #[test]
    fn entity_ref_get_by_id_invalid_component_id() {
        let invalid_component_id = ComponentId::new(usize::MAX);

        let mut world = World::new();
        let entity = world.spawn_empty().id();
        let entity = world.entity(entity);
        assert!(entity.get_by_id(invalid_component_id).is_none());
    }

    #[test]
    fn entity_mut_get_by_id_invalid_component_id() {
        let invalid_component_id = ComponentId::new(usize::MAX);

        let mut world = World::new();
        let mut entity = world.spawn_empty();
        assert!(entity.get_by_id(invalid_component_id).is_none());
        assert!(entity.get_mut_by_id(invalid_component_id).is_none());
    }
}
