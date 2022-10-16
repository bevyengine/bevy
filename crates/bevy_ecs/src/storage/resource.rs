use crate::archetype::ArchetypeComponentId;
use crate::archetype::ArchetypeComponentInfo;
use crate::component::{ComponentId, ComponentTicks, Components, StorageType};
use crate::storage::{Column, SparseSet};
use bevy_ptr::{OwningPtr, Ptr, PtrMut, UnsafeCellDeref};
use std::cell::UnsafeCell;

/// The type-erased backing storage and metadata for a single resource within a [`World`].
///
/// [`World`]: crate::world::World
pub struct ResourceData {
    column: Column,
    component_info: ArchetypeComponentInfo,
}

impl ResourceData {
    /// Returns true if the resource is populated.
    #[inline]
    pub fn is_present(&self) -> bool {
        !self.column.is_empty()
    }

    #[inline]
    pub(crate) fn component_info(&self) -> &ArchetypeComponentInfo {
        &self.component_info
    }

    /// Gets a read-only pointer to the underlying resource, if available.
    #[inline]
    pub fn get_data(&self) -> Option<Ptr<'_>> {
        self.column.get_data(0)
    }

    /// Gets a mutable pointer to the underlying resource, if available.
    #[inline]
    pub fn get_data_mut(&mut self) -> Option<PtrMut<'_>> {
        self.column.get_data_mut(0)
    }

    /// Gets a read-only reference to the change ticks of the underlying resource, if available.
    #[inline]
    pub fn get_ticks(&self) -> Option<&ComponentTicks> {
        self.column
            .get_ticks(0)
            // SAFETY: If the first row exists, a valid ticks value has been written.
            .map(|ticks| unsafe { ticks.deref() })
    }

    /// Gets a mutable reference to the change ticks of the underlying resource, if available.
    #[inline]
    pub fn get_ticks_mut(&mut self) -> Option<&mut ComponentTicks> {
        self.column
            .get_ticks(0)
            // SAFETY: If the first row exists, a valid ticks value has been written.
            // This function has exclusvie access to the underlying column.
            .map(|ticks| unsafe { ticks.deref_mut() })
    }

    #[inline]
    pub(crate) fn get_with_ticks(&self) -> Option<(Ptr<'_>, &UnsafeCell<ComponentTicks>)> {
        self.column.get(0)
    }

    /// Inserts a value into the resource. If a value is already present
    /// it will be replaced.
    ///
    /// # Safety
    /// `value` must be valid for the underlying type for the resource.
    #[inline]
    pub unsafe fn insert(&mut self, value: OwningPtr<'_>, change_tick: u32) {
        if self.is_present() {
            self.column.replace(0, value, change_tick);
        } else {
            self.column.push(value, ComponentTicks::new(change_tick));
        }
    }

    #[inline]
    pub(crate) unsafe fn insert_with_ticks(
        &mut self,
        value: OwningPtr<'_>,
        change_ticks: ComponentTicks,
    ) {
        if self.is_present() {
            self.column.replace_untracked(0, value);
            *self.column.get_ticks_unchecked(0).deref_mut() = change_ticks;
        } else {
            self.column.push(value, change_ticks);
        }
    }

    /// Removes a value from the resource, if present.
    #[inline]
    #[must_use = "The returned pointer to the removed component should be used or dropped"]
    pub fn remove(&mut self) -> Option<(OwningPtr<'_>, ComponentTicks)> {
        self.column.swap_remove_and_forget(0)
    }

    #[inline]
    pub(crate) fn remove_and_drop(&mut self) {
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
    /// The total number of resoruces stored in the [`World`]
    ///
    /// [`World`]: crate::world::World
    #[inline]
    pub fn len(&self) -> usize {
        self.resources.len()
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
    pub fn get_mut(&mut self, component_id: ComponentId) -> Option<&mut ResourceData> {
        self.resources.get_mut(component_id)
    }

    /// Fetches or initializes a new resource and returns back it's underlying column.
    ///
    /// # Panics
    /// Will panic if `component_id` is not valid for the provided `components`
    pub fn initialize_with<F>(
        &mut self,
        component_id: ComponentId,
        components: &Components,
        f: F,
    ) -> &mut ResourceData
    where
        F: FnOnce() -> ArchetypeComponentId,
    {
        self.resources.get_or_insert_with(component_id, || {
            let component_info = components.get_info(component_id).unwrap();
            ResourceData {
                column: Column::with_capacity(component_info, 1),
                component_info: ArchetypeComponentInfo {
                    archetype_component_id: f(),
                    storage_type: StorageType::Table,
                },
            }
        })
    }

    pub fn check_change_ticks(&mut self, change_tick: u32) {
        for info in self.resources.values_mut() {
            info.column.check_change_ticks(change_tick);
        }
    }
}
