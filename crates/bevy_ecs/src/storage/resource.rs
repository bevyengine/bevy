use crate::archetype::ArchetypeComponentId;
use crate::component::{ComponentId, ComponentTicks, Components, Tick, TickCells};
use crate::query::DebugCheckedUnwrap;
use crate::storage::{blob_vec::BlobBox, SparseArray};
use bevy_ptr::{OwningPtr, Ptr, UnsafeCellDeref};
use std::cell::UnsafeCell;

/// The type-erased backing storage and metadata for a single resource within a [`World`].
///
/// [`World`]: crate::world::World
pub struct ResourceData {
    data: BlobBox,
    added_tick: UnsafeCell<Tick>,
    changed_tick: UnsafeCell<Tick>,
    id: ArchetypeComponentId,
}

impl ResourceData {
    /// Returns true if the resource is populated.
    #[inline]
    pub fn is_present(&self) -> bool {
        self.data.is_present()
    }

    /// Gets the [`ArchetypeComponentId`] for the resource.
    #[inline]
    pub fn id(&self) -> ArchetypeComponentId {
        self.id
    }

    /// Gets a read-only pointer to the underlying resource, if available.
    #[inline]
    pub fn get_data(&self) -> Option<Ptr<'_>> {
        self.data.get_ptr()
    }

    /// Gets a read-only reference to the change ticks of the underlying resource, if available.
    #[inline]
    pub fn get_ticks(&self) -> Option<ComponentTicks> {
        // SAFETY: If the data is present, the ticks have been written to with valid values
        self.is_present().then(|| unsafe {
            ComponentTicks {
                added: self.added_tick.read(),
                changed: self.changed_tick.read(),
            }
        })
    }

    #[inline]
    pub(crate) fn get_with_ticks(&self) -> Option<(Ptr<'_>, TickCells<'_>)> {
        self.data.get_ptr().map(|ptr| {
            (
                ptr,
                TickCells {
                    added: &self.added_tick,
                    changed: &self.changed_tick,
                },
            )
        })
    }

    /// Inserts a value into the resource. If a value is already present
    /// it will be replaced.
    ///
    /// # Safety
    /// `value` must be valid for the underlying type for the resource.
    ///
    /// The underlying type must be [`Send`] or be inserted from the main thread.
    /// This can be validated with [`World::validate_non_send_access_untyped`].
    ///
    /// [`World::validate_non_send_access_untyped`]: crate::world::World::validate_non_send_access_untyped
    #[inline]
    pub(crate) unsafe fn insert(&mut self, value: OwningPtr<'_>, change_tick: u32) {
        if self.is_present() {
            self.data.replace(value);
        } else {
            self.data.initialize(value);
        }
        let tick = Tick::new(change_tick);
        *self.added_tick.deref_mut() = tick;
        *self.changed_tick.deref_mut() = tick;
    }

    /// Inserts a value into the resource with a pre-existing change tick. If a
    /// value is already present it will be replaced.
    ///
    /// # Safety
    /// `value` must be valid for the underlying type for the resource.
    ///
    /// The underlying type must be [`Send`] or be inserted from the main thread.
    /// This can be validated with [`World::validate_non_send_access_untyped`].
    ///
    /// [`World::validate_non_send_access_untyped`]: crate::world::World::validate_non_send_access_untyped
    #[inline]
    pub(crate) unsafe fn insert_with_ticks(
        &mut self,
        value: OwningPtr<'_>,
        change_ticks: ComponentTicks,
    ) {
        if self.is_present() {
            self.data.replace(value);
        } else {
            self.data.initialize(value);
        }
        *self.added_tick.deref_mut() = change_ticks.added;
        *self.changed_tick.deref_mut() = change_ticks.changed;
    }

    /// Removes a value from the resource, if present.
    ///
    /// # Safety
    /// The underlying type must be [`Send`] or be removed from the main thread.
    /// This can be validated with [`World::validate_non_send_access_untyped`].
    ///
    /// The removed value must be used or dropped.
    ///
    /// [`World::validate_non_send_access_untyped`]: crate::world::World::validate_non_send_access_untyped
    #[inline]
    #[must_use = "The returned pointer to the removed component should be used or dropped"]
    pub(crate) unsafe fn remove(&mut self) -> Option<(OwningPtr<'_>, ComponentTicks)> {
        self.data.remove_and_forget().map(|ptr| {
            (
                ptr,
                ComponentTicks {
                    added: self.added_tick.read(),
                    changed: self.changed_tick.read(),
                },
            )
        })
    }

    /// Removes a value from the resource, if present, and drops it.
    ///
    /// # Safety
    /// The underlying type must be [`Send`] or be removed from the main thread.
    /// This can be validated with [`World::validate_non_send_access_untyped`].
    ///
    /// [`World::validate_non_send_access_untyped`]: crate::world::World::validate_non_send_access_untyped
    #[inline]
    pub(crate) unsafe fn remove_and_drop(&mut self) {
        self.data.clear();
    }

    pub(crate) fn check_change_ticks(&mut self, change_tick: u32) {
        self.added_tick.get_mut().check_tick(change_tick);
        self.changed_tick.get_mut().check_tick(change_tick);
    }
}

