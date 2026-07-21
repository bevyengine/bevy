//! Provides [`AlignedVec`] based on [bevy_platform::collections::AlignedVec](https://github.com/rkyv/rkyv/blob/main/rkyv/src/util/alloc/aligned_vec.rs)'s implementation but the alignment can be set at runtime.
//!
//! The original source code, adapted here, is copyright 2021 David Koloski, used here under the MIT License.
#![expect(unsafe_code, reason = "This struct needs to interact with raw memory")]

use alloc::alloc::{alloc, dealloc, handle_alloc_error, realloc};
#[cfg(feature = "bytemuck")]
use alloc::vec::Vec;
use core::{
    alloc::Layout,
    borrow::{Borrow, BorrowMut},
    fmt,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut, Index, IndexMut},
    ptr::NonNull,
    slice,
};

/// A vector of bytes that dynamically aligns its memory to the specified alignment.
///
/// ```
/// # use bevy_platform::collections::AlignedVec;
/// let bytes = AlignedVec::with_capacity(4096, 1);
/// assert_eq!(bytes.as_ptr().align_offset(4096), 0);
/// ```
pub struct AlignedVec {
    ptr: NonNull<u8>,
    align: usize,
    cap: usize,
    len: usize,
}

impl Drop for AlignedVec {
    fn drop(&mut self) {
        if self.cap != 0 {
            // SAFETY: both `ptr` and `layout` are valid
            unsafe {
                dealloc(self.ptr.as_ptr(), self.layout());
            }
        }
    }
}

impl AlignedVec {
    /// The alignment of the vector
    #[inline]
    pub fn alignment(&self) -> usize {
        self.align
    }

    /// Maximum valid size of [`Layout`].
    ///
    /// Dictated by the requirements of [`Layout::from_size_align`]:
    /// `size`, when rounded up to the nearest multiple of `align`, must not overflow `isize`.
    #[inline]
    const fn max_size_for_alignment(align: usize) -> usize {
        isize::MAX as usize + 1 - align
    }

    /// Maximum valid capacity of the vector with `self.align`.
    #[inline]
    fn max_capacity(&self) -> usize {
        Self::max_size_for_alignment(self.alignment())
    }

    /// Constructs a new, empty `AlignedVec`.
    ///
    /// The vector will not allocate until elements are pushed into it.
    ///
    /// # Examples
    /// ```
    /// # use bevy_platform::collections::AlignedVec;
    /// let mut vec = AlignedVec::new(16);
    /// ```
    #[inline]
    pub fn new(align: usize) -> Self {
        Self::with_capacity(align, 0)
    }

    /// Constructs a new, empty `AlignedVec` with the specified alignment and capacity.
    ///
    /// The vector will be able to hold exactly `capacity` bytes without
    /// reallocating. If `capacity` is 0, the vector will not allocate.
    ///
    /// # Examples
    /// ```
    /// # use bevy_platform::collections::AlignedVec;
    /// let mut vec = AlignedVec::with_capacity(16, 10);
    ///
    /// // The vector contains no items, even though it has capacity for more
    /// assert_eq!(vec.len(), 0);
    /// assert_eq!(vec.capacity(), 10);
    ///
    /// // These are all done without reallocating...
    /// for i in 0..10 {
    ///     vec.push(i);
    /// }
    /// assert_eq!(vec.len(), 10);
    /// assert_eq!(vec.capacity(), 10);
    ///
    /// // ...but this may make the vector reallocate
    /// vec.push(11);
    /// assert_eq!(vec.len(), 11);
    /// assert!(vec.capacity() >= 11);
    /// ```
    #[inline]
    pub fn with_capacity(align: usize, capacity: usize) -> Self {
        assert!(align > 0, "align must be 1 or more");
        assert!(align.is_power_of_two(), "align must be a power of 2");
        // As `align` has to be a power of 2, this caps `align` at a max
        // of `(isize::MAX + 1) / 2` (1 GiB on 32-bit systems).
        assert!(
            align < isize::MAX as usize,
            "align must be less than isize::MAX"
        );

        if capacity == 0 {
            Self {
                ptr: NonNull::without_provenance(
                    // SAFETY: `align` is checked to be non-zero.
                    unsafe { core::num::NonZero::<usize>::new_unchecked(align) },
                ),
                align,
                cap: 0,
                len: 0,
            }
        } else {
            assert!(
                capacity <= Self::max_size_for_alignment(align),
                "`capacity` when rounded up to the nearest multiple of align overflows `isize`"
            );

            let ptr = {
                // SAFETY: align > 0, align is power of two and capacity <= max size for alignment.
                let layout = unsafe { Layout::from_size_align_unchecked(capacity, align) };
                // SAFETY: capacity is not zero.
                let ptr = unsafe { alloc(layout) };
                if ptr.is_null() {
                    handle_alloc_error(layout);
                }
                // SAFETY: ptr is not null
                unsafe { NonNull::new_unchecked(ptr) }
            };

            Self {
                ptr,
                align,
                cap: capacity,
                len: 0,
            }
        }
    }

