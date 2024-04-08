use std::alloc::{alloc, handle_alloc_error, realloc, Layout};
use std::num::NonZeroUsize;
use std::ptr::NonNull;

use bevy_ptr::ThinSlicePtr;

use crate::query::DebugCheckedUnwrap;

// TODO: Better docs
/// Similar to [`Vec<T>`], but with the capacity and length cut out for performance reasons.
/// Similar to [`ThinSlicePtr`], but [`ThinArrayPtr`] supports reallocs (extending / shrinking the array), and swap-removes.
pub struct ThinArrayPtr<T> {
    data: NonNull<T>,
}

impl<T> ThinArrayPtr<T> {
    // TODO: Docs
    pub fn with_capacity(capacity: usize) -> Self {
        let mut arr = ThinArrayPtr {
            data: NonNull::dangling(),
        };
        if capacity > 0 {
            // SAFETY:
            // - The `current_capacity` is 0 because it was just created
            unsafe { arr.reserve_exact(0, 0, capacity) };
        }
        arr
    }

    // TODO: Is this actually needed? I think it can save a lot of branching because using `grow_exact` will check if the capacity is 0 every time.
    // But if the caller has the capacity saved, and they are sure the capacity is 0, they can use `alloc` and save a branch.
    /// Allocate memory for the array, this should only be used if not previous allocation has been made (capacity = 0)
    ///
    /// # Panics
    /// - Panics if the new capacity overflows `usize`
    ///
    /// # Safety
    /// The caller must:
    /// - Ensure that the current capacity is indeed 0
    /// - Update their saved `capacity` value to reflect the fact that it was changed
    pub unsafe fn alloc(&mut self, count: NonZeroUsize) {
        let new_layout =
            Layout::array::<T>(count.get()).expect("layout should be valid (arithmatic overflow)");
        // SAFETY:
        // - layout has non-zero size, `count` > 0, `size` > 0 (ThinArrayPtr doesn't support ZSTs)
        self.data = NonNull::new(unsafe { alloc(new_layout) })
            .unwrap_or_else(|| handle_alloc_error(new_layout))
            .cast();
    }

