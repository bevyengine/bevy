use super::blob_vec::array_layout;
use crate::storage::blob_vec::array_layout_unchecked;
use alloc::alloc::handle_alloc_error;
use bevy_ptr::{OwningPtr, Ptr, PtrMut};
use bevy_utils::OnDrop;
use core::{alloc::Layout, cell::UnsafeCell, num::NonZeroUsize, ptr::NonNull};

/// A flat, type-erased data storage type similar to a [`BlobVec`](super::blob_vec::BlobVec), but with the length and capacity cut out
/// for performance reasons. This type is reliant on its owning type to store the capacity and length information.
///
/// Used to densely store homogeneous ECS data. A blob is usually just an arbitrary block of contiguous memory without any identity, and
/// could be used to represent any arbitrary data (i.e. string, arrays, etc). This type only stores meta-data about the Blob that it stores,
/// and a pointer to the location of the start of the array, similar to a C array.
pub(super) struct BlobArray {
    item_layout: Layout,
    // the `data` ptr's layout is always `array_layout(item_layout, capacity)`
    data: NonNull<u8>,
    // None if the underlying type doesn't need to be dropped
    pub drop: Option<unsafe fn(OwningPtr<'_>)>,
    #[cfg(debug_assertions)]
    capacity: usize,
}

impl BlobArray {
    /// Create a new [`BlobArray`] with a specified `capacity`.
    /// If `capacity` is 0, no allocations will be made.
    ///
    /// `drop` is an optional function pointer that is meant to be invoked when any element in the [`BlobArray`]
    /// should be dropped. For all Rust-based types, this should match 1:1 with the implementation of [`Drop`]
    /// if present, and should be `None` if `T: !Drop`. For non-Rust based types, this should match any cleanup
    /// processes typically associated with the stored element.
    ///
    /// # Safety
    /// `drop` should be safe to call with an [`OwningPtr`] pointing to any item that's been placed into this [`BlobArray`].
    /// If `drop` is `None`, the items will be leaked. This should generally be set as None based on [`needs_drop`].
    ///
    /// [`needs_drop`]: std::mem::needs_drop
    pub unsafe fn with_capacity(
        item_layout: Layout,
        drop_fn: Option<unsafe fn(OwningPtr<'_>)>,
        capacity: usize,
    ) -> Self {
        if capacity == 0 {
            let align = NonZeroUsize::new(item_layout.align()).expect("alignment must be > 0");
            let data = bevy_ptr::dangling_with_align(align);
            Self {
                item_layout,
                drop: drop_fn,
                data,
                #[cfg(debug_assertions)]
                capacity,
            }
        } else {
            let mut arr = Self::with_capacity(item_layout, drop_fn, 0);
            // SAFETY: `capacity` > 0
            unsafe { arr.alloc(NonZeroUsize::new_unchecked(capacity)) }
            arr
        }
    }

    /// Returns the [`Layout`] of the element type stored in the vector.
    #[inline]
    pub fn layout(&self) -> Layout {
        self.item_layout
    }

    /// Return `true` if this [`BlobArray`] stores `ZSTs`.
    pub fn is_zst(&self) -> bool {
        self.item_layout.size() == 0
    }

    /// Returns a reference to the element at `index`, without doing bounds checking.
    ///
    /// *`len` refers to the length of the array, the number of elements that have been initialized, and are safe to read.
    /// Just like [`Vec::len`], or [`BlobVec::len`](super::blob_vec::BlobVec::len).*
    ///
    /// # Safety
    /// - The element at index `index` is safe to access.
    ///   (If the safety requirements of every method that has been used on `Self` have been fulfilled, the caller just needs to ensure that `index` < `len`)
    ///
    /// [`Vec::len`]: alloc::vec::Vec::len
    #[inline]
    pub unsafe fn get_unchecked(&self, index: usize) -> Ptr<'_> {
        #[cfg(debug_assertions)]
        debug_assert!(index < self.capacity);
        let size = self.item_layout.size();
        // SAFETY:
        // - The caller ensures that `index` fits in this array,
        //   so this operation will not overflow the original allocation.
        // - `size` is a multiple of the erased type's alignment,
        //   so adding a multiple of `size` will preserve alignment.
        unsafe { self.get_ptr().byte_add(index * size) }
    }

