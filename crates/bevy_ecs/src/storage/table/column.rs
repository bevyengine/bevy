use super::*;
use crate::storage::{
    blob_array::{new_blob_array, BlobArray, BlobArrayCreation},
    thin_array_ptr::ThinArrayPtr,
};

/// A type-erased contiguous container for data of a homogeneous type.
///
/// Conceptually, a [`Column`] is very similar to a type-erased `Vec<T>`.
/// It also stores the change detection ticks for its components, kept in two separate
/// contiguous buffers internally. An element shares its data across these buffers by using the
/// same index (i.e. the entity at row 3 has it's data at index 3 and its change detection ticks at
/// index 3). A slice to these contiguous blocks of memory can be fetched
/// via [`Column::get_data_slice`], [`Column::get_added_ticks_slice`], and
/// [`Column::get_changed_ticks_slice`].
///
/// For performance reasons, [`Column`] doesn't store its buffers' length and capacity, it relies on
/// the object that stores it to keep track of them. Additionally, [`Column`] distinguishes between [`ZSTs`]
/// and non-[`ZSTs`], for performance reasons as well.
///
/// Like many other low-level storage types, [`Column`] has a limited and highly unsafe
/// interface. It's highly advised to use higher level types and their safe abstractions
/// instead of working directly with [`Column`].
///
/// [`ZSTs`]: https://doc.rust-lang.org/nomicon/exotic-sizes.html#zero-sized-types-zsts
pub struct Column<const IS_ZST: bool> {
    pub(super) data: BlobArray<IS_ZST>,
    pub(super) added_ticks: ThinArrayPtr<UnsafeCell<Tick>>,
    pub(super) changed_ticks: ThinArrayPtr<UnsafeCell<Tick>>,
}

#[doc(hidden)]
pub enum ColumnCreationResult {
    ZST(Column<true>),
    NotZST(Column<false>),
}

/// Create a new [`Column`] for the given [`ComponentInfo`] with some `capacity`
pub(super) fn column_with_capacity(
    capacity: usize,
    component_info: &ComponentInfo,
) -> ColumnCreationResult {
    // SAFETY: `ComponentInfo` has the correct `Layout` and drop function
    let blob_arr = unsafe { new_blob_array(component_info.layout(), component_info.drop()) };
    let added_ticks = ThinArrayPtr::with_capacity(capacity);
    let changed_ticks = ThinArrayPtr::with_capacity(capacity);

    match blob_arr {
        BlobArrayCreation::NotZST(mut data) => {
            if let Some(cap) = NonZeroUsize::new(capacity) {
                data.alloc(cap);
            }
            ColumnCreationResult::NotZST(Column {
                data,
                added_ticks,
                changed_ticks,
            })
        }
        BlobArrayCreation::ZST(data) => ColumnCreationResult::ZST(Column {
            data,
            added_ticks,
            changed_ticks,
        }),
    }
}

impl<const IS_ZST: bool> Column<IS_ZST> {
    /// Swap-remove and drop the removed element.
    ///
    /// # Safety
    /// The caller must:
    /// - ensure that `row.as_usize()` < `len`
    /// - ensure that `last_element_index` = `len - 1`
    /// - either _update the `len`_ to `len - 1`, or immidiatly initialize another element in the `last_element_index`
    pub unsafe fn swap_remove_and_drop_unchecked(
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
    }

    /// Swap-remove and forget the removed element (caller's responsibility to drop)
    ///
    /// # Safety
    /// The caller must:
    /// - ensure that `row.as_usize()` < `len`
    /// - ensure that `last_element_index` = `len - 1`
    /// - either _update the `len`_ to `len - 1`, or immidiatly initialize another element in the `last_element_index`
    pub unsafe fn swap_remove_and_forget_unchecked(
        &mut self,
        last_element_index: usize,
        row: TableRow,
    ) {
        self.data
            .swap_remove_and_forget_unchecked(row.as_usize(), last_element_index);
        self.added_ticks
            .swap_remove_and_forget_unchecked(row.as_usize(), last_element_index);
        self.changed_ticks
            .swap_remove_and_forget_unchecked(row.as_usize(), last_element_index);
    }

    // TODO: docs
    pub unsafe fn realloc(&mut self, current_capacity: NonZeroUsize, new_capacity: NonZeroUsize) {
        self.data.realloc(current_capacity, new_capacity);
        self.added_ticks.realloc(current_capacity, new_capacity);
        self.changed_ticks.realloc(current_capacity, new_capacity);
    }

    // TODO: docs
    pub unsafe fn alloc(&mut self, new_capacity: NonZeroUsize) {
        self.data.alloc(new_capacity);
        self.added_ticks.alloc(new_capacity);
        self.changed_ticks.alloc(new_capacity);
    }

    /// Writes component data to the column at given row.
    /// Assumes the slot is uninitialized, drop is not called.
    /// To overwrite existing initialized value, use `replace` instead.
    ///
    /// # Safety
    /// The caller must ensure that `row.as_usize()` < `len`
    #[inline]
    pub(crate) unsafe fn initialize(&mut self, row: TableRow, data: OwningPtr<'_>, tick: Tick) {
        self.data.initialize_unchecked(row.as_usize(), data);
        *self.added_ticks.get_unchecked_mut(row.as_usize()).get_mut() = tick;
        *self
            .changed_ticks
            .get_unchecked_mut(row.as_usize())
            .get_mut() = tick;
    }

    /// Writes component data to the column at given row.
    /// Assumes the slot is initialized, calls drop.
    ///
    /// # Safety
    /// The caller must ensure that `row.as_usize()` < `len`
    #[inline]
    pub(crate) unsafe fn replace(&mut self, row: TableRow, data: OwningPtr<'_>, change_tick: Tick) {
        self.data.replace_unchecked(row.as_usize(), data);
        *self
            .changed_ticks
            .get_unchecked_mut(row.as_usize())
            .get_mut() = change_tick;
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
        other: &mut Column<IS_ZST>,
        other_last_element_index: usize,
        src_row: TableRow,
        dst_row: TableRow,
    ) {
        debug_assert!(self.data.layout() == other.data.layout());
        // Init the data
        let src_val = other
            .data
            .swap_remove_and_forget_unchecked(src_row.as_usize(), other_last_element_index);
        self.data.initialize_unchecked(dst_row.as_usize(), src_val);
        // Init added_ticks
        let added_tick_ptr = other
            .added_ticks
            .swap_remove_and_forget_unchecked(src_row.as_usize(), other_last_element_index);
        let added_tick = std::ptr::read(added_tick_ptr);
        self.added_ticks
            .initialize_unchecked(dst_row.as_usize(), added_tick);
        // Init changed_ticks
        let changed_tick_ptr = other
            .changed_ticks
            .swap_remove_and_forget_unchecked(src_row.as_usize(), other_last_element_index);
        let changed_tick = std::ptr::read(changed_tick_ptr);
        self.changed_ticks
            .initialize_unchecked(dst_row.as_usize(), changed_tick);
    }

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

            // - `i` < `len`
            // we have a mutable reference to `self`
            unsafe { self.changed_ticks.get_unchecked_mut(i) }
                .get_mut()
                .check_tick(change_tick);
        }
    }

    pub(crate) unsafe fn clear(&mut self, len: usize) {
        self.data.clear_elements(len);
        self.added_ticks.clear_elements(len);
        self.changed_ticks.clear_elements(len);
    }
}

//
//
//
//
