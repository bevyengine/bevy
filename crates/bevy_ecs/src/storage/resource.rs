use crate::archetype::ArchetypeComponentId;
use crate::component::{ComponentId, ComponentTicks, Components, TickCells};
use crate::storage::{Column, SparseSet};
use bevy_ptr::{OwningPtr, Ptr, UnsafeCellDeref};

/// The type-erased backing storage and metadata for a single resource within a [`World`].
///
/// [`World`]: crate::world::World
pub struct ResourceData {
    column: Column,
    id: ArchetypeComponentId,
}

impl ResourceData {
    /// Returns true if the resource is populated.
    #[inline]
    pub fn is_present(&self) -> bool {
        !self.column.is_empty()
    }

    /// Gets the [`ArchetypeComponentId`] for the resource.
    #[inline]
    pub fn id(&self) -> ArchetypeComponentId {
        self.id
    }

    /// Gets a read-only pointer to the underlying resource, if available.
    #[inline]
    pub fn get_data(&self) -> Option<Ptr<'_>> {
        self.column.get_data(0)
    }

    /// Gets a read-only reference to the change ticks of the underlying resource, if available.
    #[inline]
    pub fn get_ticks(&self) -> Option<ComponentTicks> {
        self.column.get_ticks(0)
    }

    #[inline]
    pub(crate) fn get_with_ticks(&self) -> Option<(Ptr<'_>, TickCells<'_>)> {
        self.column.get(0)
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
            self.column.replace(0, value, change_tick);
        } else {
            self.column.push(value, ComponentTicks::new(change_tick));
        }
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
            self.column.replace_untracked(0, value);
            *self.column.get_added_ticks_unchecked(0).deref_mut() = change_ticks.added;
            *self.column.get_changed_ticks_unchecked(0).deref_mut() = change_ticks.changed;
        } else {
            self.column.push(value, change_ticks);
        }
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
        self.column.swap_remove_and_forget(0)
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
        self.column.clear();
    }
}

/// The backing store for all [`Resource`]s stored in the [`World`].
///
/// [`Resource`]: crate::system::Resource
/// [`World`]: crate::world::World
#[derive(Default)]
pub struct Resources {
    resources: SparseSet<ComponentId, ResourceData>,
}

impl Resources {
    /// The total number of resources stored in the [`World`]
    ///
    /// [`World`]: crate::world::World
    #[inline]
    pub fn len(&self) -> usize {
        self.resources.len()
    }

    /// Iterate over all resources that have been initialized, i.e. given a [`ComponentId`]
    pub fn iter(&self) -> impl Iterator<Item = (ComponentId, &ResourceData)> {
        self.resources.iter().map(|(id, data)| (*id, data))
    }

    /// Returns true if there are no resources stored in the [`World`],
    /// false otherwise.
    ///
    /// [`World`]: crate::world::World
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
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
            ResourceData {
                column: Column::with_capacity(component_info, 1),
                id: f(),
            }
        })
    }

    pub(crate) fn check_change_ticks(&mut self, change_tick: u32) {
        for info in self.resources.values_mut() {
            info.column.check_change_ticks(change_tick);
        }
    }
}
