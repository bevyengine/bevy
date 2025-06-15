use super::*;
use crate::{
    change_detection::MaybeLocation,
    component::TickCells,
    storage::{blob_array::BlobArray, thin_array_ptr::ThinArrayPtr},
};
use alloc::vec::Vec;
use bevy_ptr::PtrMut;
use core::panic::Location;

/// Very similar to a normal [`Column`], but with the capacities and lengths cut out for performance reasons.
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
            .swap_remove_and_drop_unchecked_nonoverlapping(row.index(), last_element_index);
        self.added_ticks
            .swap_remove_unchecked_nonoverlapping(row.index(), last_element_index);
        self.changed_ticks
            .swap_remove_unchecked_nonoverlapping(row.index(), last_element_index);
        self.changed_by.as_mut().map(|changed_by| {
            changed_by.swap_remove_unchecked_nonoverlapping(row.index(), last_element_index);
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
            .swap_remove_and_drop_unchecked(row.index(), last_element_index);
        self.added_ticks
            .swap_remove_and_drop_unchecked(row.index(), last_element_index);
        self.changed_ticks
            .swap_remove_and_drop_unchecked(row.index(), last_element_index);
        self.changed_by.as_mut().map(|changed_by| {
            changed_by.swap_remove_and_drop_unchecked(row.index(), last_element_index);
        });
    }

    /// Swap-remove and forget the removed element.
    ///
    /// # Safety
    /// - `last_element_index` must be the index of the last element—stored in the highest place in memory.
    /// - `row.as_usize()` <= `last_element_index`
    /// -   The caller should update the their saved length to reflect the change (decrement it by 1).
    pub(crate) unsafe fn swap_remove_and_forget_unchecked(
        &mut self,
        last_element_index: usize,
        row: TableRow,
    ) {
        let _ = self
            .data
            .swap_remove_unchecked(row.index(), last_element_index);
        self.added_ticks
            .swap_remove_unchecked(row.index(), last_element_index);
        self.changed_ticks
            .swap_remove_unchecked(row.index(), last_element_index);
        self.changed_by
            .as_mut()
            .map(|changed_by| changed_by.swap_remove_unchecked(row.index(), last_element_index));
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
        self.data.initialize_unchecked(row.index(), data);
        *self.added_ticks.get_unchecked_mut(row.index()).get_mut() = tick;
        *self.changed_ticks.get_unchecked_mut(row.index()).get_mut() = tick;
        self.changed_by
            .as_mut()
            .map(|changed_by| changed_by.get_unchecked_mut(row.index()).get_mut())
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
        self.data.replace_unchecked(row.index(), data);
        *self.changed_ticks.get_unchecked_mut(row.index()).get_mut() = change_tick;
        self.changed_by
            .as_mut()
            .map(|changed_by| changed_by.get_unchecked_mut(row.index()).get_mut())
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
            .swap_remove_unchecked(src_row.index(), other_last_element_index);
        self.data.initialize_unchecked(dst_row.index(), src_val);
        // Init added_ticks
        let added_tick = other
            .added_ticks
            .swap_remove_unchecked(src_row.index(), other_last_element_index);
        self.added_ticks
            .initialize_unchecked(dst_row.index(), added_tick);
        // Init changed_ticks
        let changed_tick = other
            .changed_ticks
            .swap_remove_unchecked(src_row.index(), other_last_element_index);
        self.changed_ticks
            .initialize_unchecked(dst_row.index(), changed_tick);
        self.changed_by.as_mut().zip(other.changed_by.as_mut()).map(
            |(self_changed_by, other_changed_by)| {
                let changed_by = other_changed_by
                    .swap_remove_unchecked(src_row.index(), other_last_element_index);
                self_changed_by.initialize_unchecked(dst_row.index(), changed_by);
            },
        );
    }

    /// Call [`Tick::check_tick`] on all of the ticks stored in this column.
    ///
    /// # Safety
    /// `len` is the actual length of this column
    #[inline]
    pub(crate) unsafe fn check_change_ticks(&mut self, len: usize, check: CheckChangeTicks) {
        for i in 0..len {
            // SAFETY:
            // - `i` < `len`
            // we have a mutable reference to `self`
            unsafe { self.added_ticks.get_unchecked_mut(i) }
                .get_mut()
                .check_tick(check);
            // SAFETY:
            // - `i` < `len`
            // we have a mutable reference to `self`
            unsafe { self.changed_ticks.get_unchecked_mut(i) }
                .get_mut()
                .check_tick(check);
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

    /// Get a slice to the data stored in this [`ThinColumn`].
    ///
    /// # Safety
    /// - `T` must match the type of data that's stored in this [`ThinColumn`]
    /// - `len` must match the actual length of this column (number of elements stored)
    pub unsafe fn get_data_slice<T>(&self, len: usize) -> &[UnsafeCell<T>] {
        self.data.get_sub_slice(len)
    }

    /// Get a slice to the added [`ticks`](Tick) in this [`ThinColumn`].
    ///
    /// # Safety
    /// - `len` must match the actual length of this column (number of elements stored)
    pub unsafe fn get_added_ticks_slice(&self, len: usize) -> &[UnsafeCell<Tick>] {
        self.added_ticks.as_slice(len)
    }

    /// Get a slice to the changed [`ticks`](Tick) in this [`ThinColumn`].
    ///
    /// # Safety
    /// - `len` must match the actual length of this column (number of elements stored)
    pub unsafe fn get_changed_ticks_slice(&self, len: usize) -> &[UnsafeCell<Tick>] {
        self.changed_ticks.as_slice(len)
    }

    /// Get a slice to the calling locations that last changed each value in this [`ThinColumn`]
    ///
    /// # Safety
    /// - `len` must match the actual length of this column (number of elements stored)
    pub unsafe fn get_changed_by_slice(
        &self,
        len: usize,
    ) -> MaybeLocation<&[UnsafeCell<&'static Location<'static>>]> {
        self.changed_by
            .as_ref()
            .map(|changed_by| changed_by.as_slice(len))
    }
}

/// A type-erased contiguous container for data of a homogeneous type.
///
/// Conceptually, a [`Column`] is very similar to a type-erased `Vec<T>`.
/// It also stores the change detection ticks for its components, kept in two separate
/// contiguous buffers internally. An element shares its data across these buffers by using the
/// same index (i.e. the entity at row 3 has it's data at index 3 and its change detection ticks at index 3).
///
/// Like many other low-level storage types, [`Column`] has a limited and highly unsafe
/// interface. It's highly advised to use higher level types and their safe abstractions
/// instead of working directly with [`Column`].
#[derive(Debug)]
pub struct Column {
    pub(super) data: BlobVec,
    pub(super) added_ticks: Vec<UnsafeCell<Tick>>,
    pub(super) changed_ticks: Vec<UnsafeCell<Tick>>,
    changed_by: MaybeLocation<Vec<UnsafeCell<&'static Location<'static>>>>,
}

impl Column {
    /// Constructs a new [`Column`], configured with a component's layout and an initial `capacity`.
    #[inline]
    pub(crate) fn with_capacity(component_info: &ComponentInfo, capacity: usize) -> Self {
        Column {
            // SAFETY: component_info.drop() is valid for the types that will be inserted.
            data: unsafe { BlobVec::new(component_info.layout(), component_info.drop(), capacity) },
            added_ticks: Vec::with_capacity(capacity),
            changed_ticks: Vec::with_capacity(capacity),
            changed_by: MaybeLocation::new_with(|| Vec::with_capacity(capacity)),
        }
    }

    /// Fetches the [`Layout`] for the underlying type.
    #[inline]
    pub fn item_layout(&self) -> Layout {
        self.data.layout()
    }

    /// Writes component data to the column at given row.
    /// Assumes the slot is initialized, calls drop.
    ///
    /// # Safety
    /// Assumes data has already been allocated for the given row.
    #[inline]
    pub(crate) unsafe fn replace(
        &mut self,
        row: TableRow,
        data: OwningPtr<'_>,
        change_tick: Tick,
        caller: MaybeLocation,
    ) {
        debug_assert!(row.index() < self.len());
        self.data.replace_unchecked(row.index(), data);
        *self.changed_ticks.get_unchecked_mut(row.index()).get_mut() = change_tick;
        self.changed_by
            .as_mut()
            .map(|changed_by| changed_by.get_unchecked_mut(row.index()).get_mut())
            .assign(caller);
    }

    /// Gets the current number of elements stored in the column.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Checks if the column is empty. Returns `true` if there are no elements, `false` otherwise.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Removes an element from the [`Column`].
    ///
    /// - The value will be dropped if it implements [`Drop`].
    /// - This does not preserve ordering, but is O(1).
    /// - This does not do any bounds checking.
    /// - The element is replaced with the last element in the [`Column`].
    ///
    /// # Safety
    /// `row` must be within the range `[0, self.len())`.
    #[inline]
    pub(crate) unsafe fn swap_remove_unchecked(&mut self, row: TableRow) {
        self.data.swap_remove_and_drop_unchecked(row.index());
        self.added_ticks.swap_remove(row.index());
        self.changed_ticks.swap_remove(row.index());
        self.changed_by
            .as_mut()
            .map(|changed_by| changed_by.swap_remove(row.index()));
    }

    /// Removes an element from the [`Column`] and returns it and its change detection ticks.
    /// This does not preserve ordering, but is O(1) and does not do any bounds checking.
    ///
    /// The element is replaced with the last element in the [`Column`].
    ///
    /// It's the caller's responsibility to ensure that the removed value is dropped or used.
    /// Failure to do so may result in resources not being released (i.e. files handles not being
    /// released, memory leaks, etc.)
    ///
    /// # Safety
    /// `row` must be within the range `[0, self.len())`.
    #[inline]
    #[must_use = "The returned pointer should be used to dropped the removed component"]
    pub(crate) unsafe fn swap_remove_and_forget_unchecked(
        &mut self,
        row: TableRow,
    ) -> (OwningPtr<'_>, ComponentTicks, MaybeLocation) {
        let data = self.data.swap_remove_and_forget_unchecked(row.index());
        let added = self.added_ticks.swap_remove(row.index()).into_inner();
        let changed = self.changed_ticks.swap_remove(row.index()).into_inner();
        let caller = self
            .changed_by
            .as_mut()
            .map(|changed_by| changed_by.swap_remove(row.index()).into_inner());
        (data, ComponentTicks { added, changed }, caller)
    }

    /// Pushes a new value onto the end of the [`Column`].
    ///
    /// # Safety
    /// `ptr` must point to valid data of this column's component type
    pub(crate) unsafe fn push(
        &mut self,
        ptr: OwningPtr<'_>,
        ticks: ComponentTicks,
        caller: MaybeLocation,
    ) {
        self.data.push(ptr);
        self.added_ticks.push(UnsafeCell::new(ticks.added));
        self.changed_ticks.push(UnsafeCell::new(ticks.changed));
        self.changed_by
            .as_mut()
            .zip(caller)
            .map(|(changed_by, caller)| changed_by.push(UnsafeCell::new(caller)));
    }

    /// Fetches the data pointer to the first element of the [`Column`].
    ///
    /// The pointer is type erased, so using this function to fetch anything
    /// other than the first element will require computing the offset using
    /// [`Column::item_layout`].
    #[inline]
    pub fn get_data_ptr(&self) -> Ptr<'_> {
        self.data.get_ptr()
    }

    /// Fetches the slice to the [`Column`]'s data cast to a given type.
    ///
    /// Note: The values stored within are [`UnsafeCell`].
    /// Users of this API must ensure that accesses to each individual element
    /// adhere to the safety invariants of [`UnsafeCell`].
    ///
    /// # Safety
    /// The type `T` must be the type of the items in this column.
    pub unsafe fn get_data_slice<T>(&self) -> &[UnsafeCell<T>] {
        self.data.get_slice()
    }

    /// Fetches the slice to the [`Column`]'s "added" change detection ticks.
    ///
    /// Note: The values stored within are [`UnsafeCell`].
    /// Users of this API must ensure that accesses to each individual element
    /// adhere to the safety invariants of [`UnsafeCell`].
    #[inline]
    pub fn get_added_ticks_slice(&self) -> &[UnsafeCell<Tick>] {
        &self.added_ticks
    }

    /// Fetches the slice to the [`Column`]'s "changed" change detection ticks.
    ///
    /// Note: The values stored within are [`UnsafeCell`].
    /// Users of this API must ensure that accesses to each individual element
    /// adhere to the safety invariants of [`UnsafeCell`].
    #[inline]
    pub fn get_changed_ticks_slice(&self) -> &[UnsafeCell<Tick>] {
        &self.changed_ticks
    }

    /// Fetches a reference to the data and change detection ticks at `row`.
    ///
    /// Returns `None` if `row` is out of bounds.
    #[inline]
    pub fn get(&self, row: TableRow) -> Option<(Ptr<'_>, TickCells<'_>)> {
        (row.index() < self.data.len())
            // SAFETY: The row is length checked before fetching the pointer. This is being
            // accessed through a read-only reference to the column.
            .then(|| unsafe {
                (
                    self.data.get_unchecked(row.index()),
                    TickCells {
                        added: self.added_ticks.get_unchecked(row.index()),
                        changed: self.changed_ticks.get_unchecked(row.index()),
                    },
                )
            })
    }

    /// Fetches a read-only reference to the data at `row`.
    ///
    /// Returns `None` if `row` is out of bounds.
    #[inline]
    pub fn get_data(&self, row: TableRow) -> Option<Ptr<'_>> {
        (row.index() < self.data.len()).then(|| {
            // SAFETY: The row is length checked before fetching the pointer. This is being
            // accessed through a read-only reference to the column.
            unsafe { self.data.get_unchecked(row.index()) }
        })
    }

    /// Fetches a read-only reference to the data at `row`. Unlike [`Column::get`] this does not
    /// do any bounds checking.
    ///
    /// # Safety
    /// - `row` must be within the range `[0, self.len())`.
    /// - no other mutable reference to the data of the same row can exist at the same time
    #[inline]
    pub unsafe fn get_data_unchecked(&self, row: TableRow) -> Ptr<'_> {
        debug_assert!(row.index() < self.data.len());
        self.data.get_unchecked(row.index())
    }

    /// Fetches a mutable reference to the data at `row`.
    ///
    /// Returns `None` if `row` is out of bounds.
    #[inline]
    pub fn get_data_mut(&mut self, row: TableRow) -> Option<PtrMut<'_>> {
        (row.index() < self.data.len()).then(|| {
            // SAFETY: The row is length checked before fetching the pointer. This is being
            // accessed through an exclusive reference to the column.
            unsafe { self.data.get_unchecked_mut(row.index()) }
        })
    }

    /// Fetches the "added" change detection tick for the value at `row`.
    ///
    /// Returns `None` if `row` is out of bounds.
    ///
    /// Note: The values stored within are [`UnsafeCell`].
    /// Users of this API must ensure that accesses to each individual element
    /// adhere to the safety invariants of [`UnsafeCell`].
    #[inline]
    pub fn get_added_tick(&self, row: TableRow) -> Option<&UnsafeCell<Tick>> {
        self.added_ticks.get(row.index())
    }

    /// Fetches the "changed" change detection tick for the value at `row`.
    ///
    /// Returns `None` if `row` is out of bounds.
    ///
    /// Note: The values stored within are [`UnsafeCell`].
    /// Users of this API must ensure that accesses to each individual element
    /// adhere to the safety invariants of [`UnsafeCell`].
    #[inline]
    pub fn get_changed_tick(&self, row: TableRow) -> Option<&UnsafeCell<Tick>> {
        self.changed_ticks.get(row.index())
    }

    /// Fetches the change detection ticks for the value at `row`.
    ///
    /// Returns `None` if `row` is out of bounds.
    #[inline]
    pub fn get_ticks(&self, row: TableRow) -> Option<ComponentTicks> {
        if row.index() < self.data.len() {
            // SAFETY: The size of the column has already been checked.
            Some(unsafe { self.get_ticks_unchecked(row) })
        } else {
            None
        }
    }

    /// Fetches the "added" change detection tick for the value at `row`. Unlike [`Column::get_added_tick`]
    /// this function does not do any bounds checking.
    ///
    /// # Safety
    /// `row` must be within the range `[0, self.len())`.
    #[inline]
    pub unsafe fn get_added_tick_unchecked(&self, row: TableRow) -> &UnsafeCell<Tick> {
        debug_assert!(row.index() < self.added_ticks.len());
        self.added_ticks.get_unchecked(row.index())
    }

    /// Fetches the "changed" change detection tick for the value at `row`. Unlike [`Column::get_changed_tick`]
    /// this function does not do any bounds checking.
    ///
    /// # Safety
    /// `row` must be within the range `[0, self.len())`.
    #[inline]
    pub unsafe fn get_changed_tick_unchecked(&self, row: TableRow) -> &UnsafeCell<Tick> {
        debug_assert!(row.index() < self.changed_ticks.len());
        self.changed_ticks.get_unchecked(row.index())
    }

    /// Fetches the change detection ticks for the value at `row`. Unlike [`Column::get_ticks`]
    /// this function does not do any bounds checking.
    ///
    /// # Safety
    /// `row` must be within the range `[0, self.len())`.
    #[inline]
    pub unsafe fn get_ticks_unchecked(&self, row: TableRow) -> ComponentTicks {
        debug_assert!(row.index() < self.added_ticks.len());
        debug_assert!(row.index() < self.changed_ticks.len());
        ComponentTicks {
            added: self.added_ticks.get_unchecked(row.index()).read(),
            changed: self.changed_ticks.get_unchecked(row.index()).read(),
        }
    }

    /// Clears the column, removing all values.
    ///
    /// Note that this function has no effect on the allocated capacity of the [`Column`]>
    pub fn clear(&mut self) {
        self.data.clear();
        self.added_ticks.clear();
        self.changed_ticks.clear();
        self.changed_by.as_mut().map(Vec::clear);
    }

    #[inline]
    pub(crate) fn check_change_ticks(&mut self, check: CheckChangeTicks) {
        for component_ticks in &mut self.added_ticks {
            component_ticks.get_mut().check_tick(check);
        }
        for component_ticks in &mut self.changed_ticks {
            component_ticks.get_mut().check_tick(check);
        }
    }

    /// Fetches the calling location that last changed the value at `row`.
    ///
    /// Returns `None` if `row` is out of bounds.
    ///
    /// Note: The values stored within are [`UnsafeCell`].
    /// Users of this API must ensure that accesses to each individual element
    /// adhere to the safety invariants of [`UnsafeCell`].
    #[inline]
    pub fn get_changed_by(
        &self,
        row: TableRow,
    ) -> MaybeLocation<Option<&UnsafeCell<&'static Location<'static>>>> {
        self.changed_by
            .as_ref()
            .map(|changed_by| changed_by.get(row.index()))
    }

    /// Fetches the calling location that last changed the value at `row`.
    ///
    /// Unlike [`Column::get_changed_by`] this function does not do any bounds checking.
    ///
    /// # Safety
    /// `row` must be within the range `[0, self.len())`.
    #[inline]
    pub unsafe fn get_changed_by_unchecked(
        &self,
        row: TableRow,
    ) -> MaybeLocation<&UnsafeCell<&'static Location<'static>>> {
        self.changed_by.as_ref().map(|changed_by| {
            debug_assert!(row.index() < changed_by.len());
            changed_by.get_unchecked(row.index())
        })
    }

    /// Returns the drop function for elements of the column,
    /// or `None` if they don't need to be dropped.
    #[inline]
    pub fn get_drop(&self) -> Option<unsafe fn(OwningPtr<'_>)> {
        self.data.get_drop()
    }
}