    /// Returns a mutable reference to the element at `index`, without doing bounds checking.
    ///
    /// *`len` refers to the length of the array, the number of elements that have been initialized, and are safe to read.
    /// Just like [`Vec::len`], or [`BlobVec::len`](super::blob_vec::BlobVec::len).*
    ///
    /// # Safety
    /// - The element with at index `index` is safe to access.
    ///   (If the safety requirements of every method that has been used on `Self` have been fulfilled, the caller just needs to ensure that `index` < `len`)
    ///
    /// [`Vec::len`]: alloc::vec::Vec::len
    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> PtrMut<'_> {
        #[cfg(debug_assertions)]
        debug_assert!(index < self.capacity);
        let size = self.item_layout.size();
        // SAFETY:
        // - The caller ensures that `index` fits in this vector,
        //   so this operation will not overflow the original allocation.
        // - `size` is a multiple of the erased type's alignment,
        //  so adding a multiple of `size` will preserve alignment.
        unsafe { self.get_ptr_mut().byte_add(index * size) }
    }

    /// Gets a [`Ptr`] to the start of the array
    #[inline]
    pub fn get_ptr(&self) -> Ptr<'_> {
        // SAFETY: the inner data will remain valid for as long as 'self.
        unsafe { Ptr::new(self.data) }
    }

    /// Gets a [`PtrMut`] to the start of the array
    #[inline]
    pub fn get_ptr_mut(&mut self) -> PtrMut<'_> {
        // SAFETY: the inner data will remain valid for as long as 'self.
        unsafe { PtrMut::new(self.data) }
    }

    /// Get a slice of the first `slice_len` elements in [`BlobArray`] as if it were an array with elements of type `T`
    /// To get a slice to the entire array, the caller must plug `len` in `slice_len`.
    ///
    /// *`len` refers to the length of the array, the number of elements that have been initialized, and are safe to read.
    /// Just like [`Vec::len`], or [`BlobVec::len`](super::blob_vec::BlobVec::len).*
    ///
    /// # Safety
    /// - The type `T` must be the type of the items in this [`BlobArray`].
    /// - `slice_len` <= `len`
    ///
    /// [`Vec::len`]: alloc::vec::Vec::len
    pub unsafe fn get_sub_slice<T>(&self, slice_len: usize) -> &[UnsafeCell<T>] {
        #[cfg(debug_assertions)]
        debug_assert!(slice_len <= self.capacity);
        // SAFETY: the inner data will remain valid for as long as 'self.
        unsafe {
            core::slice::from_raw_parts(self.data.as_ptr() as *const UnsafeCell<T>, slice_len)
        }
    }

    /// Clears the array, i.e. removing (and dropping) all of the elements.
    /// Note that this method has no effect on the allocated capacity of the vector.
    ///
    /// Note that this method will behave exactly the same as [`Vec::clear`].
    ///
    /// # Safety
    /// - For every element with index `i`, if `i` < `len`: It must be safe to call [`Self::get_unchecked_mut`] with `i`.
    ///   (If the safety requirements of every method that has been used on `Self` have been fulfilled, the caller just needs to ensure that `len` is correct.)
    ///
    /// [`Vec::clear`]: alloc::vec::Vec::clear
    pub unsafe fn clear(&mut self, len: usize) {
        #[cfg(debug_assertions)]
        debug_assert!(self.capacity >= len);
        if let Some(drop) = self.drop {
            // We set `self.drop` to `None` before dropping elements for unwind safety. This ensures we don't
            // accidentally drop elements twice in the event of a drop impl panicking.
            self.drop = None;
            let size = self.item_layout.size();
            for i in 0..len {
                // SAFETY:
                // * 0 <= `i` < `len`, so `i * size` must be in bounds for the allocation.
                // * `size` is a multiple of the erased type's alignment,
                //   so adding a multiple of `size` will preserve alignment.
                // * The item is left unreachable so it can be safely promoted to an `OwningPtr`.
                let item = unsafe { self.get_ptr_mut().byte_add(i * size).promote() };
                // SAFETY: `item` was obtained from this `BlobArray`, so its underlying type must match `drop`.
                unsafe { drop(item) };
            }
            self.drop = Some(drop);
        }
    }

    /// Because this method needs parameters, it can't be the implementation of the `Drop` trait.
    /// The owner of this [`BlobArray`] must call this method with the correct information.
    ///
    /// # Safety
    /// - `cap` and `len` are indeed the capacity and length of this [`BlobArray`]
    /// - This [`BlobArray`] mustn't be used after calling this method.
    pub unsafe fn drop(&mut self, cap: usize, len: usize) {
        #[cfg(debug_assertions)]
        debug_assert_eq!(self.capacity, cap);
        if cap != 0 {
            self.clear(len);
            if !self.is_zst() {
                let layout =
                    array_layout(&self.item_layout, cap).expect("array layout should be valid");
                alloc::alloc::dealloc(self.data.as_ptr().cast(), layout);
            }
            #[cfg(debug_assertions)]
            {
                self.capacity = 0;
            }
        }
    }

    /// Drops the last element in this [`BlobArray`].
    ///
    /// # Safety
    // - `last_element_index` must correspond to the last element in the array.
    // - After this method is called, the last element must not be used
    // unless [`Self::initialize_unchecked`] is called to set the value of the last element.
    pub unsafe fn drop_last_element(&mut self, last_element_index: usize) {
        #[cfg(debug_assertions)]
        debug_assert!(self.capacity > last_element_index);
        if let Some(drop) = self.drop {
            // We set `self.drop` to `None` before dropping elements for unwind safety. This ensures we don't
            // accidentally drop elements twice in the event of a drop impl panicking.
            self.drop = None;
            // SAFETY:
            let item = self.get_unchecked_mut(last_element_index).promote();
            // SAFETY:
            unsafe { drop(item) };
            self.drop = Some(drop);
        }
    }

    /// Allocate a block of memory for the array. This should be used to initialize the array, do not use this
    /// method if there are already elements stored in the array - use [`Self::realloc`] instead.
    pub(super) fn alloc(&mut self, capacity: NonZeroUsize) {
        #[cfg(debug_assertions)]
        debug_assert_eq!(self.capacity, 0);
        if !self.is_zst() {
            let new_layout = array_layout(&self.item_layout, capacity.get())
                .expect("array layout should be valid");
            // SAFETY: layout has non-zero size because capacity > 0, and the blob isn't ZST (`self.is_zst` == false)
            let new_data = unsafe { alloc::alloc::alloc(new_layout) };
            self.data = NonNull::new(new_data).unwrap_or_else(|| handle_alloc_error(new_layout));
        }
        #[cfg(debug_assertions)]
        {
            self.capacity = capacity.into();
        }
    }

    /// Reallocate memory for this array.
    /// For example, if the length (number of stored elements) reached the capacity (number of elements the current allocation can store),
    /// you might want to use this method to increase the allocation, so more data can be stored in the array.
    ///
    /// # Safety
    /// - `current_capacity` is indeed the current capacity of this array.
    /// - After calling this method, the caller must update their saved capacity to reflect the change.
    pub(super) unsafe fn realloc(
        &mut self,
        current_capacity: NonZeroUsize,
        new_capacity: NonZeroUsize,
    ) {
        #[cfg(debug_assertions)]
        debug_assert_eq!(self.capacity, current_capacity.get());
        if !self.is_zst() {
            // SAFETY: `new_capacity` can't overflow usize
            let new_layout =
                unsafe { array_layout_unchecked(&self.item_layout, new_capacity.get()) };
            // SAFETY:
            // - ptr was be allocated via this allocator
            // - the layout used to previously allocate this array is equivalent to `array_layout(&self.item_layout, current_capacity.get())`
            // - `item_layout.size() > 0` (`self.is_zst`==false) and `new_capacity > 0`, so the layout size is non-zero
            // - "new_size, when rounded up to the nearest multiple of layout.align(), must not overflow (i.e., the rounded value must be less than usize::MAX)",
            // since the item size is always a multiple of its align, the rounding cannot happen
            // here and the overflow is handled in `array_layout`
            let new_data = unsafe {
                alloc::alloc::realloc(
                    self.get_ptr_mut().as_ptr(),
                    // SAFETY: This is the Layout of the current array, it must be valid, if it hadn't have been, there would have been a panic on a previous allocation
                    array_layout_unchecked(&self.item_layout, current_capacity.get()),
                    new_layout.size(),
                )
            };
            self.data = NonNull::new(new_data).unwrap_or_else(|| handle_alloc_error(new_layout));
        }
        #[cfg(debug_assertions)]
        {
            self.capacity = new_capacity.into();
        }
    }

    /// Initializes the value at `index` to `value`. This function does not do any bounds checking.
    ///
    /// # Safety
    /// - `index` must be in bounds (`index` < capacity)
    /// - The [`Layout`] of the value must match the layout of the blobs stored in this array,
    ///   and it must be safe to use the `drop` function of this [`BlobArray`] to drop `value`.
    /// - `value` must not point to the same value that is being initialized.
    #[inline]
    pub unsafe fn initialize_unchecked(&mut self, index: usize, value: OwningPtr<'_>) {
        #[cfg(debug_assertions)]
        debug_assert!(self.capacity > index);
        let size = self.item_layout.size();
        let dst = self.get_unchecked_mut(index);
        core::ptr::copy::<u8>(value.as_ptr(), dst.as_ptr(), size);
    }

    /// Replaces the value at `index` with `value`. This function does not do any bounds checking.
    ///
    /// # Safety
    /// - Index must be in-bounds (`index` < `len`)
    /// - `value`'s [`Layout`] must match this [`BlobArray`]'s `item_layout`,
    ///   and it must be safe to use the `drop` function of this [`BlobArray`] to drop `value`.
    /// - `value` must not point to the same value that is being replaced.
    pub unsafe fn replace_unchecked(&mut self, index: usize, value: OwningPtr<'_>) {
        #[cfg(debug_assertions)]
        debug_assert!(self.capacity > index);
        // Pointer to the value in the vector that will get replaced.
        // SAFETY: The caller ensures that `index` fits in this vector.
        let destination = NonNull::from(unsafe { self.get_unchecked_mut(index) });
        let source = value.as_ptr();

        if let Some(drop) = self.drop {
            // We set `self.drop` to `None` before dropping elements for unwind safety. This ensures we don't
            // accidentally drop elements twice in the event of a drop impl panicking.
            self.drop = None;

            // Transfer ownership of the old value out of the vector, so it can be dropped.
            // SAFETY:
            // - `destination` was obtained from a `PtrMut` in this vector, which ensures it is non-null,
            //   well-aligned for the underlying type, and has proper provenance.
            // - The storage location will get overwritten with `value` later, which ensures
            //   that the element will not get observed or double dropped later.
            // - If a panic occurs, `self.len` will remain `0`, which ensures a double-drop
            //   does not occur. Instead, all elements will be forgotten.
            let old_value = unsafe { OwningPtr::new(destination) };

            // This closure will run in case `drop()` panics,
            // which ensures that `value` does not get forgotten.
            let on_unwind = OnDrop::new(|| drop(value));

            drop(old_value);

            // If the above code does not panic, make sure that `value` doesn't get dropped.
            core::mem::forget(on_unwind);

            self.drop = Some(drop);
        }

        // Copy the new value into the vector, overwriting the previous value.
        // SAFETY:
        // - `source` and `destination` were obtained from `OwningPtr`s, which ensures they are
        //   valid for both reads and writes.
        // - The value behind `source` will only be dropped if the above branch panics,
        //   so it must still be initialized and it is safe to transfer ownership into the vector.
        // - `source` and `destination` were obtained from different memory locations,
        //   both of which we have exclusive access to, so they are guaranteed not to overlap.
        unsafe {
            core::ptr::copy_nonoverlapping::<u8>(
                source,
                destination.as_ptr(),
                self.item_layout.size(),
            );
        }
    }

    /// This method will swap two elements in the array, and return the one at `index_to_remove`.
    /// It is the caller's responsibility to drop the returned pointer, if that is desirable.
    ///
    /// # Safety
    /// - `index_to_keep` must be safe to access (within the bounds of the length of the array).
    /// - `index_to_remove` must be safe to access (within the bounds of the length of the array).
    /// - `index_to_remove` != `index_to_keep`
    /// -  The caller should address the inconsistent state of the array that has occurred after the swap, either:
    ///     1) initialize a different value in `index_to_keep`
    ///     2) update the saved length of the array if `index_to_keep` was the last element.
    #[inline]
    #[must_use = "The returned pointer should be used to drop the removed element"]
    pub unsafe fn swap_remove_unchecked(
        &mut self,
        index_to_remove: usize,
        index_to_keep: usize,
    ) -> OwningPtr<'_> {
        #[cfg(debug_assertions)]
        {
            debug_assert!(self.capacity > index_to_keep);
            debug_assert!(self.capacity > index_to_remove);
        }
        if index_to_remove != index_to_keep {
            return self.swap_remove_unchecked_nonoverlapping(index_to_remove, index_to_keep);
        }
        // Now the element that used to be in index `index_to_remove` is now in index `index_to_keep` (after swap)
        // If we are storing ZSTs than the index doesn't actually matter because the size is 0.
        self.get_unchecked_mut(index_to_keep).promote()
    }

    /// The same as [`Self::swap_remove_unchecked`] but the two elements must non-overlapping.
    ///
    /// # Safety
    /// - `index_to_keep` must be safe to access (within the bounds of the length of the array).
    /// - `index_to_remove` must be safe to access (within the bounds of the length of the array).
    /// - `index_to_remove` != `index_to_keep`
    /// -  The caller should address the inconsistent state of the array that has occurred after the swap, either:
    ///     1) initialize a different value in `index_to_keep`
    ///     2) update the saved length of the array if `index_to_keep` was the last element.
    #[inline]
    pub unsafe fn swap_remove_unchecked_nonoverlapping(
        &mut self,
        index_to_remove: usize,
        index_to_keep: usize,
    ) -> OwningPtr<'_> {
        #[cfg(debug_assertions)]
        {
            debug_assert!(self.capacity > index_to_keep);
            debug_assert!(self.capacity > index_to_remove);
            debug_assert_ne!(index_to_keep, index_to_remove);
        }
        debug_assert_ne!(index_to_keep, index_to_remove);
        core::ptr::swap_nonoverlapping::<u8>(
            self.get_unchecked_mut(index_to_keep).as_ptr(),
            self.get_unchecked_mut(index_to_remove).as_ptr(),
            self.item_layout.size(),
        );
        // Now the element that used to be in index `index_to_remove` is now in index `index_to_keep` (after swap)
        // If we are storing ZSTs than the index doesn't actually matter because the size is 0.
        self.get_unchecked_mut(index_to_keep).promote()
    }

    /// This method will call [`Self::swap_remove_unchecked`] and drop the result.
    ///
    /// # Safety
    /// - `index_to_keep` must be safe to access (within the bounds of the length of the array).
    /// - `index_to_remove` must be safe to access (within the bounds of the length of the array).
    /// - `index_to_remove` != `index_to_keep`
    /// -  The caller should address the inconsistent state of the array that has occurred after the swap, either:
    ///     1) initialize a different value in `index_to_keep`
    ///     2) update the saved length of the array if `index_to_keep` was the last element.
    #[inline]
    pub unsafe fn swap_remove_and_drop_unchecked(
        &mut self,
        index_to_remove: usize,
        index_to_keep: usize,
    ) {
        #[cfg(debug_assertions)]
        {
            debug_assert!(self.capacity > index_to_keep);
            debug_assert!(self.capacity > index_to_remove);
        }
        let drop = self.drop;
        let value = self.swap_remove_unchecked(index_to_remove, index_to_keep);
        if let Some(drop) = drop {
            drop(value);
        }
    }

    /// The same as [`Self::swap_remove_and_drop_unchecked`] but the two elements must non-overlapping.
    ///
    /// # Safety
    /// - `index_to_keep` must be safe to access (within the bounds of the length of the array).
    /// - `index_to_remove` must be safe to access (within the bounds of the length of the array).
    /// - `index_to_remove` != `index_to_keep`
    /// -  The caller should address the inconsistent state of the array that has occurred after the swap, either:
    ///     1) initialize a different value in `index_to_keep`
    ///     2) update the saved length of the array if `index_to_keep` was the last element.
    #[inline]
    pub unsafe fn swap_remove_and_drop_unchecked_nonoverlapping(
        &mut self,
        index_to_remove: usize,
        index_to_keep: usize,
    ) {
        #[cfg(debug_assertions)]
        {
            debug_assert!(self.capacity > index_to_keep);
            debug_assert!(self.capacity > index_to_remove);
            debug_assert_ne!(index_to_keep, index_to_remove);
        }
        let drop = self.drop;
        let value = self.swap_remove_unchecked_nonoverlapping(index_to_remove, index_to_keep);
        if let Some(drop) = drop {
            drop(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::prelude::*;

    #[derive(Component)]
    struct PanicOnDrop;

    impl Drop for PanicOnDrop {
        fn drop(&mut self) {
            panic!("PanicOnDrop is being Dropped");
        }
    }

    #[test]
    #[should_panic(expected = "PanicOnDrop is being Dropped")]
    fn make_sure_zst_components_get_dropped() {
        let mut world = World::new();

        world.spawn(PanicOnDrop);
    }
}