    #[inline]
    fn layout(&self) -> Layout {
        // SAFETY: `cap` and `align` are valid for layout
        unsafe { Layout::from_size_align_unchecked(self.cap, self.alignment()) }
    }

    /// Clears the vector, removing all values.
    ///
    /// Note that this method has no effect on the allocated capacity of the
    /// vector.
    ///
    /// # Examples
    /// ```
    /// # use bevy_platform::collections::AlignedVec;
    /// let mut v = AlignedVec::new(16);
    /// v.extend_from_slice(&[1, 2, 3, 4]);
    ///
    /// v.clear();
    ///
    /// assert!(v.is_empty());
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Change capacity of vector.
    ///
    /// Will set capacity to exactly `new_cap`.
    /// Can be used to either grow or shrink capacity.
    /// Backing memory will be reallocated.
    ///
    /// Usually the safe methods `reserve` or `reserve_exact` are a better
    /// choice. This method only exists as a micro-optimization for very
    /// performance-sensitive code where where the calculation of capacity
    /// required has already been performed, and you want to avoid doing it
    /// again, or if you want to implement a different growth strategy.
    ///
    /// # Safety
    ///
    /// - `new_cap` when rounded up to the nearest multiple of align must not overflow `isize`
    /// - `new_cap` must be greater than or equal to [`len()`](AlignedVec::len)
    pub unsafe fn change_capacity(&mut self, new_cap: usize) {
        debug_assert!(new_cap <= self.max_capacity());
        debug_assert!(new_cap >= self.len);

        if new_cap > 0 {
            let new_ptr = if self.cap > 0 {
                // SAFETY:
                // - `self.ptr` is currently allocated because `self.cap` is
                //   greater than zero.
                // - `self.layout()` always matches the layout used to allocate
                //   the current block of memory.
                // - We checked that `new_cap` is greater than zero.
                let new_ptr = unsafe { realloc(self.ptr.as_ptr(), self.layout(), new_cap) };
                if new_ptr.is_null() {
                    // SAFETY:
                    // - `self.align` is always guaranteed to be a nonzero power
                    //   of two.
                    // - We checked that `new_cap` doesn't overflow `isize` when
                    //   rounded up to the nearest power of two.
                    let layout =
                        unsafe { Layout::from_size_align_unchecked(new_cap, self.alignment()) };
                    handle_alloc_error(layout);
                }
                new_ptr
            } else {
                // SAFETY:
                // - `self.align` is always guaranteed to be a nonzero power of
                //   two.
                // - We checked that `new_cap` doesn't overflow `isize` when
                //   rounded up to the nearest power of two.
                let layout =
                    unsafe { Layout::from_size_align_unchecked(new_cap, self.alignment()) };
                // SAFETY: We checked that `new_cap` has non-zero size.
                let new_ptr = unsafe { alloc(layout) };
                if new_ptr.is_null() {
                    handle_alloc_error(layout);
                }
                new_ptr
            };
            // SAFETY: We checked that `new_ptr` is non-null in each of the
            // branches.
            self.ptr = unsafe { NonNull::new_unchecked(new_ptr) };
            self.cap = new_cap;
        } else if self.cap > 0 {
            // SAFETY: Because the capacity is nonzero, `self.ptr` points to a
            // currently-allocated memory block. All memory blocks are allocated
            // with a layout of `self.layout()`.
            unsafe {
                dealloc(self.ptr.as_ptr(), self.layout());
            }
            self.ptr = NonNull::without_provenance(
                // SAFETY: `align` is checked to be non-zero.
                unsafe { core::num::NonZero::<usize>::new_unchecked(self.alignment()) },
            );
            self.cap = 0;
        }
    }

