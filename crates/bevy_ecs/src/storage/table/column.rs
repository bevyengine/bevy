use super::*;
use crate::{
    change_detection::MaybeLocation,
    storage::{blob_array::BlobArray, thin_array_ptr::ThinArrayPtr},
};
use core::{fmt, panic::Location};

/// Dense ECS component storage.
///
/// A series of arrays storing component data and change ticks, with the capacities and lengths cut out for
/// performance reasons.
///
/// This type is used by [`Table`], because all of the capacities and lengths of the [`Table`]'s columns must match.
///
/// Like many other low-level storage types, [`ThinColumn`] has a limited and highly unsafe
/// interface. It's highly advised to use higher level types and their safe abstractions
/// instead of working directly with [`ThinColumn`].
pub struct ThinColumn {
    pub(super) data: BlobArray,
    pub(super) added_ticks: ThinArrayPtr<UnsafeCell<Tick>>,
    pub(super) changed_ticks: ThinArrayPtr<UnsafeCell<Tick>>,
    pub(super) changed_by: MaybeLocation<ThinArrayPtr<UnsafeCell<&'static Location<'static>>>>,
}

impl fmt::Debug for ThinColumn {
    // Required method
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ThinColumn").finish()
    }
}

impl ThinColumn {
    /// Create a new [`ThinColumn`] with the given `capacity`.
    pub fn with_capacity(component_info: &ComponentInfo, capacity: usize) -> Self {
        Self {
            // SAFETY: The components stored in this columns will match the information in `component_info`
            data: unsafe {
                BlobArray::with_capacity(component_info.layout(), component_info.drop(), capacity)
            },
            added_ticks: ThinArrayPtr::with_capacity(capacity),
            changed_ticks: ThinArrayPtr::with_capacity(capacity),
            changed_by: MaybeLocation::new_with(|| ThinArrayPtr::with_capacity(capacity)),
        }
    }

    /// Swap-remove and drop the removed element, but the component at `row` must not be the last element.
    ///
    /// # Safety
    /// - `row.as_usize()` < `len`
    /// - `last_element_index` = `len - 1`
    /// - `last_element_index` != `row.as_usize()`
    /// -   The caller should update the `len` to `len - 1`, or immediately initialize another element in the `last_element_index`
    pub(crate) unsafe fn swap_remove_and_drop_unchecked_nonoverlapping(
        &mut self,
        last_element_index: usize,
        row: TableRow,
    ) {
        self.data
            .swap_remove_and_drop_unchecked_nonoverlapping(row.as_usize(), last_element_index);
        self.added_ticks
            .swap_remove_unchecked_nonoverlapping(row.as_usize(), last_element_index);
        self.changed_ticks
            .swap_remove_unchecked_nonoverlapping(row.as_usize(), last_element_index);
        self.changed_by.as_mut().map(|changed_by| {
            changed_by.swap_remove_unchecked_nonoverlapping(row.as_usize(), last_element_index);
        });
    }

    /// Swap-remove and drop the removed element.
    ///
    /// # Safety
    /// - `last_element_index` must be the index of the last element—stored in the highest place in memory.
    /// - `row.as_usize()` <= `last_element_index`
    /// -   The caller should update the their saved length to reflect the change (decrement it by 1).
    pub(crate) unsafe fn swap_remove_and_drop_unchecked(
        &mut self,
        last_element_index: usize,
        row: TableRow,
    ) {
        self.data
            .swap_remove_and_drop_unchecked(row.as_usize(), last_element_index);
        self.added_ticks
            .swap_remove_and_drop_unchecked(row.as_usize(), last_element_index);
        self.changed_ticks
            .swap_remove_and_drop_unchecked(row.as_usize(), last_element_index);
        self.changed_by.as_mut().map(|changed_by| {
            changed_by.swap_remove_and_drop_unchecked(row.as_usize(), last_element_index);
        });
    }