    // TODO: Is this actually needed? I think it can save a lot of branching because using `grow_exact` will check if the capacity is 0 every time.
    // But if the caller has the capacity saved, and they are sure that capacity > 0, they can use `realloc` and save a branch.
    /// Reallocate memory for the array, this should only be used if a previous allocation for this array has been made (capacity > 0).
    ///
    /// # Panics
    /// - Panics if the new capacity overflows `usize`
    ///
    /// # Safety
    /// The caller must:
    /// - Ensure that the current capacity is indeed greater than 0
    /// - Update their saved `capacity` value to reflect the fact that it was changed
    pub unsafe fn realloc(&mut self, current_capacity: NonZeroUsize, new_capacity: NonZeroUsize) {
        let new_layout = Layout::array::<T>(new_capacity.get())
            .expect("layout should be valid (arithmatic overflow)");
        // SAFETY:
        // - ptr was be allocated via this allocator
        // - the layout of the array is the same as `Layout::array::<T>(current_capacity)`
        // - the size of `T` is non 0 (ZSTs aren't supported in this type), and `new_capacity` > 0
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

    /// Grow the array's capacity by exactly `increment` elements.
    ///
    /// # Panics
    /// - Panics if the new capacity overflows `usize`
    ///
    /// # Safety
    /// The caller must:
    /// - Ensure that `current_capacity` is indeed the current capacity of this array
    /// - Update their saved `capacity` value to reflect the fact that it was changed
    pub unsafe fn grow_exact(&mut self, current_capacity: usize, increment: NonZeroUsize) {
        let new_capacity = NonZeroUsize::new_unchecked(
            current_capacity
                .checked_add(increment.get())
                .expect("capacity overflow"),
        );
        if current_capacity == 0 {
            // SAFETY:
            // - The current capacity is indeed 0, and the `new_capacity` > 0
            unsafe { self.alloc(new_capacity) }
        } else {
            self.realloc(NonZeroUsize::new_unchecked(current_capacity), new_capacity);
        };
    }

    /// Reserves the minimum capacity for at least `additional` more elements to be inserted in the given `BlobVec`.
    /// After calling `reserve_exact`, capacity will be greater than or equal to `self.len() + additional`. Does nothing if
    /// the capacity is already sufficient.
    ///
    /// The method will return the amount by which the capacity grew.
    ///
    /// Note that the allocator may give the collection more space than it requests. Therefore, capacity can not be relied upon
    /// to be precisely minimal.
    ///
    /// # Panics
    ///
    /// Panics if new capacity overflows `usize`.
    ///
    /// # Safety
    /// The caller must:
    /// - ensure that `current_capacity` is indeed the capacity of the array
    /// - ensure that `current_len` is indeed the len of the array
    /// - update their saved `capacity`
    pub unsafe fn reserve_exact(
        &mut self,
        current_capacity: usize,
        current_len: usize,
        additional: usize,
    ) -> usize {
        let available_space = current_capacity - current_len;
        if available_space < additional {
            // SAFETY: `available_space < additional`, so `additional - available_space > 0`
            let increment = unsafe { NonZeroUsize::new_unchecked(additional - available_space) };
            // SAFETY:
            // - `current_capacity` is indeed the current capacity
            // - the caller will update their saved `capacity`
            unsafe { self.grow_exact(current_capacity, increment) };
            return increment.get();
        }
        0
    }

    /// Reserves capacity for at least additional more elements to be inserted in [`Self`].
    /// The collection may reserve more space to speculatively avoid frequent reallocations.
    /// After calling `reserve`, capacity will be greater than or equal to `self.len() + additional`.
    /// Does nothing if capacity is already sufficient.
    ///
    /// The method will return the amount by which the capacity grew.
    ///
    /// # Panics
    /// Panics if the new capacity exceeds isize::MAX bytes.
    ///
    /// # Safety
    /// The caller must:
    /// - ensure that `current_capacity` is indeed the capacity of the array
    /// - ensure that `current_len` is indeed the len of the array
    /// - update their saved `capacity`
    #[inline]
    pub unsafe fn reserve(
        &mut self,
        current_capacity: usize,
        current_len: usize,
        additional: usize,
    ) -> usize {
        /// Similar to `reserve_exact`. This method ensures that the capacity will grow at least `self.capacity()` if there is no
        /// enough space to hold `additional` more elements.
        #[cold]
        unsafe fn do_reserve<T>(
            slf: &mut ThinArrayPtr<T>,
            current_capacity: usize,
            current_len: usize,
            additional: usize,
        ) {
            let increment = current_capacity.max(additional - (current_capacity - current_len));
            let increment = NonZeroUsize::new(increment).unwrap();
            slf.grow_exact(current_capacity, increment);
        }
        if current_capacity - current_len < additional {
            do_reserve::<T>(self, current_capacity, current_len, additional);
            return additional;
        }
        0
    }

    /// Initializes the value at `index` to `value`. This function does not do any bounds checking.
    ///
    /// # Safety
    /// index must be in bounds (`index` < `len`)
    #[inline]
    pub unsafe fn initialize_unchecked(&mut self, index: usize, value: T) {
        // SAFETY: `index` is in bounds
        let ptr = unsafe { self.get_unchecked_raw(index) };
        // SAFETY: `index` is in bounds, therefore the pointer to that location in the array is valid, and aligned.
        unsafe { core::ptr::write(ptr, value) };
    }

    /// Get a raw pointer to the element at `index`. This method doesn't do any bounds checking.
    ///
    /// # Safety
    /// - `index` must be in bounds (`index` < `len`)
    pub unsafe fn get_unchecked_raw(&mut self, index: usize) -> *mut T {
        // SAFETY:
        // - `self.data` and the resulting pointer are in the same allocated object
        // - the memory adress of the last element doesn't overflow `isize`, so if `index` is in bounds, it won't overflow either
        unsafe { self.data.as_ptr().add(index) }
    }

    /// Get a reference to the element at `index`. This method doesn't do any bounds checking.
    ///
    /// # Safety
    /// - `index` must be in bounds (`index` < `len`)
    /// - The element at index `index` must be safe to read
    pub unsafe fn get_unchecked(&self, index: usize) -> &'_ T {
        // SAFETY:
        // - `self.data` and the resulting pointer are in the same allocated object
        // - the memory adress of the last element doesn't overflow `isize`, so if `index` is in bounds, it won't overflow either
        let ptr = unsafe { self.data.as_ptr().add(index) };

        // SAFETY:
        // - The pointer is properly aligned
        // - It is derefrancable (all in the same allocation)
        // - `index` < `len` and the element is safe to write to, so its valid
        // - We have a reference to self, so no other mutable accesses to the element can occur
        unsafe {
            ptr.as_ref()
                // SAFETY: We can use `unwarp_unchecked` because the pointer isn't null)
                .unwrap_unchecked()
        }
    }

    /// Get a mutable reference to the element at `index`. This method doesn't do any bounds checking.
    ///
    /// # Safety
    /// - `index` must be in bounds (`index` < `len`)
    /// - The element at index `index` must be safe to write to
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &'_ mut T {
        // SAFETY:
        // - `self.data` and the resulting pointer are in the same allocated object
        // - the memory adress of the last element doesn't overflow `isize`, so if `index` is in bounds, it won't overflow either
        let ptr = unsafe { self.data.as_ptr().add(index) };

