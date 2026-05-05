use core::{cell::UnsafeCell, panic::Location};

use bevy_platform::cell::SyncUnsafeCell;
use bevy_ptr::{OwningPtr, Ptr};
use nonmax::NonMaxU32;

use crate::{
    change_detection::{CheckChangeTicks, ComponentTickCells, ComponentTicks, MaybeLocation, Tick},
    component::{ComponentId, ComponentInfo},
    entity::Entity,
    storage::{Column, SparseSet, TableRow},
};

/// A collection of resource storages, indexed by [`ComponentId`]
///
/// Can be accessed via [`Storages`](crate::storage::Storages)
#[derive(Default)]
pub struct ResourceStorages {
    /// Column is always one element long.
    resources: SparseSet<ComponentId, ResourceStorage>,
}

impl ResourceStorages {
    pub(crate) fn init(&mut self, component_info: &ComponentInfo) {
        self.resources
            .get_or_insert_with(component_info.id(), || ResourceStorage::new(component_info));
    }

    /// Gets a reference to the [`ResourceStorage`] of a [`ComponentId`].
    /// This may be `None` if the component has never been spawned.
    pub fn get(&self, component_id: ComponentId) -> Option<&ResourceStorage> {
        self.resources.get(component_id)
    }

    /// Gets a reference to the [`ResourceStorage`] of a [`ComponentId`].
    /// This may be `None` if the component has never been spawned.
    pub fn get_mut(&mut self, component_id: ComponentId) -> Option<&mut ResourceStorage> {
        self.resources.get_mut(component_id)
    }

    /// Iterate all resources.
    pub fn iter(&self) -> impl Iterator<Item = (ComponentId, Entity)> {
        self.resources
            .iter()
            .filter_map(|(&id, storage)| match storage.state {
                Populated(entity) => Some((id, entity)),
                _ => None,
            })
    }

    pub(crate) fn check_change_ticks(&mut self, check: CheckChangeTicks) {
        for storage in self.resources.values_mut() {
            storage.check_change_ticks(check);
        }
    }

    /// Clears all resource entities
    ///
    /// # Panics
    /// - Panics if any of the components stored within implement [`Drop`] and any of them panic.
    pub(crate) fn clear_entities(&mut self) {
        for storage in self.resources.values_mut() {
            if let Some(entity) = storage.entity() {
                storage.remove(entity);
            }
        }
    }
}

const ROW: TableRow = TableRow::new(NonMaxU32::ZERO);

enum ResourceState {
    /// There is no entity assigned to hold this resource.
    /// This can be because the resource hasn't been inserted so far,
    /// or because `IsResource` has been removed
    /// (possibly because the entity has been despawned).
    NoEntity,
    /// There is an entity assigned to hold this resource, but the
    /// resource currently isn't present in the world.
    Unpopulated(Entity),
    /// The resource currently is present in the world.
    Populated(Entity),
}

use ResourceState::*;

/// Storage for an individual resource.
pub struct ResourceStorage {
    state: ResourceState,
    /// When the resource already exists but gets inserted into a different
    /// entity, we discard the new insertion. The storage itself can't
    /// stop the archetype move, but marks the failed insert;
    /// the `IsResource::on_insert` hook then moves the entity back into
    /// the archetype without the resource and clears the flag.
    insert_just_failed: SyncUnsafeCell<bool>,
    /// capacity: 1
    /// length: 1 if populated, 0 otherwise
    data: Column,
}

impl ResourceStorage {
    fn new(component_info: &ComponentInfo) -> Self {
        Self {
            state: NoEntity,
            insert_just_failed: SyncUnsafeCell::new(false),
            data: Column::with_capacity(component_info, 1),
        }
    }

    /// Returns the entity responsible for holding this resource,
    /// even if the resource doesn't currently exist in the world.
    ///
    /// Can return `None` if the resource has never been inserted before,
    /// or has been despawned.
    pub fn entity(&self) -> Option<Entity> {
        match self.state {
            NoEntity => None,
            Unpopulated(entity) | Populated(entity) => Some(entity),
        }
    }

    /// Returns whether the given resource exists.
    pub fn populated(&self) -> bool {
        match self.state {
            NoEntity | Unpopulated(_) => false,
            Populated(_) => true,
        }
    }

    /// Inserts the component `value` into this resource storage.
    ///
    /// Will fail if another entity already has this resource.
    ///
    /// # Safety
    /// The `value` pointer must point to a valid address that matches the [`Layout`](std::alloc::Layout)
    /// inside the [`ComponentInfo`] given when constructing this sparse set.
    pub(crate) unsafe fn insert(
        &mut self,
        entity: Entity,
        value: OwningPtr<'_>,
        change_tick: Tick,
        caller: MaybeLocation,
    ) {
        match self.state {
            NoEntity | Unpopulated(_) => {
                self.state = Populated(entity);
                self.data.initialize(ROW, value, change_tick, caller);
            }
            Populated(existing_entity) if existing_entity == entity => {
                self.data.replace(ROW, value, change_tick, caller);
            }
            Populated(_) => {
                if let Some(drop) = self.get_drop() {
                    // SAFETY: Drop function came from value's component descriptor
                    unsafe {
                        drop(value);
                    }
                }
                self.insert_just_failed = SyncUnsafeCell::new(true);
            }
        }
    }

    /// # SAFETY
    /// Must have exclusive access to this storage
    pub(crate) unsafe fn check_and_clear_failed_insert(&self) -> bool {
        // SAFETY: No other references
        unsafe { self.insert_just_failed.get().replace(false) }
    }