    /// Removes an element from the [`ThinColumn`] and returns it.
    /// This does not preserve ordering, but is O(1) and does not do any bounds checking.
    ///
    /// The element is replaced with the last element in the [`ThinColumn`].
    ///
    /// It's the caller's responsibility to ensure that the returned value is dropped or used.
    /// Failure to do so may result in resources not being released (i.e. file handles not
    /// being release, memory leaks, etc)
    ///
    /// # Safety
    /// - `last_element_index` must be the index of the last element—stored in the highest place in memory.
    /// - `row.as_usize()` <= `last_element_index`
    /// -   The caller should update the their saved length to reflect the change (decrement it by 1).
    pub(crate) unsafe fn swap_remove_and_forget_unchecked(
        &mut self,
        last_element_index: usize,
        row: TableRow,
    ) -> OwningPtr {
        let data = self
            .data
            .swap_remove_unchecked(row.as_usize(), last_element_index);
        self.added_ticks
            .swap_remove_unchecked(row.as_usize(), last_element_index);
        self.changed_ticks
            .swap_remove_unchecked(row.as_usize(), last_element_index);
        self.changed_by
            .as_mut()
            .map(|changed_by| changed_by.swap_remove_unchecked(row.as_usize(), last_element_index));

        data
    }

    /// Call [`realloc`](std::alloc::realloc) to expand / shrink the memory allocation for this [`ThinColumn`]
    ///
    /// # Safety
    /// - `current_capacity` must be the current capacity of this column (the capacity of `self.data`, `self.added_ticks`, `self.changed_tick`)
    /// -   The caller should make sure their saved `capacity` value is updated to `new_capacity` after this operation.
    pub(crate) unsafe fn realloc(
        &mut self,
        current_capacity: NonZeroUsize,
        new_capacity: NonZeroUsize,
    ) {
        self.data.realloc(current_capacity, new_capacity);
        self.added_ticks.realloc(current_capacity, new_capacity);
        self.changed_ticks.realloc(current_capacity, new_capacity);
        self.changed_by
            .as_mut()
            .map(|changed_by| changed_by.realloc(current_capacity, new_capacity));
    }

    /// Call [`alloc`](std::alloc::alloc) to allocate memory for this [`ThinColumn`]
    /// The caller should make sure their saved `capacity` value is updated to `new_capacity` after this operation.
    pub(crate) fn alloc(&mut self, new_capacity: NonZeroUsize) {
        self.data.alloc(new_capacity);
        self.added_ticks.alloc(new_capacity);
        self.changed_ticks.alloc(new_capacity);
        self.changed_by
            .as_mut()
            .map(|changed_by| changed_by.alloc(new_capacity));
    }

    /// Writes component data to the column at the given row.
    /// Assumes the slot is uninitialized, drop is not called.
    /// To overwrite existing initialized value, use [`Self::replace`] instead.
    ///
    /// # Safety
    /// - `row.as_usize()` must be in bounds.
    /// - `comp_ptr` holds a component that matches the `component_id`
    #[inline]
    pub(crate) unsafe fn initialize(
        &mut self,
        row: TableRow,
        data: OwningPtr<'_>,
        tick: Tick,
        caller: MaybeLocation,
    ) {
        self.data.initialize_unchecked(row.as_usize(), data);
        *self.added_ticks.get_unchecked_mut(row.as_usize()).get_mut() = tick;
        *self
            .changed_ticks
            .get_unchecked_mut(row.as_usize())
            .get_mut() = tick;
        self.changed_by
            .as_mut()
            .map(|changed_by| changed_by.get_unchecked_mut(row.as_usize()).get_mut())
            .assign(caller);
    }

    /// Writes component data to the column at given row. Assumes the slot is initialized, drops the previous value.
    ///
    /// # Safety
    /// - `row.as_usize()` must be in bounds.
    /// - `data` holds a component that matches the `component_id`
    #[inline]
    pub(crate) unsafe fn replace(
        &mut self,
        row: TableRow,
        data: OwningPtr<'_>,
        change_tick: Tick,
        caller: MaybeLocation,
    ) {
        self.data.replace_unchecked(row.as_usize(), data);
        *self
            .changed_ticks
            .get_unchecked_mut(row.as_usize())
            .get_mut() = change_tick;
        self.changed_by
            .as_mut()
            .map(|changed_by| changed_by.get_unchecked_mut(row.as_usize()).get_mut())
            .assign(caller);
    }