/// The backing store for all [`Resource`]s stored in the [`World`].
///
/// [`Resource`]: crate::system::Resource
/// [`World`]: crate::world::World
#[derive(Default)]
pub struct Resources {
    component_ids: Vec<ComponentId>,
    resources: SparseArray<ComponentId, ResourceData>,
}

impl Resources {
    /// The total number of resources stored in the [`World`]
    ///
    /// [`World`]: crate::world::World
    #[inline]
    pub fn len(&self) -> usize {
        self.component_ids.len()
    }

    /// Iterate over all resources that have been initialized, i.e. given a [`ComponentId`]
    pub fn iter(&self) -> impl Iterator<Item = (ComponentId, &ResourceData)> {
        self.component_ids.iter().copied().map(|component_id| {
            // SAFETY: If a component ID is in component_ids, it's populated in the sparse array.
            (component_id, unsafe {
                self.resources.get(component_id).debug_checked_unwrap()
            })
        })
    }

    /// Returns true if there are no resources stored in the [`World`],
    /// false otherwise.
    ///
    /// [`World`]: crate::world::World
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.component_ids.is_empty()
    }

    /// Gets read-only access to a resource, if it exists.
    #[inline]
    pub fn get(&self, component_id: ComponentId) -> Option<&ResourceData> {
        self.resources.get(component_id)
    }

    /// Gets mutable access to a resource, if it exists.
    #[inline]
    pub(crate) fn get_mut(&mut self, component_id: ComponentId) -> Option<&mut ResourceData> {
        self.resources.get_mut(component_id)
    }

    /// Fetches or initializes a new resource and returns back it's underlying column.
    ///
    /// # Panics
    /// Will panic if `component_id` is not valid for the provided `components`
    pub(crate) fn initialize_with(
        &mut self,
        component_id: ComponentId,
        components: &Components,
        f: impl FnOnce() -> ArchetypeComponentId,
    ) -> &mut ResourceData {
        self.resources.get_or_insert_with(component_id, || {
            let component_info = components.get_info(component_id).unwrap();
            self.component_ids.push(component_id);
            ResourceData {
                // SAFETY: component_info.drop() is valid for the types that will be inserted.
                data: unsafe { BlobBox::new(component_info.layout(), component_info.drop()) },
                added_tick: UnsafeCell::new(Tick::new(0)),
                changed_tick: UnsafeCell::new(Tick::new(0)),
                id: f(),
            }
        })
    }

    pub(crate) fn check_change_ticks(&mut self, change_tick: u32) {
        for component_id in &self.component_ids {
            // SAFETY: If a component ID is in component_ids, it's populated in the sparse array.
            unsafe {
                self.resources
                    .get_mut(*component_id)
                    .debug_checked_unwrap()
                    .check_change_ticks(change_tick);
            }
        }
    }
}