        // SAFETY:
        // - The pointer is properly aligned
        // - It is derefrancable (all in the same allocation)
        // - `index` < `len` and the element is safe to write to, so its valid
        // - We have a mutable reference to `self`
        unsafe {
            ptr.as_mut()
                // SAFETY: We can use `unwarp_unchecked` because the pointer isn't null)
                .unwrap_unchecked()
        }
    }

    // TODO: Docs
    /// # Safety
    /// The caller must:
    /// - ensure that `index < len`
    /// - ensure that `last_element_index` = `len - 1`
    /// - update their saved length value to reflect that the last element has been removed (decrement it)
    pub unsafe fn swap_remove_and_forget_unchecked(
        &mut self,
        index: usize,
        last_element_index: usize,
    ) -> *mut T {
        if index != last_element_index {
            std::ptr::swap_nonoverlapping(
                self.get_unchecked_raw(index),
                self.get_unchecked_raw(last_element_index),
                1,
            );
        }
        self.get_unchecked_raw(last_element_index)
    }

    // TODO: Docs
    /// # Safety
    /// The caller must:
    /// - ensure that `index < len`
    /// - ensure that `last_element_index` = `len - 1`
    /// - update their saved length value to reflect that the last element has been removed (decrement it)
    pub unsafe fn swap_remove_and_drop_unchecked(
        &mut self,
        index: usize,
        last_element_index: usize,
    ) {
        std::ptr::drop_in_place(self.swap_remove_and_forget_unchecked(index, last_element_index))
    }

    /// Push a new `T` onto the top of the array. This will increase the capacity if needed (realloc).
    ///
    /// The method will return the amount by which the capacity grew.
    ///
    /// # Safety
    /// - ensure that `current_capacity` is indeed the capacity of the array
    /// - ensure that `current_len` is indeed the len of the array
    /// - update their saved `capacity`
    /// - update their saved `len` (increment it)
    pub unsafe fn push(&mut self, current_capacity: usize, current_len: usize, value: T) -> usize {
        let additional = self.reserve(current_capacity, current_len, 1);
        // SAFETY: `self.reserve(.., 1)` effectivly incremented len, so `current_len` is smaller the the "real" len
        unsafe { self.initialize_unchecked(current_len, value) };
        additional
    }

    /// Pop the last element of the array.
    ///
    /// # Safety
    /// - ensure that `current_len` is indeed the len of the array
    /// - update their saved `len` (decrement it)
    pub unsafe fn pop(&mut self, current_len: usize) -> Option<T> {
        if current_len == 0 {
            None
        } else {
            Some(std::ptr::read(self.data.as_ptr().add(current_len)))
        }
    }

    /// Clears the array, removing (and dropping) Note that this method has no effect on the allocated capacity of the vector.
    ///
    /// # Safety
    /// The caller must:
    /// - ensure that `current_len` is indeed the length of the array
    /// - update their saved length value
    pub unsafe fn clear_elements(&mut self, mut current_len: usize) {
        while self.pop(current_len).is_some() {
            current_len -= 1;
        }
    }

    /// Drop the entire array and all its elements.
    /// # Safety
    /// The caller must:
    /// - ensure that `current_len` is indeed the length of the array
    /// - ensure that `current_capacity` is indeed the capacity of the array
    pub unsafe fn drop(mut self, current_capacity: usize, current_len: usize) {
        if current_capacity != 0 {
            self.clear_elements(current_len);
            let layout = Layout::array::<T>(current_capacity).expect("layout should be valid");
            std::alloc::dealloc(self.data.as_ptr().cast(), layout);
        }
    }

    // TODO: Docs
    /// # Safety
    /// - `slice_len` must match the actual length of the array
    /// but if `slice_len` will be smaller, the slice will just be smaller than need be - no UB
    pub unsafe fn to_slice<'a>(&'a self, slice_len: usize) -> &'a [T] {
        // SAFETY:
        // - the data is valid - allocated with the same allocater
        // - non-null and well-aligned
        // - we have a shared refernce to self - the data will not be mutated during 'a
        unsafe { std::slice::from_raw_parts(self.data.as_ptr(), slice_len) }
    }
}

impl<T> From<Box<[T]>> for ThinArrayPtr<T> {
    fn from(value: Box<[T]>) -> Self {
        if Layout::new::<T>().size() == 0 {
            panic!("Can't use ThinArrayPtr for ZSTs");
        }
        let slice_ptr = Box::<[T]>::into_raw(value);
        // SAFETY: We just got the pointer from a reference
        let first_element_ptr = unsafe { (*slice_ptr).as_mut_ptr() };
        Self {
            // SAFETY: The pointer can't be null, it came from a reference
            data: unsafe { NonNull::new_unchecked(first_element_ptr) },
        }
    }
}
