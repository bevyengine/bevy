use super::*;
use crate::{
    component::TickCells,
    storage::{blob_array::BlobArray, thin_array_ptr::ThinArrayPtr},
};
use alloc::vec::Vec;
use bevy_ptr::PtrMut;

/// Very similar to a normal [`Column`], but with the capacities and lengths cut out for performance reasons.
///
/// This type is used by [`Table`], because all of the capacities and lengths of the [`Table`]'s columns must match.
///
/// Like many other low-level storage types, [`ThinColumn`] has a limited and highly unsafe
/// interface. It's highly advised to use higher level types and their safe abstractions
/// instead of working directly with [`ThinColumn`].
pub struct ThinColumn {
    pub(super) data: BlobArray,
    pub(super) ticks: Option<ThinColumnTicks>,
    #[cfg(feature = "track_location")]
    pub(super) changed_by: ThinArrayPtr<UnsafeCell<&'static Location<'static>>>,
}

impl ThinColumn {
    /// Create a new [`ThinColumn`] with the given `capacity`.
    pub fn with_capacity(component_info: &ComponentInfo, capacity: usize) -> Self {
        Self {
            // SAFETY: The components stored in this columns will match the information in `component_info`
            data: unsafe {
                BlobArray::with_capacity(component_info.layout(), component_info.drop(), capacity)
            },
            ticks: component_info
                .change_detection_enabled()
                .then_some(ThinColumnTicks {
                    added: ThinArrayPtr::with_capacity(capacity),
                    changed: ThinArrayPtr::with_capacity(capacity),
                }),
            #[cfg(feature = "track_location")]
            changed_by: ThinArrayPtr::with_capacity(capacity),
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
        if let Some(ticks) = &mut self.ticks {
            ticks
                .added
                .swap_remove_unchecked_nonoverlapping(row.as_usize(), last_element_index);
            ticks
                .changed
                .swap_remove_unchecked_nonoverlapping(row.as_usize(), last_element_index);
        }
        #[cfg(feature = "track_location")]
        self.changed_by
            .swap_remove_unchecked_nonoverlapping(row.as_usize(), last_element_index);
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
        if let Some(ticks) = &mut self.ticks {
            ticks
                .added
                .swap_remove_and_drop_unchecked(row.as_usize(), last_element_index);
            ticks
                .changed
                .swap_remove_and_drop_unchecked(row.as_usize(), last_element_index);
        }
        #[cfg(feature = "track_location")]
        self.changed_by
            .swap_remove_and_drop_unchecked(row.as_usize(), last_element_index);
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
            .swap_remove_unchecked(row.as_usize(), last_element_index);
        if let Some(ticks) = &mut self.ticks {
            ticks
                .added
                .swap_remove_unchecked(row.as_usize(), last_element_index);
            ticks
                .changed
                .swap_remove_unchecked(row.as_usize(), last_element_index);
        }
        #[cfg(feature = "track_location")]
        self.changed_by
            .swap_remove_unchecked(row.as_usize(), last_element_index);
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
        if let Some(ticks) = &mut self.ticks {
            ticks.added.realloc(current_capacity, new_capacity);
            ticks.changed.realloc(current_capacity, new_capacity);
        }
        #[cfg(feature = "track_location")]
        self.changed_by.realloc(current_capacity, new_capacity);
    }

    /// Call [`alloc`](std::alloc::alloc) to allocate memory for this [`ThinColumn`]
    /// The caller should make sure their saved `capacity` value is updated to `new_capacity` after this operation.
    pub(crate) fn alloc(&mut self, new_capacity: NonZeroUsize) {
        self.data.alloc(new_capacity);
        if let Some(ticks) = &mut self.ticks {
            ticks.added.alloc(new_capacity);
            ticks.changed.alloc(new_capacity);
        }
        #[cfg(feature = "track_location")]
        self.changed_by.alloc(new_capacity);
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
        #[cfg(feature = "track_location")] caller: &'static Location<'static>,
    ) {
        self.data.initialize_unchecked(row.as_usize(), data);
        if let Some(ticks) = &mut self.ticks {
            *ticks.added.get_unchecked_mut(row.as_usize()).get_mut() = tick;
            *ticks.changed.get_unchecked_mut(row.as_usize()).get_mut() = tick;
        }
        #[cfg(feature = "track_location")]
        {
            *self.changed_by.get_unchecked_mut(row.as_usize()).get_mut() = caller;
        }
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
        #[cfg(feature = "track_location")] caller: &'static Location<'static>,
    ) {
        self.data.replace_unchecked(row.as_usize(), data);
        if let Some(ticks) = &mut self.ticks {
            *ticks.changed.get_unchecked_mut(row.as_usize()).get_mut() = change_tick;
        }
        #[cfg(feature = "track_location")]
        {
            *self.changed_by.get_unchecked_mut(row.as_usize()).get_mut() = caller;
        }
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
    ///  - either both or neither of `self.ticks` and `other.ticks` must be none
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
        if let Some(ticks) = &mut self.ticks {
            // SAFETY: Must be present because of safety invariants
            let other_ticks = other.ticks.as_mut().unwrap_unchecked();
            // Init added_ticks
            let added_tick = other_ticks
                .added
                .swap_remove_unchecked(src_row.as_usize(), other_last_element_index);
            ticks
                .added
                .initialize_unchecked(dst_row.as_usize(), added_tick);
            // Init changed_ticks
            let changed_tick = other_ticks
                .changed
                .swap_remove_unchecked(src_row.as_usize(), other_last_element_index);
            ticks
                .changed
                .initialize_unchecked(dst_row.as_usize(), changed_tick);
        }
        #[cfg(feature = "track_location")]
        let changed_by = other
            .changed_by
            .swap_remove_unchecked(src_row.as_usize(), other_last_element_index);
        #[cfg(feature = "track_location")]
        self.changed_by
            .initialize_unchecked(dst_row.as_usize(), changed_by);
    }

    /// Call [`Tick::check_tick`] on all of the ticks stored in this column.
    ///
    /// # Safety
    /// `len` is the actual length of this column
    #[inline]
    pub(crate) unsafe fn check_change_ticks(&mut self, len: usize, change_tick: Tick) {
        if let Some(ticks) = &mut self.ticks {
            for i in 0..len {
                // SAFETY:
                // - `i` < `len`
                // we have a mutable reference to `self`
                unsafe { ticks.added.get_unchecked_mut(i) }
                    .get_mut()
                    .check_tick(change_tick);
                // SAFETY:
                // - `i` < `len`
                // we have a mutable reference to `self`
                unsafe { ticks.changed.get_unchecked_mut(i) }
                    .get_mut()
                    .check_tick(change_tick);
            }
        }
    }

    /// Clear all the components from this column.
    ///
    /// # Safety
    /// - `len` must match the actual length of the column
    /// -   The caller must not use the elements this column's data until [`initializing`](Self::initialize) it again (set `len` to 0).
    pub(crate) unsafe fn clear(&mut self, len: usize) {
        self.data.clear(len);
        if let Some(ticks) = &mut self.ticks {
            ticks.added.clear_elements(len);
            ticks.changed.clear_elements(len);
        }
        #[cfg(feature = "track_location")]
        self.changed_by.clear_elements(len);
    }

    /// Because this method needs parameters, it can't be the implementation of the `Drop` trait.
    /// The owner of this [`ThinColumn`] must call this method with the correct information.
    ///
    /// # Safety
    /// - `len` is indeed the length of the column
    /// - `cap` is indeed the capacity of the column
    /// - the data stored in `self` will never be used again
    pub(crate) unsafe fn drop(&mut self, cap: usize, len: usize) {
        self.data.drop(cap, len);
        if let Some(ticks) = &mut self.ticks {
            ticks.added.drop(cap, len);
            ticks.changed.drop(cap, len);
        }
        #[cfg(feature = "track_location")]
        self.changed_by.drop(cap, len);
    }

    /// Drops the last component in this column.
    ///
    /// # Safety
    /// - `last_element_index` is indeed the index of the last element
    /// - the data stored in `last_element_index` will never be used unless properly initialized again.
    pub(crate) unsafe fn drop_last_component(&mut self, last_element_index: usize) {
        if let Some(ticks) = &mut self.ticks {
            core::ptr::drop_in_place(ticks.added.get_unchecked_raw(last_element_index));
            core::ptr::drop_in_place(ticks.changed.get_unchecked_raw(last_element_index));
        }
        self.data.drop_last_element(last_element_index);
        #[cfg(feature = "track_location")]
        core::ptr::drop_in_place(self.changed_by.get_unchecked_raw(last_element_index));
    }

    /// Get a slice to the data stored in this [`ThinColumn`].
    ///
    /// # Safety
    /// - `T` must match the type of data that's stored in this [`ThinColumn`]
    /// - `len` must match the actual length of this column (number of elements stored)
    pub unsafe fn get_data_slice<T>(&self, len: usize) -> &[UnsafeCell<T>] {
        self.data.get_sub_slice(len)
    }

    /// Get the data at a given `row`.
    ///
    /// # Safety
    /// `row` must be within bounds
    pub unsafe fn get_data_unchecked(&self, row: TableRow) -> Ptr<'_> {
        self.data.get_unchecked(row.as_usize())
    }