    /// Removes the element from `other` at `src_row` and inserts it
    /// into the current column to initialize the values at `dst_row`.
    /// Does not do any bounds checking.
    ///
    /// # Safety
    ///  - `other` must have the same data layout as `self`
    ///  - `src_row` must be in bounds for `other`
    ///  - `dst_row` must be in bounds for `self`
    ///  - `other[src_row]` must be initialized to a valid value.
    ///  - `self[dst_row]` must not be initialized yet.
    #[inline]
    pub(crate) unsafe fn initialize_from_unchecked(
        &mut self,
        other: &mut ThinColumn,
        other_last_element_index: usize,
        src_row: TableRow,
        dst_row: TableRow,
    ) {
        debug_assert!(self.data.layout() == other.data.layout());
        // Init the data
        let src_val = other
            .data
            .swap_remove_unchecked(src_row.as_usize(), other_last_element_index);
        self.data.initialize_unchecked(dst_row.as_usize(), src_val);
        // Init added_ticks
        let added_tick = other
            .added_ticks
            .swap_remove_unchecked(src_row.as_usize(), other_last_element_index);
        self.added_ticks
            .initialize_unchecked(dst_row.as_usize(), added_tick);
        // Init changed_ticks
        let changed_tick = other
            .changed_ticks
            .swap_remove_unchecked(src_row.as_usize(), other_last_element_index);
        self.changed_ticks
            .initialize_unchecked(dst_row.as_usize(), changed_tick);
        self.changed_by.as_mut().zip(other.changed_by.as_mut()).map(
            |(self_changed_by, other_changed_by)| {
                let changed_by = other_changed_by
                    .swap_remove_unchecked(src_row.as_usize(), other_last_element_index);
                self_changed_by.initialize_unchecked(dst_row.as_usize(), changed_by);
            },
        );
    }

    /// Call [`Tick::check_tick`] on all of the ticks stored in this column.
    ///
    /// # Safety
    /// `len` is the actual length of this column
    #[inline]
    pub(crate) unsafe fn check_change_ticks(&mut self, len: usize, change_tick: Tick) {
        for i in 0..len {
            // SAFETY:
            // - `i` < `len`
            // we have a mutable reference to `self`
            unsafe { self.added_ticks.get_unchecked_mut(i) }
                .get_mut()
                .check_tick(change_tick);
            // SAFETY:
            // - `i` < `len`
            // we have a mutable reference to `self`
            unsafe { self.changed_ticks.get_unchecked_mut(i) }
                .get_mut()
                .check_tick(change_tick);
        }
    }

    /// Clear all the components from this column.
    ///
    /// # Safety
    /// - `len` must match the actual length of the column
    /// -   The caller must not use the elements this column's data until [`initializing`](Self::initialize) it again (set `len` to 0).
    pub(crate) unsafe fn clear(&mut self, len: usize) {
        self.added_ticks.clear_elements(len);
        self.changed_ticks.clear_elements(len);
        self.data.clear(len);
        self.changed_by
            .as_mut()
            .map(|changed_by| changed_by.clear_elements(len));
    }

    /// Because this method needs parameters, it can't be the implementation of the `Drop` trait.
    /// The owner of this [`ThinColumn`] must call this method with the correct information.
    ///
    /// # Safety
    /// - `len` is indeed the length of the column
    /// - `cap` is indeed the capacity of the column
    /// - the data stored in `self` will never be used again
    pub(crate) unsafe fn drop(&mut self, cap: usize, len: usize) {
        self.added_ticks.drop(cap, len);
        self.changed_ticks.drop(cap, len);
        self.data.drop(cap, len);
        self.changed_by
            .as_mut()
            .map(|changed_by| changed_by.drop(cap, len));
    }

