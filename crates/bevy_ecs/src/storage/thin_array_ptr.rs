use crate::query::DebugCheckedUnwrap;
use alloc::{
    alloc::{alloc, handle_alloc_error, realloc},
    boxed::Box,
};
use core::{
    alloc::Layout,
    mem::{needs_drop, size_of},
    num::NonZeroUsize,
    ptr::{self, NonNull},
};

/// Similar to [`Vec<T>`], but with the capacity and length cut out for performance reasons.
///
/// This type can be treated as a `ManuallyDrop<Box<[T]>>` without a built in length. To avoid
/// memory leaks, [`drop`](Self::drop) must be called when no longer in use.
///
/// [`Vec<T>`]: alloc::vec::Vec
pub struct ThinArrayPtr<T> {
    data: NonNull<T>,
    #[cfg(debug_assertions)]
    capacity: usize,
}

impl<T> ThinArrayPtr<T> {
    fn empty() -> Self {
        #[cfg(debug_assertions)]
        {
            Self {
                data: NonNull::dangling(),
                capacity: 0,
            }
        }
        #[cfg(not(debug_assertions))]
        {
            Self {
                data: NonNull::dangling(),
            }
        }
    }

    #[inline(always)]
    fn set_capacity(&mut self, _capacity: usize) {
        #[cfg(debug_assertions)]
        {
            self.capacity = _capacity;
        }
    }