    /// Returns a reference to the entity's component value.
    ///
    /// Returns `None` if this entity doesn't have this component.
    #[inline]
    pub fn get_with_entity(&self, entity: Entity) -> Option<Ptr<'_>> {
        match self.state {
            Populated(existing) if existing == entity => Some(
                // SAFETY: length is 1
                unsafe { self.data.get_data_unchecked(ROW) },
            ),
            _ => None,
        }
    }

    /// Returns a reference to the resource's component value.
    ///
    /// Returns `None` if no entity has this component.
    #[inline]
    pub fn get(&self) -> Option<Ptr<'_>> {
        match self.state {
            Populated(_) => Some(
                // SAFETY: length is 1
                unsafe { self.data.get_data_unchecked(ROW) },
            ),
            _ => None,
        }
    }

    /// Returns references to the entity's component value and its added and changed ticks.
    ///
    /// Returns `None` if no entity has this component.
    #[inline]
    pub fn get_with_ticks(&self) -> Option<(Ptr<'_>, ComponentTickCells<'_>)> {
        match self.state {
            NoEntity | Unpopulated(_) => None,
            Populated(_) => Some(
                // SAFETY: length is 1
                unsafe {
                    (
                        self.data.get_data_unchecked(ROW),
                        ComponentTickCells {
                            added: self.data.get_added_tick_unchecked(ROW),
                            changed: self.data.get_changed_tick_unchecked(ROW),
                            changed_by: self.data.get_changed_by_unchecked(ROW),
                        },
                    )
                },
            ),
        }
    }

    /// Returns a reference to the "added" tick of the entity's component value.
    ///
    /// Returns `None` if no entity has this component.
    #[inline]
    pub fn get_added_tick(&self) -> Option<&UnsafeCell<Tick>> {
        match self.state {
            NoEntity | Unpopulated(_) => None,
            Populated(_) => Some(
                // SAFETY: length is 1
                unsafe { self.data.get_added_tick_unchecked(ROW) },
            ),
        }
    }

    /// Returns a reference to the "changed" tick of the entity's component value.
    ///
    /// Returns `None` if no entity has this component.
    #[inline]
    pub fn get_changed_tick(&self) -> Option<&UnsafeCell<Tick>> {
        match self.state {
            NoEntity | Unpopulated(_) => None,
            Populated(_) => Some(
                // SAFETY: length is 1
                unsafe { self.data.get_changed_tick_unchecked(ROW) },
            ),
        }
    }

    /// Returns a reference to the "added" and "changed" ticks of the entity's component value.
    ///
    /// Returns `None` if no entity has this component.
    #[inline]
    pub fn get_ticks(&self) -> Option<ComponentTicks> {
        match self.state {
            NoEntity | Unpopulated(_) => None,
            Populated(_) => Some(
                // SAFETY: length is 1
                unsafe { self.data.get_ticks_unchecked(ROW) },
            ),
        }
    }

    /// Returns a reference to the calling location that last changed the entity's component value.
    ///
    /// Returns `None` if no entity has this component.
    #[inline]
    pub fn get_changed_by(&self) -> MaybeLocation<Option<&UnsafeCell<&'static Location<'static>>>> {
        MaybeLocation::new_with_flattened(|| {
            match self.state {
                NoEntity | Unpopulated(_) => None,
                Populated(_) => Some(
                    // SAFETY: length is 1
                    unsafe { self.data.get_changed_by_unchecked(ROW) },
                ),
            }
        })
    }

    /// Returns the drop function for the component type stored in the sparse set,
    /// or `None` if it doesn't need to be dropped.
    #[inline]
    pub fn get_drop(&self) -> Option<unsafe fn(OwningPtr<'_>)> {
        self.data.get_drop()
    }

    /// Removes (and drops) the entity's component value from the sparse set.
    ///
    /// Returns `true` if `entity` had a component value in the sparse set.
    pub(crate) fn remove(&mut self, entity: Entity) -> bool {
        match self.state {
            Populated(existing_entity) if existing_entity == entity => {
                self.state = Unpopulated(existing_entity);
                // SAFETY: Value is being removed
                unsafe {
                    self.data.drop_last_component(0);
                }
                true
            }
            _ => false,
        }
    }

    /// Removes the resource and returns a pointer to the associated value (if it exists).
    #[must_use = "The returned pointer must be used to drop the removed component."]
    pub(crate) fn remove_and_forget(&mut self, entity: Entity) -> Option<OwningPtr<'_>> {
        match self.state {
            Populated(existing_entity) if existing_entity == entity => {
                self.state = Unpopulated(existing_entity);
                // SAFETY: Value is being removed
                Some(unsafe { OwningPtr::new(self.data.get_data_unchecked(ROW).into()) })
            }
            _ => None,
        }
    }

    pub(crate) fn check_change_ticks(&mut self, check: CheckChangeTicks) {
        if matches!(self.state, Populated(_)) {
            // SAFETY: Data has one element
            unsafe { self.data.check_change_ticks(1, check) };
        }
    }

    pub(crate) fn clear_entity_association(&mut self, entity: Entity) {
        if let Unpopulated(existing_entity) = self.state
            && (existing_entity == entity)
        {
            self.state = NoEntity;
        }
    }
}

impl Drop for ResourceStorage {
    fn drop(&mut self) {
        // SAFETY:
        // `cap` and `len` always as specified on `data` doc comment.
        // `data` is never accessed again after this call.
        unsafe {
            self.data.drop(
                1,
                match self.state {
                    Populated(_) => 1,
                    _ => 0,
                },
            );
        }
    }
}