    /// Get the change detection data.
    #[inline]
    pub fn ticks(&self) -> Option<&ThinColumnTicks> {
        self.ticks.as_ref()
    }

    /// Get the change detection data.
    #[inline]
    pub fn ticks_mut(&mut self) -> Option<&mut ThinColumnTicks> {
        self.ticks.as_mut()
    }

    /// Get a slice to the calling locations that last changed each value in this [`ThinColumn`]
    ///
    /// # Safety
    /// - `len` must match the actual length of this column (number of elements stored)
    #[cfg(feature = "track_location")]
    pub unsafe fn get_changed_by_slice(
        &self,
        len: usize,
    ) -> &[UnsafeCell<&'static Location<'static>>] {
        self.changed_by.as_slice(len)
    }

    /// Allocates change ticks for this column.
    ///
    /// # Safety
    /// - `len` must match the actual length of this column (number of elements stored)
    pub(crate) unsafe fn enable_change_detection(&mut self, len: usize, tick: Tick) {
        if self.ticks.is_some() {
            panic!("ThinColumn already has change ticks");
        }
        let make_ticks = || {
            let mut ticks = ThinArrayPtr::with_capacity(len);
            for i in 0..len {
                ticks.initialize_unchecked(i, UnsafeCell::new(tick));
            }
            ticks
        };
        self.ticks = Some(ThinColumnTicks {
            added: make_ticks(),
            changed: make_ticks(),
        });
    }
}