    /// Shrinks the capacity of the vector as much as possible.
    ///
    /// It will drop down as close as possible to the length but the allocator
    /// may still inform the vector that there is space for a few more
    /// elements.
    ///
    /// # Examples
    /// ```
    /// # use bevy_platform::collections::AlignedVec;
    /// let mut vec = AlignedVec::with_capacity(16, 10);
    /// vec.extend_from_slice(&[1, 2, 3]);
    /// assert_eq!(vec.capacity(), 10);
    /// vec.shrink_to_fit();
    /// assert!(vec.capacity() >= 3);
    ///
    /// vec.clear();
    /// vec.shrink_to_fit();
    /// assert!(vec.capacity() == 0);
    /// ```
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        if self.cap != self.len {
            // SAFETY: New capacity is equal to length, and cannot exceed max as it's shrinking
            unsafe { self.change_capacity(self.len) };
        }
    }

    /// Returns an unsafe mutable pointer to the vector's buffer.
    ///
    /// The caller must ensure that the vector outlives the pointer this
    /// function returns, or else it will end up pointing to garbage.
    /// Modifying the vector may cause its buffer to be reallocated, which
    /// would also make any pointers to it invalid.
    ///
    /// # Examples
    /// ```
    /// # use bevy_platform::collections::AlignedVec;
    /// // Allocate 1-aligned vector big enough for 4 bytes.
    /// let size = 4;
    /// let mut x = AlignedVec::with_capacity(1, size);
    /// let x_ptr = x.as_mut_ptr();
    ///
    /// // Initialize elements via raw pointer writes, then set length.
    /// unsafe {
    ///     for i in 0..size {
    ///         *x_ptr.add(i) = i as u8;
    ///     }
    ///     x.set_len(size);
    /// }
    /// assert_eq!(&*x, &[0, 1, 2, 3]);
    /// ```
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr.as_ptr()
    }

    /// Extracts a mutable slice of the entire vector.
    ///
    /// Equivalent to `&mut s[..]`.
    ///
    /// # Examples
    /// ```
    /// # use bevy_platform::collections::AlignedVec;
    /// let mut vec = AlignedVec::new(16);
    /// vec.extend_from_slice(&[1, 2, 3, 4, 5]);
    /// assert_eq!(vec.as_mut_slice().len(), 5);
    /// for i in 0..5 {
    ///     assert_eq!(vec.as_mut_slice()[i], i as u8 + 1);
    ///     vec.as_mut_slice()[i] = i as u8;
    ///     assert_eq!(vec.as_mut_slice()[i], i as u8);
    /// }
    /// ```
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY: `ptr` and `len` are valid to construct slice
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }

    /// Returns a raw pointer to the vector's buffer.
    ///
    /// The caller must ensure that the vector outlives the pointer this
    /// function returns, or else it will end up pointing to garbage.
    /// Modifying the vector may cause its buffer to be reallocated, which
    /// would also make any pointers to it invalid.
    ///
    /// The caller must also ensure that the memory the pointer
    /// (non-transitively) points to is never written to (except inside an
    /// `UnsafeCell`) using this pointer or any pointer derived from it. If
    /// you need to mutate the contents of the slice, use
    /// [`as_mut_ptr`](AlignedVec::as_mut_ptr).
    ///
    /// # Examples
    /// ```
    /// # use bevy_platform::collections::AlignedVec;
    /// let mut x = AlignedVec::new(16);
    /// x.extend_from_slice(&[1, 2, 4]);
    /// let x_ptr = x.as_ptr();
    ///
    /// unsafe {
    ///     for i in 0..x.len() {
    ///         assert_eq!(*x_ptr.add(i), 1 << i);
    ///     }
    /// }
    /// ```
    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }

    /// Extracts a slice containing the entire vector.
    ///
    /// Equivalent to `&s[..]`.
    ///
    /// # Examples
    /// ```
    /// # use bevy_platform::collections::AlignedVec;
    /// let mut vec = AlignedVec::new(16);
    /// vec.extend_from_slice(&[1, 2, 3, 4, 5]);
    /// assert_eq!(vec.as_slice().len(), 5);
    /// for i in 0..5 {
    ///     assert_eq!(vec.as_slice()[i], i as u8 + 1);
    /// }
    /// ```
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: `ptr` and `len` are valid to construct slice
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    /// Returns the number of elements the vector can hold without reallocating.
    ///
    /// # Examples
    /// ```
    /// # use bevy_platform::collections::AlignedVec;
    /// let vec = AlignedVec::with_capacity(16, 10);
    /// assert_eq!(vec.capacity(), 10);
    /// ```
    #[inline]
    pub fn capacity(&self) -> usize {
        self.cap
    }

    /// Reserves capacity for at least `additional` more bytes to be inserted
    /// into the given `AlignedVec`. The collection may reserve more space
    /// to avoid frequent reallocations. After calling `reserve`, capacity
    /// will be greater than or equal to `self.len() + additional`. Does
    /// nothing if capacity is already sufficient.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity when rounded up to the nearest multiple of align overflow `isize`.
    ///
    /// # Examples
    /// ```
    /// # use bevy_platform::collections::AlignedVec;
    ///
    /// let mut vec = AlignedVec::new(16);
    /// vec.push(1);
    /// vec.reserve(10);
    /// assert!(vec.capacity() >= 11);
    /// ```
    pub fn reserve(&mut self, additional: usize) {
        // Cannot wrap because capacity always exceeds len,
        // but avoids having to handle potential overflow here
        let remaining = self.cap.wrapping_sub(self.len);
        if additional > remaining {
            self.do_reserve(additional);
        }
    }

    /// Extend capacity after `reserve` has found it's necessary.
    ///
    /// Actually performing the extension is in this separate function marked
    /// `#[cold]` to hint to compiler that this branch is not often taken.
    /// This keeps the path for common case where capacity is already sufficient
    /// as fast as possible, and makes `reserve` more likely to be inlined.
    /// This is the same trick that Rust's `Vec::reserve` uses.
    #[cold]
    fn do_reserve(&mut self, additional: usize) {
        let new_cap = self
            .len
            .checked_add(additional)
            .expect("cannot reserve a larger AlignedVec");
        // SAFETY: `do_reserve` is only called when capacity grows
        unsafe { self.grow_capacity_to(new_cap) };
    }

    /// Grows total capacity of vector to `new_cap` or more.
    ///
    /// Capacity after this call will be `new_cap` rounded up to next power of
    /// 2, unless that would exceed maximum capacity, in which case capacity
    /// is capped at the maximum.
    ///
    /// This is same growth strategy used by `reserve`, `push` and
    /// `extend_from_slice`.
    ///
    /// Usually the safe methods `reserve` or `reserve_exact` are a better
    /// choice. This method only exists as a micro-optimization for very
    /// performance-sensitive code where where the calculation of capacity
    /// required has already been performed, and you want to avoid doing it
    /// again.
    ///
    /// Maximum capacity is `isize::MAX + 1 - self.align` bytes.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity when rounded up to the nearest multiple of align overflow `isize`.
    ///
    /// # Safety
    ///
    /// - `new_cap` must be greater than current
    ///   [`capacity()`](AlignedVec::capacity)
    ///
    /// # Examples
    /// ```
    /// # use bevy_platform::collections::AlignedVec;
    ///
    /// let mut vec = AlignedVec::new(16);
    /// vec.push(1);
    /// unsafe { vec.grow_capacity_to(50) };
    /// assert_eq!(vec.len(), 1);
    /// assert_eq!(vec.capacity(), 64);
    /// ```
    pub unsafe fn grow_capacity_to(&mut self, new_cap: usize) {
        debug_assert!(new_cap > self.cap);

        let new_cap = if new_cap > (isize::MAX as usize + 1) >> 1 {
            // Rounding up to next power of 2 would result in `isize::MAX + 1`
            // or higher, which exceeds max capacity. So cap at max
            // instead.
            assert!(
                new_cap <= self.max_capacity(),
                "cannot reserve a larger AlignedVec"
            );
            self.max_capacity()
        } else {
            // Cannot overflow due to check above
            new_cap.next_power_of_two()
        };
        let min_non_zero_cap = 8;
        let new_cap = core::cmp::max(new_cap, min_non_zero_cap);
        // SAFETY: We just checked that `new_cap` is greater than or equal to
        // `len` and less than or equal to `max_capacity`.
        unsafe {
            self.change_capacity(new_cap);
        }
    }

    /// Resizes the Vec in-place so that len is equal to `new_len`.
    ///
    /// If `new_len` is greater than len, the Vec is extended by the difference,
    /// with each additional slot filled with value. If `new_len` is less than
    /// len, the Vec is simply truncated.
    ///
    /// # Panics
    ///
    /// Panics if the new length when rounded up to the nearest multiple of align overflow `isize`.
    ///
    /// # Examples
    /// ```
    /// # use bevy_platform::collections::AlignedVec;
    ///
    /// let mut vec = AlignedVec::new(16);
    /// vec.push(3);
    /// vec.resize(3, 2);
    /// assert_eq!(vec.as_slice(), &[3, 2, 2]);
    ///
    /// let mut vec = AlignedVec::new(16);
    /// vec.extend_from_slice(&[1, 2, 3, 4]);
    /// vec.resize(2, 0);
    /// assert_eq!(vec.as_slice(), &[1, 2]);
    /// ```
    pub fn resize(&mut self, new_len: usize, value: u8) {
        if new_len > self.len {
            let additional = new_len - self.len;
            self.reserve(additional);
            // SAFETY: ptr is valid to write after `reserve`
            unsafe {
                core::ptr::write_bytes(self.ptr.as_ptr().add(self.len), value, additional);
            }
        }
        // SAFETY: required elements are initialized for `new_len`
        unsafe {
            self.set_len(new_len);
        }
    }

    /// Returns `true` if the vector contains no elements.
    ///
    /// # Examples
    /// ```
    /// # use bevy_platform::collections::AlignedVec;
    ///
    /// let mut v = Vec::new();
    /// assert!(v.is_empty());
    ///
    /// v.push(1);
    /// assert!(!v.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the number of elements in the vector, also referred to as its
    /// 'length'.
    ///
    /// # Examples
    /// ```
    /// # use bevy_platform::collections::AlignedVec;
    ///
    /// let mut a = AlignedVec::new(16);
    /// a.extend_from_slice(&[1, 2, 3]);
    /// assert_eq!(a.len(), 3);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Consumes and leaks the `AlignedVec`, returning a mutable reference to
    /// the contents, `&'static mut [u8]`.
    ///
    /// This method does not reallocate or shrink the `AlignedVec`, so the
    /// leaked allocation may include unused capacity that is not part of the
    /// returned slice.
    ///
    /// This function is mainly useful for data that lives for the remainder of
    /// the program's life. Dropping the returned reference will cause a memory
    /// leak.
    ///
    /// # Examples
    ///
    /// Simple usage:
    ///
    /// ```
    /// # use std::alloc::{Layout, dealloc};
    /// # use bevy_platform::collections::AlignedVec;
    ///
    /// let mut x = AlignedVec::new(16);
    /// x.extend_from_slice(&[1, 2, 3]);
    /// # let layout = Layout::from_size_align(x.capacity(), 16).unwrap();
    /// let static_ref: &'static mut [u8] = x.leak();
    /// static_ref[0] += 1;
    /// assert_eq!(static_ref, &[2, 2, 3]);
    /// # // Need to manually dealloc to avoid triggering Miri's leak check
    /// # unsafe {
    /// #     dealloc(static_ref.as_mut_ptr(), layout);
    /// # }
    /// ```
    pub fn leak(self) -> &'static mut [u8] {
        let mut me = ManuallyDrop::new(self);
        // SAFETY: `ptr` and `len` are valid to construct slice
        unsafe { slice::from_raw_parts_mut(me.as_mut_ptr(), me.len) }
    }

    /// Copies and appends all bytes in a slice to the `AlignedVec`.
    ///
    /// The elements of the slice are appended in-order.
    ///
    /// # Examples
    /// ```
    /// # use bevy_platform::collections::AlignedVec;
    ///
    /// let mut vec = AlignedVec::new(16);
    /// vec.push(1);
    /// vec.extend_from_slice(&[2, 3, 4]);
    /// assert_eq!(vec.as_slice(), &[1, 2, 3, 4]);
    /// ```
    pub fn extend_from_slice(&mut self, other: &[u8]) {
        self.reserve(other.len());
        // SAFETY: memory is reserved for copy
        unsafe {
            core::ptr::copy_nonoverlapping(
                other.as_ptr(),
                self.as_mut_ptr().add(self.len()),
                other.len(),
            );
        }
        self.len += other.len();
    }

    /// Removes the last element from a vector and returns it, or `None` if it
    /// is empty.
    ///
    /// # Examples
    /// ```
    /// # use bevy_platform::collections::AlignedVec;
    ///
    /// let mut vec = AlignedVec::new(16);
    /// vec.extend_from_slice(&[1, 2, 3]);
    /// assert_eq!(vec.pop(), Some(3));
    /// assert_eq!(vec.as_slice(), &[1, 2]);
    /// ```
    #[inline]
    pub fn pop(&mut self) -> Option<u8> {
        if self.len == 0 {
            None
        } else {
            let result = self[self.len - 1];
            self.len -= 1;
            Some(result)
        }
    }

    /// Appends an element to the back of a collection.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity when rounded up to the nearest multiple of align overflow `isize`.
    ///
    /// # Examples
    /// ```
    /// # use bevy_platform::collections::AlignedVec;
    ///
    /// let mut vec = AlignedVec::new(16);
    /// vec.extend_from_slice(&[1, 2]);
    /// vec.push(3);
    /// assert_eq!(vec.as_slice(), &[1, 2, 3]);
    /// ```
    #[inline]
    pub fn push(&mut self, value: u8) {
        if self.len == self.cap {
            self.reserve_for_push();
        }

        // SAFETY: memory is reserved for writing.
        unsafe {
            self.as_mut_ptr().add(self.len).write(value);
            self.len += 1;
        }
    }

    /// Extend capacity by at least 1 byte after `push` has found it's
    /// necessary.
    ///
    /// Actually performing the extension is in this separate function marked
    /// `#[cold]` to hint to compiler that this branch is not often taken.
    /// This keeps the path for common case where capacity is already sufficient
    /// as fast as possible, and makes `push` more likely to be inlined.
    /// This is the same trick that Rust's `Vec::push` uses.
    #[cold]
    fn reserve_for_push(&mut self) {
        // `len` is always less than `isize::MAX`, so no possibility of overflow
        // here
        let new_cap = self.len + 1;
        // SAFETY: `reserve_for_push` is only called when capacity grows
        unsafe { self.grow_capacity_to(new_cap) };
    }

    /// Reserves the minimum capacity for exactly `additional` more elements to
    /// be inserted in the given `AlignedVec`. After calling
    /// `reserve_exact`, capacity will be greater than or equal
    /// to `self.len() + additional`. Does nothing if the capacity is already
    /// sufficient.
    ///
    /// Note that the allocator may give the collection more space than it
    /// requests. Therefore, capacity can not be relied upon to be precisely
    /// minimal. Prefer reserve if future insertions are expected.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity when rounded up to the nearest multiple of align overflow `isize`.
    ///
    /// # Examples
    /// ```
    /// # use bevy_platform::collections::AlignedVec;
    ///
    /// let mut vec = AlignedVec::new(16);
    /// vec.push(1);
    /// vec.reserve_exact(10);
    /// assert!(vec.capacity() >= 11);
    /// ```
    pub fn reserve_exact(&mut self, additional: usize) {
        // This function does not use the hot/cold paths trick that `reserve`
        // and `push` do, on assumption that user probably knows this will
        // require an increase in capacity. Otherwise, they'd likely use
        // `reserve`.
        let new_cap = self
            .len
            .checked_add(additional)
            .expect("cannot reserve a larger AlignedVec");
        if new_cap > self.cap {
            assert!(
                new_cap <= self.max_capacity(),
                "cannot reserve a larger AlignedVec"
            );
            // SAFETY: `new_cap` is `self.len + additional` thus it is >= `self.len`
            unsafe { self.change_capacity(new_cap) };
        }
    }

    /// Forces the length of the vector to `new_len`.
    ///
    /// This is a low-level operation that maintains none of the normal
    /// invariants of the type.
    ///
    /// # Safety
    ///
    /// - `new_len` must be less than or equal to
    ///   [`capacity()`](AlignedVec::capacity)
    /// - The elements at `old_len..new_len` must be initialized
    ///
    /// # Examples
    /// ```
    /// # use bevy_platform::collections::AlignedVec;
    /// let mut vec = AlignedVec::with_capacity(16, 3);
    /// vec.extend_from_slice(&[1, 2, 3]);
    ///
    /// // SAFETY:
    /// // 1. `old_len..0` is empty to no elements need to be initialized.
    /// // 2. `0 <= capacity` always holds whatever capacity is.
    /// unsafe {
    ///     vec.set_len(0);
    /// }
    /// ```
    pub unsafe fn set_len(&mut self, new_len: usize) {
        debug_assert!(new_len <= self.capacity());

        self.len = new_len;
    }

    /// Converts the vector into `Vec<T>`.
    ///
    /// Panics if any of the following assertions fail:
    /// ```rust,ignore
    /// assert!(align_of::<T>() == self.alignment());
    /// assert!(self.len().is_multiple_of(size_of::<T>()));
    /// assert!(self.capacity().is_multiple_of(size_of::<T>()));
    /// ```
    ///
    /// # Examples
    /// ```
    /// # use bevy_platform::collections::AlignedVec;
    /// let mut v = AlignedVec::new(2);
    /// v.extend_from_slice(&[1, 2, 3, 4]);
    ///
    /// let vec: Vec<u16> = v.into_vec();
    /// assert_eq!(vec.len(), 2);
    /// assert_eq!(vec.as_slice(), &[513, 1027]);
    /// ```
    #[cfg(feature = "bytemuck")]
    pub fn into_vec<T: bytemuck::AnyBitPattern>(self) -> Vec<T> {
        const {
            assert!(size_of::<T>() != 0);
        }
        assert!(align_of::<T>() == self.alignment());
        assert!(self.len().is_multiple_of(size_of::<T>()));
        assert!(self.capacity().is_multiple_of(size_of::<T>()));
        let (ptr, _align, len, cap) = self.into_raw_parts();
        // SAFETY: the raw parts from `self` are valid to be used as `Vec`
        unsafe {
            Vec::from_raw_parts(
                ptr.cast::<T>().as_ptr(),
                len / size_of::<T>(),
                cap / size_of::<T>(),
            )
        }
    }

    /// Casts the vector to a slice of `T`.
    ///
    /// Panics:
    /// * If `T` has a greater alignment requirement than the `AlignedVec`.
    /// * If the size of `AlignedVec` is not a multiple of `size_of::<T>()`
    #[cfg(feature = "bytemuck")]
    pub fn cast_slice<T: bytemuck::AnyBitPattern>(&self) -> &[T] {
        assert!(align_of::<T>() <= self.alignment());
        bytemuck::cast_slice(self.as_slice())
    }

    /// Casts the vector to a mutable slice of `T`.
    ///
    /// Panics:
    /// * If `T` has a greater alignment requirement than the `AlignedVec`.
    /// * If the size of `AlignedVec` is not a multiple of `size_of::<T>()`
    #[cfg(feature = "bytemuck")]
    pub fn cast_slice_mut<T: bytemuck::AnyBitPattern + bytemuck::NoUninit>(&mut self) -> &mut [T] {
        assert!(align_of::<T>() <= self.alignment());
        bytemuck::cast_slice_mut(self.as_mut_slice())
    }

    /// Decompose an [`AlignedVec`] into its raw components: `(NonNull pointer, align,
    /// length, capacity)`.
    ///
    /// The returned parts can be used to re-assemble the [`AlignedVec`] using
    /// the [`from_raw_parts`](AlignedVec::from_raw_parts) function.
    ///
    /// After calling this function, the caller is responsible for the memory
    /// previously managed by the [`AlignedVec`]. The only way to do this is
    /// to convert the [`NonNull`] pointer, the length and the capacity back
    /// into an [`AlignedVec`] using the [`from_raw_parts`](AlignedVec::from_raw_parts)
    /// function, allowing the destructor to perform the cleanup.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_platform::collections::AlignedVec;
    ///
    /// let mut v: AlignedVec = AlignedVec::new(16);
    /// for i in 1..=5 {
    ///     v.push(i);
    /// }
    ///
    /// let (ptr, align, len, cap) = v.into_raw_parts();
    ///
    /// let rebuilt: AlignedVec =
    ///     unsafe { AlignedVec::from_raw_parts(ptr, align, len, cap) };
    /// assert_eq!(rebuilt.as_slice(), &[1, 2, 3, 4, 5]);
    /// ```
    #[must_use = "losing the pointer will leak memory"]
    pub fn into_raw_parts(self) -> (NonNull<u8>, usize, usize, usize) {
        let this = ManuallyDrop::new(self);
        (this.ptr, this.align, this.len, this.cap)
    }

    /// Create an [`AlignedVec`] directly from a [`NonNull`] pointer, a length
    /// and a capacity.
    ///
    /// # Safety
    ///
    /// This is highly unsafe, due to the number of invariants that aren't
    /// checked:
    ///
    /// * If the capacity is nonzero, `ptr` must have
    ///   been allocated using the global allocator, such as via the [`alloc::alloc`]
    ///   function. If the capacity is zero, `ptr` need only be aligned.
    /// * `align` needs to be equal to the alignment as what `ptr` was allocated with,
    ///   if the pointer is required to be allocated.
    /// * The `capacity` (i.e. the allocated size in bytes), if
    ///   nonzero, needs to be the same size as the pointer was allocated with.
    ///   (Because similar to alignment, [`dealloc`] must be called with the same
    ///   layout `size`.)
    /// * `length` needs to be less than or equal to `capacity`.
    /// * `capacity` needs to be the capacity that the pointer was allocated with,
    ///   if the pointer is required to be allocated.
    /// * The allocated size in bytes must be no larger than `isize::MAX`.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_platform::collections::AlignedVec;
    ///
    /// let mut v: AlignedVec = AlignedVec::new(16);
    /// for i in 1..=5 {
    ///     v.push(i);
    /// }
    ///
    /// let (ptr, align, len, cap) = v.into_raw_parts();
    ///
    /// let rebuilt: AlignedVec =
    ///     unsafe { AlignedVec::from_raw_parts(ptr, align, len, cap) };
    /// assert_eq!(rebuilt.as_slice(), &[1, 2, 3, 4, 5]);
    /// ```
    pub unsafe fn from_raw_parts(ptr: NonNull<u8>, align: usize, len: usize, cap: usize) -> Self {
        Self {
            ptr,
            align,
            len,
            cap,
        }
    }
}

