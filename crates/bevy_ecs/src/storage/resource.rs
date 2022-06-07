use crate::archetype::ArchetypeComponentId;
use crate::archetype::ArchetypeComponentInfo;
use crate::component::{ComponentId, ComponentTicks};
use crate::storage::{Column, SparseSet};
use bevy_ptr::{OwningPtr, Ptr, PtrMut, UnsafeCellDeref};
use std::cell::UnsafeCell;

pub(crate) struct ResourceInfo {
    pub data: Column,
    pub component_info: ArchetypeComponentInfo,
}

/// The backing store for all [`Resource`]s stored in the [`World`].
///
/// [`Resource`]: crate::system::Resource
/// [`World`]: crate::world::World
#[derive(Default)]
pub struct Resources {
    pub(crate) resources: SparseSet<ComponentId, ResourceInfo>,
}

impl Resources {
    /// Gets the [`ArchetypeComponentId`] for a given resoruce.
    #[inline]
    pub fn get_archetype_component_id(
        &self,
        component_id: ComponentId,
    ) -> Option<ArchetypeComponentId> {
        self.resources
            .get(component_id)
            .map(|info| info.component_info.archetype_component_id)
    }

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

    /// Gets a read-only [`Ptr`] to a resource, if available.
    #[inline]
    pub fn get(&self, component_id: ComponentId) -> Option<Ptr<'_>> {
        let column = &self.resources.get(component_id)?.data;
        // SAFE: if a resource column exists, row 0 exists as well. caller takes ownership of the
        // ptr value / drop is called when R is dropped
        (!column.is_empty()).then(|| unsafe { column.get_data_unchecked(0) })
    }

    /// Gets a read-only [`Ptr`] to a resource, if available.
    #[inline]
    pub fn get_mut(&mut self, component_id: ComponentId) -> Option<PtrMut<'_>> {
        let column = &mut self.resources.get_mut(component_id)?.data;
        // SAFE: if a resource column exists, row 0 exists as well. caller takes ownership of the
        // ptr value / drop is called when R is dropped
        (!column.is_empty()).then(|| unsafe { column.get_data_unchecked_mut(0) })
    }

    /// Gets the [`ComponentTicks`] to a resource, if available.
    #[inline]
    pub fn get_ticks(&self, component_id: ComponentId) -> Option<&ComponentTicks> {
        let column = &self.resources.get(component_id)?.data;
        // SAFE: if a resource column exists, row 0 exists as well. caller takes ownership of the
        // ptr value / drop is called when R is dropped
        (!column.is_empty()).then(|| unsafe { column.get_ticks_unchecked(0).deref() })
    }

    /// Checks if the a resource is currently stored with a given ID.
    #[inline]
    pub fn contains(&self, component_id: ComponentId) -> bool {
        self.resources
            .get(component_id)
            .map(|info| !info.data.is_empty())
            .unwrap_or(false)
    }

    #[inline]
    pub(crate) fn get_with_ticks(
        &self,
        component_id: ComponentId,
    ) -> Option<(Ptr<'_>, &UnsafeCell<ComponentTicks>)> {
        let column = &self.resources.get(component_id)?.data;
        // SAFE: if a resource column exists, row 0 exists as well. caller takes ownership of the
        // ptr value / drop is called when R is dropped
        (!column.is_empty())
            .then(|| unsafe { (column.get_data_unchecked(0), column.get_ticks_unchecked(0)) })
    }

    /// Inserts a resource into the world.
    ///
    /// # Safety
    /// ptr must point to valid data of this column's component type which
    /// must correspond to the provided ID.
    #[inline]
    pub unsafe fn insert(
        &mut self,
        component_id: ComponentId,
        data: OwningPtr<'_>,
        ticks: ComponentTicks,
    ) -> Option<()> {
        let column = &mut self.resources.get_mut(component_id)?.data;
        debug_assert!(column.is_empty());
        column.push(data, ticks);
        Some(())
    }

    /// Removes a resource from the world.
    ///
    /// # Safety
    /// ptr must point to valid data of this column's component type which
    /// must correspond to the provided ID.
    #[inline]
    #[must_use = "The returned pointer to the removed component should be used or dropped"]
    pub fn remove(&mut self, component_id: ComponentId) -> Option<(OwningPtr<'_>, ComponentTicks)> {
        let column = &mut self.resources.get_mut(component_id)?.data;
        if column.is_empty() {
            return None;
        }
        // SAFE: if a resource column exists, row 0 exists as well. caller takes ownership of the
        // ptr value / drop is called when R is dropped
        unsafe { Some(column.swap_remove_and_forget_unchecked(0)) }
    }

    #[inline]
    pub(crate) fn remove_and_drop(&mut self, component_id: ComponentId) -> Option<()> {
        let column = &mut self.resources.get_mut(component_id)?.data;
        if column.is_empty() {
            return None;
        }
        // SAFE: if a resource column exists, row 0 exists as well. The removed value is dropped
        // immediately.
        unsafe {
            column.swap_remove_unchecked(0);
            Some(())
        }
    }

    pub fn check_change_ticks(&mut self, change_tick: u32) {
        for info in self.resources.values_mut() {
            info.data.check_change_ticks(change_tick);
        }
    }
}