/// Change detection information for a [`ThinColumn`].
pub struct ThinColumnTicks {
    pub(super) added: ThinArrayPtr<UnsafeCell<Tick>>,
    pub(super) changed: ThinArrayPtr<UnsafeCell<Tick>>,
}

impl ThinColumnTicks {
    /// Fetches the "added" change detection tick for the value at `row`. Unlike [`Column::get_added_tick`]
    /// this function does not do any bounds checking.
    ///
    /// # Safety
    /// `row` must be within the range `[0, self.len())`.
    #[inline]
    pub unsafe fn get_added_tick_unchecked(&self, row: TableRow) -> &UnsafeCell<Tick> {
        self.added.get_unchecked(row.as_usize())
    }

    /// Fetches the "changed" change detection tick for the value at `row`. Unlike [`Column::get_changed_tick`]
    /// this function does not do any bounds checking.
    ///
    /// # Safety
    /// `row` must be within the range `[0, self.len())`.
    #[inline]
    pub unsafe fn get_changed_tick_unchecked(&self, row: TableRow) -> &UnsafeCell<Tick> {
        self.changed.get_unchecked(row.as_usize())
    }

    /// Get a slice to the added [`ticks`](Tick) in this [`ThinColumn`].
    ///
    /// # Safety
    /// - `len` must match the actual length of this column (number of elements stored)
    pub unsafe fn get_added_ticks_slice(&self, len: usize) -> &[UnsafeCell<Tick>] {
        self.added.as_slice(len)
    }