impl AsMut<[u8]> for AlignedVec {
    fn as_mut(&mut self) -> &mut [u8] {
        self.as_mut_slice()
    }
}

impl AsRef<[u8]> for AlignedVec {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl Borrow<[u8]> for AlignedVec {
    fn borrow(&self) -> &[u8] {
        self.as_slice()
    }
}

impl BorrowMut<[u8]> for AlignedVec {
    fn borrow_mut(&mut self) -> &mut [u8] {
        self.as_mut_slice()
    }
}

impl Clone for AlignedVec {
    fn clone(&self) -> Self {
        let mut result = Self::with_capacity(self.align, self.len);
        result.len = self.len;
        // SAFETY: Both pointers are valid to do full copy and not overlap
        unsafe { core::ptr::copy_nonoverlapping(self.as_ptr(), result.as_mut_ptr(), self.len) };
        result
    }
}

impl fmt::Debug for AlignedVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_slice().fmt(f)
    }
}

impl Deref for AlignedVec {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl DerefMut for AlignedVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<I: slice::SliceIndex<[u8]>> Index<I> for AlignedVec {
    type Output = <I as slice::SliceIndex<[u8]>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.as_slice()[index]
    }
}

impl<I: slice::SliceIndex<[u8]>> IndexMut<I> for AlignedVec {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.as_mut_slice()[index]
    }
}

#[cfg(feature = "bytemuck")]
impl<T: bytemuck::NoUninit> From<Vec<T>> for AlignedVec {
    fn from(value: Vec<T>) -> Self {
        let (ptr, len, cap) = value.into_raw_parts();
        // SAFETY: `ptr` from `Vec` is non-null
        let ptr = unsafe { NonNull::new_unchecked(ptr.cast::<u8>()) };
        // SAFETY: the raw parts from `Vec` are valid to be used as `AlignedVec`
        unsafe {
            AlignedVec::from_raw_parts(
                ptr,
                align_of::<T>(),
                len * size_of::<T>(),
                cap * size_of::<T>(),
            )
        }
    }
}

// SAFETY: `AlignedVec`, like `Vec<u8>`, is safe to send to another thread
unsafe impl Send for AlignedVec {}

// SAFETY: `AlignedVec`, like `Vec<u8>`, is safe to share between threads
unsafe impl Sync for AlignedVec {}

impl Unpin for AlignedVec {}