    /// Create a new [`ThinArrayPtr`] with a given capacity. If the `capacity` is 0, this will no allocate any memory.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        let mut arr = Self::empty();
        if capacity > 0 {
            // SAFETY:
            // - The `current_capacity` is 0 because it was just created
            unsafe { arr.alloc(NonZeroUsize::new_unchecked(capacity)) };
        }
        arr
    }

    /// Allocate memory for the array, this should only be used if not previous allocation has been made (capacity = 0)
    /// The caller should update their saved `capacity` value to reflect the fact that it was changed
    ///
    /// # Panics
    /// - Panics if the new capacity overflows `usize`
    pub fn alloc(&mut self, capacity: NonZeroUsize) {
        self.set_capacity(capacity.get());
        if size_of::<T>() != 0 {
            let new_layout = Layout::array::<T>(capacity.get())
                .expect("layout should be valid (arithmetic overflow)");
            // SAFETY:
            // - layout has non-zero size, `capacity` > 0, `size` > 0 (`size_of::<T>() != 0`)
            self.data = NonNull::new(unsafe { alloc(new_layout) })
                .unwrap_or_else(|| handle_alloc_error(new_layout))
                .cast();
        }
    }

    /// Reallocate memory for the array, this should only be used if a previous allocation for this array has been made (capacity > 0).
    ///
    /// # Panics
    /// - Panics if the new capacity overflows `usize`
    ///
    /// # Safety
    /// - The current capacity is indeed greater than 0
    /// - The caller should update their saved `capacity` value to reflect the fact that it was changed
    pub unsafe fn realloc(&mut self, current_capacity: NonZeroUsize, new_capacity: NonZeroUsize) {
        #[cfg(debug_assertions)]
        assert_eq!(self.capacity, current_capacity.get());
        self.set_capacity(new_capacity.get());
        if size_of::<T>() != 0 {
            let new_layout =
                Layout::array::<T>(new_capacity.get()).expect("overflow while allocating memory");
            // SAFETY:
            // - ptr was be allocated via this allocator
            // - the layout of the array is the same as `Layout::array::<T>(current_capacity)`
            // - the size of `T` is non 0, and `new_capacity` > 0
            // - "new_size, when rounded up to the nearest multiple of layout.align(), must not overflow (i.e., the rounded value must be less than usize::MAX)",
            // since the item size is always a multiple of its align, the rounding cannot happen
            // here and the overflow is handled in `Layout::array`
            self.data = NonNull::new(unsafe {
                realloc(
                    self.data.cast().as_ptr(),
                    // We can use `unwrap_unchecked` because this is the Layout of the current allocation, it must be valid
                    Layout::array::<T>(current_capacity.get()).debug_checked_unwrap(),
                    new_layout.size(),
                )
            })
            .unwrap_or_else(|| handle_alloc_error(new_layout))
            .cast();
        }
    }

    /// Initializes the value at `index` to `value`. This function does not do any bounds checking.
    ///
    /// # Safety
    /// `index` must be in bounds i.e. within the `capacity`.
    /// if `index` = `len` the caller should update their saved `len` value to reflect the fact that it was changed
    #[inline]
    pub unsafe fn initialize_unchecked(&mut self, index: usize, value: T) {
        // SAFETY: `index` is in bounds
        let ptr = unsafe { self.get_unchecked_raw(index) };
        // SAFETY: `index` is in bounds, therefore the pointer to that location in the array is valid, and aligned.
        unsafe { ptr::write(ptr, value) };
    }

    /// Get a raw pointer to the element at `index`. This method doesn't do any bounds checking.
    ///
    /// # Safety
    /// - `index` must be safe to access.
    #[inline]
    pub unsafe fn get_unchecked_raw(&mut self, index: usize) -> *mut T {
        // SAFETY:
        // - `self.data` and the resulting pointer are in the same allocated object
        // - the memory address of the last element doesn't overflow `isize`, so if `index` is in bounds, it won't overflow either
        unsafe { self.data.as_ptr().add(index) }
    }

    /// Get a reference to the element at `index`. This method doesn't do any bounds checking.
    ///
    /// # Safety
    /// - `index` must be safe to read.
    #[inline]
    pub unsafe fn get_unchecked(&self, index: usize) -> &'_ T {
        // SAFETY:
        // - `self.data` and the resulting pointer are in the same allocated object
        // - the memory address of the last element doesn't overflow `isize`, so if `index` is in bounds, it won't overflow either
        let ptr = unsafe { self.data.as_ptr().add(index) };
        // SAFETY:
        // - The pointer is properly aligned
        // - It is dereferenceable (all in the same allocation)
        // - `index` < `len` and the element is safe to write to, so its valid
        // - We have a reference to self, so no other mutable accesses to the element can occur
        unsafe {
            ptr.as_ref()
                // SAFETY: We can use `unwarp_unchecked` because the pointer isn't null)
                .debug_checked_unwrap()
        }
    }

    /// Get a mutable reference to the element at `index`. This method doesn't do any bounds checking.
    ///
    /// # Safety
    /// - `index` must be safe to write to.
    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &'_ mut T {
        // SAFETY:
        // - `self.data` and the resulting pointer are in the same allocated object
        // - the memory address of the last element doesn't overflow `isize`, so if `index` is in bounds, it won't overflow either
        let ptr = unsafe { self.data.as_ptr().add(index) };
        // SAFETY:
        // - The pointer is properly aligned
        // - It is dereferenceable (all in the same allocation)
        // - `index` < `len` and the element is safe to write to, so its valid
        // - We have a mutable reference to `self`
        unsafe {
            ptr.as_mut()
                // SAFETY: We can use `unwarp_unchecked` because the pointer isn't null)
                .unwrap_unchecked()
        }
    }

    /// Perform a [`swap-remove`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.swap_remove) and return the removed value.
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
    ) -> T {
        #[cfg(debug_assertions)]
        {
            debug_assert!(self.capacity > index_to_keep);
            debug_assert!(self.capacity > index_to_remove);
            debug_assert_ne!(index_to_keep, index_to_remove);
        }
        let base_ptr = self.data.as_ptr();
        let value = ptr::read(base_ptr.add(index_to_remove));
        ptr::copy_nonoverlapping(
            base_ptr.add(index_to_keep),
            base_ptr.add(index_to_remove),
            1,
        );
        value
    }

    /// Perform a [`swap-remove`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.swap_remove) and return the removed value.
    ///
    /// # Safety
    /// - `index_to_keep` must be safe to access (within the bounds of the length of the array).
    /// - `index_to_remove` must be safe to access (within the bounds of the length of the array).
    /// - `index_to_remove` != `index_to_keep`
    /// -  The caller should address the inconsistent state of the array that has occurred after the swap, either:
    ///     1) initialize a different value in `index_to_keep`
    ///     2) update the saved length of the array if `index_to_keep` was the last element.
    #[inline]
    pub unsafe fn swap_remove_unchecked(
        &mut self,
        index_to_remove: usize,
        index_to_keep: usize,
    ) -> T {
        if index_to_remove != index_to_keep {
            return self.swap_remove_unchecked_nonoverlapping(index_to_remove, index_to_keep);
        }
        ptr::read(self.data.as_ptr().add(index_to_remove))
    }

    /// Perform a [`swap-remove`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.swap_remove) and drop the removed value.
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
        let val = &mut self.swap_remove_unchecked(index_to_remove, index_to_keep);
        ptr::drop_in_place(ptr::from_mut(val));
    }

    /// Get a raw pointer to the last element of the array, return `None` if the length is 0
    ///
    /// # Safety
    /// - ensure that `current_len` is indeed the len of the array
    #[inline]
    unsafe fn last_element(&mut self, current_len: usize) -> Option<*mut T> {
        (current_len != 0).then_some(self.data.as_ptr().add(current_len - 1))
    }

    /// Clears the array, removing (and dropping) Note that this method has no effect on the allocated capacity of the vector.
    ///
    /// # Safety
    /// - `current_len` is indeed the length of the array
    /// -   The caller should update their saved length value
    pub unsafe fn clear_elements(&mut self, mut current_len: usize) {
        if needs_drop::<T>() {
            while let Some(to_drop) = self.last_element(current_len) {
                ptr::drop_in_place(to_drop);
                current_len -= 1;
            }
        }
    }

    /// Drop the entire array and all its elements.
    ///
    /// # Safety
    /// - `current_len` is indeed the length of the array
    /// - `current_capacity` is indeed the capacity of the array
    /// - The caller must not use this `ThinArrayPtr` in any way after calling this function
    pub unsafe fn drop(&mut self, current_capacity: usize, current_len: usize) {
        #[cfg(debug_assertions)]
        assert_eq!(self.capacity, current_capacity);
        if current_capacity != 0 {
            self.clear_elements(current_len);
            let layout = Layout::array::<T>(current_capacity).expect("layout should be valid");
            alloc::alloc::dealloc(self.data.as_ptr().cast(), layout);
        }
        self.set_capacity(0);
    }

    /// Get the [`ThinArrayPtr`] as a slice with a given length.
    ///
    /// # Safety
    /// - `slice_len` must match the actual length of the array
    #[inline]
    pub unsafe fn as_slice(&self, slice_len: usize) -> &[T] {
        // SAFETY:
        // - the data is valid - allocated with the same allocator
        // - non-null and well-aligned
        // - we have a shared reference to self - the data will not be mutated during 'a
        unsafe { core::slice::from_raw_parts(self.data.as_ptr(), slice_len) }
    }
}

impl<T> From<Box<[T]>> for ThinArrayPtr<T> {
    fn from(value: Box<[T]>) -> Self {
        let _len = value.len();
        let slice_ptr = Box::<[T]>::into_raw(value);
        // SAFETY: We just got the pointer from a reference
        let first_element_ptr = unsafe { (*slice_ptr).as_mut_ptr() };
        Self {
            // SAFETY: The pointer can't be null, it came from a reference
            data: unsafe { NonNull::new_unchecked(first_element_ptr) },
            #[cfg(debug_assertions)]
            capacity: _len,
        }
    }
}