    /// Get a slice to the changed [`ticks`](Tick) in this [`ThinColumn`].
    ///
    /// # Safety
    /// - `len` must match the actual length of this column (number of elements stored)
    pub unsafe fn get_changed_ticks_slice(&self, len: usize) -> &[UnsafeCell<Tick>] {
        self.changed.as_slice(len)
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
    pub(super) ticks: Option<ColumnTicks>,
    #[cfg(feature = "track_location")]
    changed_by: Vec<UnsafeCell<&'static Location<'static>>>,
}

impl Column {
    /// Constructs a new [`Column`], configured with a component's layout and an initial `capacity`.
    #[inline]
    pub(crate) fn with_capacity(component_info: &ComponentInfo, capacity: usize) -> Self {
        Column {
            // SAFETY: component_info.drop() is valid for the types that will be inserted.
            data: unsafe { BlobVec::new(component_info.layout(), component_info.drop(), capacity) },
            ticks: component_info
                .change_detection_enabled()
                .then_some(ColumnTicks {
                    added: Vec::with_capacity(capacity),
                    changed: Vec::with_capacity(capacity),
                }),
            #[cfg(feature = "track_location")]
            changed_by: Vec::with_capacity(capacity),
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
        #[cfg(feature = "track_location")] caller: &'static Location<'static>,
    ) {
        debug_assert!(row.as_usize() < self.len());
        self.data.replace_unchecked(row.as_usize(), data);
        if let Some(ticks) = &mut self.ticks {
            *ticks.changed.get_unchecked_mut(row.as_usize()).get_mut() = change_tick;
        }
        #[cfg(feature = "track_location")]
        {
            *self.changed_by.get_unchecked_mut(row.as_usize()).get_mut() = caller;
        }
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
        self.data.swap_remove_and_drop_unchecked(row.as_usize());
        if let Some(ticks) = &mut self.ticks {
            ticks.added.swap_remove(row.as_usize());
            ticks.changed.swap_remove(row.as_usize());
        }
        #[cfg(feature = "track_location")]
        self.changed_by.swap_remove(row.as_usize());
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
    ) -> (OwningPtr<'_>, Option<ComponentTicks>, MaybeLocation) {
        let data = self.data.swap_remove_and_forget_unchecked(row.as_usize());
        let ticks = if let Some(ticks) = &mut self.ticks {
            let added = ticks.added.swap_remove(row.as_usize()).into_inner();
            let changed = ticks.changed.swap_remove(row.as_usize()).into_inner();
            Some(ComponentTicks { added, changed })
        } else {
            None
        };
        #[cfg(feature = "track_location")]
        let caller = self.changed_by.swap_remove(row.as_usize()).into_inner();
        #[cfg(not(feature = "track_location"))]
        let caller = ();
        (data, ticks, caller)
    }

    /// Pushes a new value onto the end of the [`Column`].
    ///
    /// # Safety
    /// `ptr` must point to valid data of this column's component type
    pub(crate) unsafe fn push(
        &mut self,
        ptr: OwningPtr<'_>,
        ComponentTicks { added, changed }: ComponentTicks,
        #[cfg(feature = "track_location")] caller: &'static Location<'static>,
    ) {
        self.data.push(ptr);
        if let Some(ticks) = &mut self.ticks {
            ticks.added.push(UnsafeCell::new(added));
            ticks.changed.push(UnsafeCell::new(changed));
        }
        #[cfg(feature = "track_location")]
        self.changed_by.push(UnsafeCell::new(caller));
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

    /// Fetches a reference to the data and change detection ticks at `row`.
    ///
    /// Returns `None` if `row` is out of bounds.
    #[inline]
    pub fn get(&self, row: TableRow) -> Option<(Ptr<'_>, Option<TickCells<'_>>)> {
        (row.as_usize() < self.data.len())
            // SAFETY: The row is length checked before fetching the pointer. This is being
            // accessed through a read-only reference to the column.
            .then(|| unsafe {
                (
                    self.data.get_unchecked(row.as_usize()),
                    self.ticks().map(|t| TickCells {
                        added: t.added.get_unchecked(row.as_usize()),
                        changed: t.changed.get_unchecked(row.as_usize()),
                    }),
                )
            })
    }

    /// Fetches a read-only reference to the data at `row`.
    ///
    /// Returns `None` if `row` is out of bounds.
    #[inline]
    pub fn get_data(&self, row: TableRow) -> Option<Ptr<'_>> {
        (row.as_usize() < self.data.len()).then(|| {
            // SAFETY: The row is length checked before fetching the pointer. This is being
            // accessed through a read-only reference to the column.
            unsafe { self.data.get_unchecked(row.as_usize()) }
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
        debug_assert!(row.as_usize() < self.data.len());
        self.data.get_unchecked(row.as_usize())
    }

    /// Fetches a mutable reference to the data at `row`.
    ///
    /// Returns `None` if `row` is out of bounds.
    #[inline]
    pub fn get_data_mut(&mut self, row: TableRow) -> Option<PtrMut<'_>> {
        (row.as_usize() < self.data.len()).then(|| {
            // SAFETY: The row is length checked before fetching the pointer. This is being
            // accessed through an exclusive reference to the column.
            unsafe { self.data.get_unchecked_mut(row.as_usize()) }
        })
    }

    /// Fetches the change detection ticks for the value at `row`.
    ///
    /// Returns `None` if `row` is out of bounds.
    #[inline]
    pub fn get_ticks(&self, row: TableRow) -> Option<Option<ComponentTicks>> {
        if row.as_usize() < self.data.len() {
            // SAFETY: The size of the column has already been checked.
            Some(unsafe { self.get_ticks_unchecked(row) })
        } else {
            None
        }
    }

    /// Fetches the change detection ticks for the value at `row`. Unlike [`Column::get_ticks`]
    /// this function does not do any bounds checking.
    ///
    /// # Safety
    /// `row` must be within the range `[0, self.len())`.
    #[inline]
    pub unsafe fn get_ticks_unchecked(&self, row: TableRow) -> Option<ComponentTicks> {
        self.ticks().map(|t| {
            debug_assert!(row.as_usize() < t.added.len());
            debug_assert!(row.as_usize() < t.changed.len());
            ComponentTicks {
                added: t.added.get_unchecked(row.as_usize()).read(),
                changed: t.changed.get_unchecked(row.as_usize()).read(),
            }
        })
    }

    /// Clears the column, removing all values.
    ///
    /// Note that this function has no effect on the allocated capacity of the [`Column`]>
    pub fn clear(&mut self) {
        self.data.clear();
        if let Some(ticks) = &mut self.ticks {
            ticks.added.clear();
            ticks.changed.clear();
        }
        #[cfg(feature = "track_location")]
        self.changed_by.clear();
    }

    #[inline]
    pub(crate) fn check_change_ticks(&mut self, change_tick: Tick) {
        if let Some(ticks) = &mut self.ticks {
            for component_ticks in &mut ticks.added {
                component_ticks.get_mut().check_tick(change_tick);
            }
            for component_ticks in &mut ticks.changed {
                component_ticks.get_mut().check_tick(change_tick);
            }
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
    #[cfg(feature = "track_location")]
    pub fn get_changed_by(&self, row: TableRow) -> Option<&UnsafeCell<&'static Location<'static>>> {
        self.changed_by.get(row.as_usize())
    }

    /// Fetches the calling location that last changed the value at `row`.
    ///
    /// Unlike [`Column::get_changed_by`] this function does not do any bounds checking.
    ///
    /// # Safety
    /// `row` must be within the range `[0, self.len())`.
    #[inline]
    #[cfg(feature = "track_location")]
    pub unsafe fn get_changed_by_unchecked(
        &self,
        row: TableRow,
    ) -> &UnsafeCell<&'static Location<'static>> {
        debug_assert!(row.as_usize() < self.changed_by.len());
        self.changed_by.get_unchecked(row.as_usize())
    }

    /// Get the change detection data.
    #[inline]
    pub fn ticks(&self) -> Option<&ColumnTicks> {
        self.ticks.as_ref()
    }

    /// Get the change detection data.
    #[inline]
    pub fn ticks_mut(&mut self) -> Option<&mut ColumnTicks> {
        self.ticks.as_mut()
    }

    /// Allocates change ticks for this column.
    pub(crate) fn enable_change_detection(&mut self, tick: Tick) {
        let make_ticks = || (0..self.len()).map(|_| UnsafeCell::new(tick)).collect();
        self.ticks = Some(ColumnTicks {
            added: make_ticks(),
            changed: make_ticks(),
        });
    }
}

/// Change detection information for a [`Column`].
#[derive(Debug)]
pub struct ColumnTicks {
    pub(super) added: Vec<UnsafeCell<Tick>>,
    pub(super) changed: Vec<UnsafeCell<Tick>>,
}

impl ColumnTicks {
    /// Fetches the slice to the [`Column`]'s "added" change detection ticks.
    ///
    /// Note: The values stored within are [`UnsafeCell`].
    /// Users of this API must ensure that accesses to each individual element
    /// adhere to the safety invariants of [`UnsafeCell`].
    #[inline]
    pub fn get_added_ticks_slice(&self) -> &[UnsafeCell<Tick>] {
        &self.added
    }

    /// Fetches the slice to the [`Column`]'s "changed" change detection ticks.
    ///
    /// Note: The values stored within are [`UnsafeCell`].
    /// Users of this API must ensure that accesses to each individual element
    /// adhere to the safety invariants of [`UnsafeCell`].
    #[inline]
    pub fn get_changed_ticks_slice(&self) -> &[UnsafeCell<Tick>] {
        &self.changed
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
        self.added.get(row.as_usize())
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
        self.changed.get(row.as_usize())
    }

    /// Fetches the "added" change detection tick for the value at `row`. Unlike [`Column::get_added_tick`]
    /// this function does not do any bounds checking.
    ///
    /// # Safety
    /// `row` must be within the range `[0, self.len())`.
    #[inline]
    pub unsafe fn get_added_tick_unchecked(&self, row: TableRow) -> &UnsafeCell<Tick> {
        debug_assert!(row.as_usize() < self.added.len());
        self.added.get_unchecked(row.as_usize())
    }

    /// Fetches the "changed" change detection tick for the value at `row`. Unlike [`Column::get_changed_tick`]
    /// this function does not do any bounds checking.
    ///
    /// # Safety
    /// `row` must be within the range `[0, self.len())`.
    #[inline]
    pub unsafe fn get_changed_tick_unchecked(&self, row: TableRow) -> &UnsafeCell<Tick> {
        debug_assert!(row.as_usize() < self.changed.len());
        self.changed.get_unchecked(row.as_usize())
    }
}