    /// Drops the last component in this column.
    ///
    /// # Safety
    /// - `last_element_index` is indeed the index of the last element
    /// - the data stored in `last_element_index` will never be used unless properly initialized again.
    pub(crate) unsafe fn drop_last_component(&mut self, last_element_index: usize) {
        core::ptr::drop_in_place(self.added_ticks.get_unchecked_raw(last_element_index));
        core::ptr::drop_in_place(self.changed_ticks.get_unchecked_raw(last_element_index));
        self.changed_by.as_mut().map(|changed_by| {
            core::ptr::drop_in_place(changed_by.get_unchecked_raw(last_element_index));
        });
        self.data.drop_last_element(last_element_index);
    }

    /// Returns the component data in this column at the given row
    ///
    /// # Safety
    /// - `row` must be within bounds (`row` < len)
    #[inline]
    pub unsafe fn get_data_unchecked(&self, row: TableRow) -> Ptr {
        self.data.get_unchecked(row.as_usize())
    }

    /// Returns the added tick in this column at the given row
    ///
    /// # Safety
    /// - `row` must be within bounds (`row` < len)
    #[inline]
    pub unsafe fn get_added_tick_unchecked(&self, row: TableRow) -> &UnsafeCell<Tick> {
        self.added_ticks.get_unchecked(row.as_usize())
    }

    /// Returns the changed tick in this column at the given row
    ///
    /// # Safety
    /// - `row` must be within bounds (`row` < len)
    #[inline]
    pub unsafe fn get_changed_tick_unchecked(&self, row: TableRow) -> &UnsafeCell<Tick> {
        self.changed_ticks.get_unchecked(row.as_usize())
    }

    /// Returns the component ticks in this column at the given row
    ///
    /// # Safety
    /// - `row` must be within bounds (`row` < len)
    #[inline]
    pub unsafe fn get_ticks_unchecked(&self, row: TableRow) -> ComponentTicks {
        let added = self.get_added_tick_unchecked(row).read();
        let changed = self.get_changed_tick_unchecked(row).read();
        ComponentTicks { added, changed }
    }

    /// Returns the calling location that last modified the given row in this column
    ///
    /// # Safety
    /// - `row` must be within bounds (`row` < len)
    #[inline]
    pub unsafe fn get_changed_by_unchecked(
        &self,
        row: TableRow,
    ) -> MaybeLocation<&UnsafeCell<&'static Location<'static>>> {
        self.changed_by
            .as_ref()
            .map(|changed_by| changed_by.get_unchecked(row.as_usize()))
    }

    /// Get a slice to the data stored in this [`ThinColumn`].
    ///
    /// # Safety
    /// - `T` must match the type of data that's stored in this [`ThinColumn`]
    /// - `len` must match the actual length of this column (number of elements stored)
    #[inline]
    pub unsafe fn get_data_slice<T>(&self, len: usize) -> &[UnsafeCell<T>] {
        self.data.get_sub_slice(len)
    }

    /// Get a slice to the added [`ticks`](Tick) in this [`ThinColumn`].
    ///
    /// # Safety
    /// - `len` must match the actual length of this column (number of elements stored)
    #[inline]
    pub unsafe fn get_added_ticks_slice(&self, len: usize) -> &[UnsafeCell<Tick>] {
        self.added_ticks.as_slice(len)
    }

    /// Get a slice to the changed [`ticks`](Tick) in this [`ThinColumn`].
    ///
    /// # Safety
    /// - `len` must match the actual length of this column (number of elements stored)
    #[inline]
    pub unsafe fn get_changed_ticks_slice(&self, len: usize) -> &[UnsafeCell<Tick>] {
        self.changed_ticks.as_slice(len)
    }

    /// Get a slice to the calling locations that last changed each value in this [`ThinColumn`]
    ///
    /// # Safety
    /// - `len` must match the actual length of this column (number of elements stored)
    #[inline]
    pub unsafe fn get_changed_by_slice(
        &self,
        len: usize,
    ) -> MaybeLocation<&[UnsafeCell<&'static Location<'static>>]> {
        self.changed_by
            .as_ref()
            .map(|changed_by| changed_by.as_slice(len))
    }

    /// Returns the drop function for elements of the column,
    /// or `None` if they don't need to be dropped.
    #[inline]
    pub fn get_drop(&self) -> Option<unsafe fn(OwningPtr<'_>)> {
        self.data.drop
    }
}
