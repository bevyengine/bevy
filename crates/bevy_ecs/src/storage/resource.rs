use crate::archetype::ArchetypeComponentId;
use crate::archetype::ArchetypeComponentInfo;
use crate::component::{ComponentId, ComponentTicks};
use crate::storage::{Column, SparseSet};
use bevy_ptr::{OwningPtr, Ptr, PtrMut, UnsafeCellDeref};
use std::cell::UnsafeCell;

/// The backing store for all [`Resource`]s stored in the [`World`].
///
/// [`Resource`]: crate::system::system_param::Resource
/// [`World`]: crate::world::World
#[derive(Default)]
pub struct Resources {
    pub(crate) resources: SparseSet<ComponentId, Column>,
    pub(crate) components: SparseSet<ComponentId, ArchetypeComponentInfo>,
}

impl Resources {
    /// Gets the [`ArchetypeComponentId`] for a given resoruce.
    #[inline]
    pub fn get_archetype_component_id(
        &self,
        component_id: ComponentId,
    ) -> Option<ArchetypeComponentId> {
        self.components
            .get(component_id)
            .map(|info| info.archetype_component_id)
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

    #[inline]
    pub fn get(&self, component_id: ComponentId) -> Option<Ptr<'_>> {
        let column = self.resources.get(component_id)?;
        // SAFE: if a resource column exists, row 0 exists as well. caller takes ownership of the
        // ptr value / drop is called when R is dropped
        (!column.is_empty()).then(|| unsafe { column.get_data_unchecked(0) })
    }

    #[inline]
    pub fn get_mut(&mut self, component_id: ComponentId) -> Option<PtrMut<'_>> {
        let column = self.resources.get_mut(component_id)?;
        // SAFE: if a resource column exists, row 0 exists as well. caller takes ownership of the
        // ptr value / drop is called when R is dropped
        (!column.is_empty()).then(|| unsafe { column.get_data_unchecked_mut(0) })
    }

    #[inline]
    pub fn get_ticks(&self, component_id: ComponentId) -> Option<&ComponentTicks> {
        let column = self.resources.get(component_id)?;
        // SAFE: if a resource column exists, row 0 exists as well. caller takes ownership of the
        // ptr value / drop is called when R is dropped
        (!column.is_empty()).then(|| unsafe { column.get_ticks_unchecked(0).deref() })
    }

    #[inline]
    pub fn contains(&self, component_id: ComponentId) -> bool {
        self.resources.contains(component_id)
    }

    #[inline]
    pub(crate) fn get_with_ticks_unchecked(
        &self,
        component_id: ComponentId,
    ) -> Option<(Ptr<'_>, &UnsafeCell<ComponentTicks>)> {
        let column = self.resources.get(component_id)?;
        // SAFE: if a resource column exists, row 0 exists as well. caller takes ownership of the
        // ptr value / drop is called when R is dropped
        (!column.is_empty())
            .then(|| unsafe { (column.get_data_unchecked(0), column.get_ticks_unchecked(0)) })
    }

    // # Safety
    // - ptr must point to valid data of this column's component type
    #[inline]
    pub unsafe fn insert(
        &mut self,
        component_id: ComponentId,
        data: OwningPtr<'_>,
        ticks: ComponentTicks,
    ) -> Option<()> {
        let column = self.resources.get_mut(component_id)?;
        debug_assert!(column.is_empty());
        column.push(data, ticks);
        Some(())
    }

    #[inline]
    #[must_use = "The returned pointer to the removed component should be used or dropped"]
    pub fn remove(&mut self, component_id: ComponentId) -> Option<(OwningPtr<'_>, ComponentTicks)> {
        let column = self.resources.get_mut(component_id)?;
        if column.is_empty() {
            return None;
        }
        // SAFE: if a resource column exists, row 0 exists as well. caller takes ownership of the
        // ptr value / drop is called when R is dropped
        unsafe { Some(column.swap_remove_and_forget_unchecked(0)) }
    }

    pub fn check_change_ticks(&mut self, change_tick: u32) {
        for column in self.storages.resources.columns_mut() {
            column.check_change_ticks(change_tick);
        }
    }
}
