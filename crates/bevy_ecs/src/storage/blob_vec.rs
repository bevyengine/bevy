use std::{
    alloc::{handle_alloc_error, Layout},
    cell::UnsafeCell,
    num::NonZeroUsize,
    ptr::NonNull,
};

use bevy_ptr::{OwningPtr, Ptr, PtrMut};
use bevy_utils::OnDrop;

/// A flat, type-erased data storage type
///
/// Used to densely store homogeneous ECS data. A blob is usually just an arbitrary block of contiguous memory without any identity, and
/// could be used to represent any arbitrary data (i.e. string, arrays, etc). This type is an extendable and re-allocatable blob, which makes
/// it a blobby Vec, a `BlobVec`.
pub(super) struct BlobVec {
    item_layout: Layout,
    capacity: usize,
    /// Number of elements, not bytes
    len: usize,
    // the `data` ptr's layout is always `array_layout(item_layout, capacity)`
    data: NonNull<u8>,
    // None if the underlying type doesn't need to be dropped
    drop: Option<unsafe fn(OwningPtr<'_>)>,
}

// We want to ignore the `drop` field in our `Debug` impl
impl std::fmt::Debug for BlobVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlobVec")
            .field("item_layout", &self.item_layout)
            .field("capacity", &self.capacity)
            .field("len", &self.len)
            .field("data", &self.data)
            .finish()
    }
}

impl BlobVec {
    /// Creates a new [`BlobVec`] with the specified `capacity`.
    ///
    /// `drop` is an optional function pointer that is meant to be invoked when any element in the [`BlobVec`]
    /// should be dropped. For all Rust-based types, this should match 1:1 with the implementation of [`Drop`]
    /// if present, and should be `None` if `T: !Drop`. For non-Rust based types, this should match any cleanup
    /// processes typically associated with the stored element.
    ///
    /// # Safety
    ///
    /// `drop` should be safe to call with an [`OwningPtr`] pointing to any item that's been pushed into this [`BlobVec`].
    ///
    /// If `drop` is `None`, the items will be leaked. This should generally be set as None based on [`needs_drop`].
    ///
    /// [`needs_drop`]: core::mem::needs_drop
    pub unsafe fn new(
        item_layout: Layout,
        drop: Option<unsafe fn(OwningPtr<'_>)>,
        capacity: usize,
    ) -> BlobVec {
        let align = NonZeroUsize::new(item_layout.align()).expect("alignment must be > 0");
        let data = bevy_ptr::dangling_with_align(align);
        if item_layout.size() == 0 {
            BlobVec {
                data,
                // ZST `BlobVec` max size is `usize::MAX`, and `reserve_exact` for ZST assumes
                // the capacity is always `usize::MAX` and panics if it overflows.
                capacity: usize::MAX,
                len: 0,
                item_layout,
                drop,
            }
        } else {
            let mut blob_vec = BlobVec {
                data,
                capacity: 0,
                len: 0,
                item_layout,
                drop,
            };
            blob_vec.reserve_exact(capacity);
            blob_vec
        }
    }

    /// Returns the number of elements in the vector.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the vector contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the total number of elements the vector can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the [`Layout`] of the element type stored in the vector.
    #[inline]
    pub fn layout(&self) -> Layout {
        self.item_layout
    }

    /// Reserves the minimum capacity for at least `additional` more elements to be inserted in the given `BlobVec`.
    /// After calling `reserve_exact`, capacity will be greater than or equal to `self.len() + additional`. Does nothing if
    /// the capacity is already sufficient.
    ///
    /// Note that the allocator may give the collection more space than it requests. Therefore, capacity can not be relied upon
    /// to be precisely minimal.
    ///
    /// # Panics
    ///
    /// Panics if new capacity overflows `usize`.
    pub fn reserve_exact(&mut self, additional: usize) {
        let available_space = self.capacity - self.len;
        if available_space < additional {
            // SAFETY: `available_space < additional`, so `additional - available_space > 0`
            let increment = unsafe { NonZeroUsize::new_unchecked(additional - available_space) };
            self.grow_exact(increment);
        }
    }

    /// Reserves the minimum capacity for at least `additional` more elements to be inserted in the given `BlobVec`.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        /// Similar to `reserve_exact`. This method ensures that the capacity will grow at least `self.capacity()` if there is no
        /// enough space to hold `additional` more elements.
        #[cold]
        fn do_reserve(slf: &mut BlobVec, additional: usize) {
            let increment = slf.capacity.max(additional - (slf.capacity - slf.len));
            let increment = NonZeroUsize::new(increment).unwrap();
            slf.grow_exact(increment);
        }

        if self.capacity - self.len < additional {
            do_reserve(self, additional);
        }
    }

    /// Grows the capacity by `increment` elements.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity overflows `usize`.
    /// For ZST it panics unconditionally because ZST `BlobVec` capacity
    /// is initialized to `usize::MAX` and always stays that way.
    fn grow_exact(&mut self, increment: NonZeroUsize) {
        let new_capacity = self
            .capacity
            .checked_add(increment.get())
            .expect("capacity overflow");
        let new_layout =
            array_layout(&self.item_layout, new_capacity).expect("array layout should be valid");
        let new_data = if self.capacity == 0 {
            // SAFETY:
            // - layout has non-zero size as per safety requirement
            unsafe { std::alloc::alloc(new_layout) }
        } else {
            // SAFETY:
            // - ptr was be allocated via this allocator
            // - the layout of the ptr was `array_layout(self.item_layout, self.capacity)`
            // - `item_layout.size() > 0` and `new_capacity > 0`, so the layout size is non-zero
            // - "new_size, when rounded up to the nearest multiple of layout.align(), must not overflow (i.e., the rounded value must be less than usize::MAX)",
            // since the item size is always a multiple of its alignment, the rounding cannot happen
            // here and the overflow is handled in `array_layout`
            unsafe {
                std::alloc::realloc(
                    self.get_ptr_mut().as_ptr(),
                    array_layout(&self.item_layout, self.capacity)
                        .expect("array layout should be valid"),
                    new_layout.size(),
                )
            }
        };

        self.data = NonNull::new(new_data).unwrap_or_else(|| handle_alloc_error(new_layout));
        self.capacity = new_capacity;
    }

    /// Initializes the value at `index` to `value`. This function does not do any bounds checking.
    ///
    /// # Safety
    /// - index must be in bounds
    /// - the memory in the [`BlobVec`] starting at index `index`, of a size matching this [`BlobVec`]'s
    /// `item_layout`, must have been previously allocated.
    #[inline]
    pub unsafe fn initialize_unchecked(&mut self, index: usize, value: OwningPtr<'_>) {
        debug_assert!(index < self.len());
        let ptr = self.get_unchecked_mut(index);
        std::ptr::copy_nonoverlapping::<u8>(value.as_ptr(), ptr.as_ptr(), self.item_layout.size());
    }

    /// Replaces the value at `index` with `value`. This function does not do any bounds checking.
    ///
    /// # Safety
    /// - index must be in-bounds
    /// - the memory in the [`BlobVec`] starting at index `index`, of a size matching this
    /// [`BlobVec`]'s `item_layout`, must have been previously initialized with an item matching
    /// this [`BlobVec`]'s `item_layout`
    /// - the memory at `*value` must also be previously initialized with an item matching this
    /// [`BlobVec`]'s `item_layout`
    pub unsafe fn replace_unchecked(&mut self, index: usize, value: OwningPtr<'_>) {
        debug_assert!(index < self.len());

        // Pointer to the value in the vector that will get replaced.
        // SAFETY: The caller ensures that `index` fits in this vector.
        let destination = NonNull::from(unsafe { self.get_unchecked_mut(index) });
        let source = value.as_ptr();

        if let Some(drop) = self.drop {
            // Temporarily set the length to zero, so that if `drop` panics the caller
            // will not be left with a `BlobVec` containing a dropped element within
            // its initialized range.
            let old_len = self.len;
            self.len = 0;

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

            // Make the vector's contents observable again, since panics are no longer possible.
            self.len = old_len;
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
            std::ptr::copy_nonoverlapping::<u8>(
                source,
                destination.as_ptr(),
                self.item_layout.size(),
            );
        }
    }

    /// Appends an element to the back of the vector.
    ///
    /// # Safety
    /// The `value` must match the [`layout`](`BlobVec::layout`) of the elements in the [`BlobVec`].
    #[inline]
    pub unsafe fn push(&mut self, value: OwningPtr<'_>) {
        self.reserve(1);
        let index = self.len;
        self.len += 1;
        self.initialize_unchecked(index, value);
    }

    /// Forces the length of the vector to `len`.
    ///
    /// # Safety
    /// `len` must be <= `capacity`. if length is decreased, "out of bounds" items must be dropped.
    /// Newly added items must be immediately populated with valid values and length must be
    /// increased. For better unwind safety, call [`BlobVec::set_len`] _after_ populating a new
    /// value.
    #[inline]
    pub unsafe fn set_len(&mut self, len: usize) {
        debug_assert!(len <= self.capacity());
        self.len = len;
    }

    /// Performs a "swap remove" at the given `index`, which removes the item at `index` and moves
    /// the last item in the [`BlobVec`] to `index` (if `index` is not the last item). It is the
    /// caller's responsibility to drop the returned pointer, if that is desirable.
    ///
    /// # Safety
    /// It is the caller's responsibility to ensure that `index` is less than `self.len()`.
    #[inline]
    #[must_use = "The returned pointer should be used to dropped the removed element"]
    pub unsafe fn swap_remove_and_forget_unchecked(&mut self, index: usize) -> OwningPtr<'_> {
        debug_assert!(index < self.len());
        // Since `index` must be strictly less than `self.len` and `index` is at least zero,
        // `self.len` must be at least one. Thus, this cannot underflow.
        let new_len = self.len - 1;
        let size = self.item_layout.size();
        if index != new_len {
            std::ptr::swap_nonoverlapping::<u8>(
                self.get_unchecked_mut(index).as_ptr(),
                self.get_unchecked_mut(new_len).as_ptr(),
                size,
            );
        }
        self.len = new_len;
        // Cannot use get_unchecked here as this is technically out of bounds after changing len.
        // SAFETY:
        // - `new_len` is less than the old len, so it must fit in this vector's allocation.
        // - `size` is a multiple of the erased type's alignment,
        //   so adding a multiple of `size` will preserve alignment.
        // - The removed element lives as long as this vector's mutable reference.
        let p = unsafe { self.get_ptr_mut().byte_add(new_len * size) };
        // SAFETY: The removed element is unreachable by this vector so it's safe to promote the
        // `PtrMut` to an `OwningPtr`.
        unsafe { p.promote() }
    }

    /// Removes the value at `index` and copies the value stored into `ptr`.
    /// Does not do any bounds checking on `index`.
    /// The removed element is replaced by the last element of the `BlobVec`.
    ///
    /// # Safety
    /// It is the caller's responsibility to ensure that `index` is < `self.len()`
    /// and that `self[index]` has been properly initialized.
    #[inline]
    pub unsafe fn swap_remove_unchecked(&mut self, index: usize, ptr: PtrMut<'_>) {
        debug_assert!(index < self.len());
        let last = self.get_unchecked_mut(self.len - 1).as_ptr();
        let target = self.get_unchecked_mut(index).as_ptr();
        // Copy the item at the index into the provided ptr
        std::ptr::copy_nonoverlapping::<u8>(target, ptr.as_ptr(), self.item_layout.size());
        // Recompress the storage by moving the previous last element into the
        // now-free row overwriting the previous data. The removed row may be the last
        // one so a non-overlapping copy must not be used here.
        std::ptr::copy::<u8>(last, target, self.item_layout.size());
        // Invalidate the data stored in the last row, as it has been moved
        self.len -= 1;
    }

    /// Removes the value at `index` and drops it.
    /// Does not do any bounds checking on `index`.
    /// The removed element is replaced by the last element of the `BlobVec`.
    ///
    /// # Safety
    /// It is the caller's responsibility to ensure that `index` is `< self.len()`.
    #[inline]
    pub unsafe fn swap_remove_and_drop_unchecked(&mut self, index: usize) {
        debug_assert!(index < self.len());
        let drop = self.drop;
        let value = self.swap_remove_and_forget_unchecked(index);
        if let Some(drop) = drop {
            drop(value);
        }
    }

    /// Returns a reference to the element at `index`, without doing bounds checking.
    ///
    /// # Safety
    /// It is the caller's responsibility to ensure that `index < self.len()`.
    #[inline]
    pub unsafe fn get_unchecked(&self, index: usize) -> Ptr<'_> {
        debug_assert!(index < self.len());
        let size = self.item_layout.size();
        // SAFETY:
        // - The caller ensures that `index` fits in this vector,
        //   so this operation will not overflow the original allocation.
        // - `size` is a multiple of the erased type's alignment,
        //   so adding a multiple of `size` will preserve alignment.
        // - The element at `index` outlives this vector's reference.
        unsafe { self.get_ptr().byte_add(index * size) }
    }

    /// Returns a mutable reference to the element at `index`, without doing bounds checking.
    ///
    /// # Safety
    /// It is the caller's responsibility to ensure that `index < self.len()`.
    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> PtrMut<'_> {
        debug_assert!(index < self.len());
        let size = self.item_layout.size();
        // SAFETY:
        // - The caller ensures that `index` fits in this vector,
        //   so this operation will not overflow the original allocation.
        // - `size` is a multiple of the erased type's alignment,
        //   so adding a multiple of `size` will preserve alignment.
        // - The element at `index` outlives this vector's mutable reference.
        unsafe { self.get_ptr_mut().byte_add(index * size) }
    }

    /// Gets a [`Ptr`] to the start of the vec
    #[inline]
    pub fn get_ptr(&self) -> Ptr<'_> {
        // SAFETY: the inner data will remain valid for as long as 'self.
        unsafe { Ptr::new(self.data) }
    }

    /// Gets a [`PtrMut`] to the start of the vec
    #[inline]
    pub fn get_ptr_mut(&mut self) -> PtrMut<'_> {
        // SAFETY: the inner data will remain valid for as long as 'self.
        unsafe { PtrMut::new(self.data) }
    }

    /// Get a reference to the entire [`BlobVec`] as if it were an array with elements of type `T`
    ///
    /// # Safety
    /// The type `T` must be the type of the items in this [`BlobVec`].
    pub unsafe fn get_slice<T>(&self) -> &[UnsafeCell<T>] {
        // SAFETY: the inner data will remain valid for as long as 'self.
        unsafe { std::slice::from_raw_parts(self.data.as_ptr() as *const UnsafeCell<T>, self.len) }
    }

    /// Clears the vector, removing (and dropping) all values.
    ///
    /// Note that this method has no effect on the allocated capacity of the vector.
    pub fn clear(&mut self) {
        let len = self.len;
        // We set len to 0 _before_ dropping elements for unwind safety. This ensures we don't
        // accidentally drop elements twice in the event of a drop impl panicking.
        self.len = 0;
        if let Some(drop) = self.drop {
            let size = self.item_layout.size();
            for i in 0..len {
                // SAFETY:
                // * 0 <= `i` < `len`, so `i * size` must be in bounds for the allocation.
                // * `size` is a multiple of the erased type's alignment,
                //   so adding a multiple of `size` will preserve alignment.
                // * The item lives until it's dropped.
                // * The item is left unreachable so it can be safely promoted to an `OwningPtr`.
                // NOTE: `self.get_unchecked_mut(i)` cannot be used here, since the `debug_assert`
                // would panic due to `self.len` being set to 0.
                let item = unsafe { self.get_ptr_mut().byte_add(i * size).promote() };
                // SAFETY: `item` was obtained from this `BlobVec`, so its underlying type must match `drop`.
                unsafe { drop(item) };
            }
        }
    }
}

impl Drop for BlobVec {
    fn drop(&mut self) {
        self.clear();
        let array_layout =
            array_layout(&self.item_layout, self.capacity).expect("array layout should be valid");
        if array_layout.size() > 0 {
            // SAFETY: data ptr layout is correct, swap_scratch ptr layout is correct
            unsafe {
                std::alloc::dealloc(self.get_ptr_mut().as_ptr(), array_layout);
            }
        }
    }
}

/// From <https://doc.rust-lang.org/beta/src/core/alloc/layout.rs.html>
fn array_layout(layout: &Layout, n: usize) -> Option<Layout> {
    let (array_layout, offset) = repeat_layout(layout, n)?;
    debug_assert_eq!(layout.size(), offset);
    Some(array_layout)
}

// TODO: replace with `Layout::repeat` if/when it stabilizes
/// From <https://doc.rust-lang.org/beta/src/core/alloc/layout.rs.html>
fn repeat_layout(layout: &Layout, n: usize) -> Option<(Layout, usize)> {
    // This cannot overflow. Quoting from the invariant of Layout:
    // > `size`, when rounded up to the nearest multiple of `align`,
    // > must not overflow (i.e., the rounded value must be less than
    // > `usize::MAX`)
    let padded_size = layout.size() + padding_needed_for(layout, layout.align());
    let alloc_size = padded_size.checked_mul(n)?;

    // SAFETY: self.align is already known to be valid and alloc_size has been
    // padded already.
    unsafe {
        Some((
            Layout::from_size_align_unchecked(alloc_size, layout.align()),
            padded_size,
        ))
    }
}

/// From <https://doc.rust-lang.org/beta/src/core/alloc/layout.rs.html>
const fn padding_needed_for(layout: &Layout, align: usize) -> usize {
    let len = layout.size();

    // Rounded up value is:
    //   len_rounded_up = (len + align - 1) & !(align - 1);
    // and then we return the padding difference: `len_rounded_up - len`.
    //
    // We use modular arithmetic throughout:
    //
    // 1. align is guaranteed to be > 0, so align - 1 is always
    //    valid.
    //
    // 2. `len + align - 1` can overflow by at most `align - 1`,
    //    so the &-mask with `!(align - 1)` will ensure that in the
    //    case of overflow, `len_rounded_up` will itself be 0.
    //    Thus the returned padding, when added to `len`, yields 0,
    //    which trivially satisfies the alignment `align`.
    //
    // (Of course, attempts to allocate blocks of memory whose
    // size and padding overflow in the above manner should cause
    // the allocator to yield an error anyway.)

    let len_rounded_up = len.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
    len_rounded_up.wrapping_sub(len)
}

#[cfg(test)]
mod tests {
    use crate as bevy_ecs; // required for derive macros
    use crate::{component::Component, ptr::OwningPtr, world::World};

    use super::BlobVec;
    use std::{alloc::Layout, cell::RefCell, mem, rc::Rc};

    unsafe fn drop_ptr<T>(x: OwningPtr<'_>) {
        // SAFETY: The pointer points to a valid value of type `T` and it is safe to drop this value.
        unsafe {
            x.drop_as::<T>();
        }
    }

    /// # Safety
    ///
    /// `blob_vec` must have a layout that matches `Layout::new::<T>()`
    unsafe fn push<T>(blob_vec: &mut BlobVec, value: T) {
        OwningPtr::make(value, |ptr| {
            blob_vec.push(ptr);
        });
    }

    /// # Safety
    ///
    /// `blob_vec` must have a layout that matches `Layout::new::<T>()`
    unsafe fn swap_remove<T>(blob_vec: &mut BlobVec, index: usize) -> T {
        assert!(index < blob_vec.len());
        let value = blob_vec.swap_remove_and_forget_unchecked(index);
        value.read::<T>()
    }

    /// # Safety
    ///
    /// `blob_vec` must have a layout that matches `Layout::new::<T>()`, it most store a valid `T`
    /// value at the given `index`
    unsafe fn get_mut<T>(blob_vec: &mut BlobVec, index: usize) -> &mut T {
        assert!(index < blob_vec.len());
        blob_vec.get_unchecked_mut(index).deref_mut::<T>()
    }

    #[test]
    fn resize_test() {
        let item_layout = Layout::new::<usize>();
        // SAFETY: `drop` fn is `None`, usize doesn't need dropping
        let mut blob_vec = unsafe { BlobVec::new(item_layout, None, 64) };
        // SAFETY: `i` is a usize, i.e. the type corresponding to `item_layout`
        unsafe {
            for i in 0..1_000 {
                push(&mut blob_vec, i as usize);
            }
        }

        assert_eq!(blob_vec.len(), 1_000);
        assert_eq!(blob_vec.capacity(), 1_024);
    }

    #[derive(Debug, Eq, PartialEq, Clone)]
    struct Foo {
        a: u8,
        b: String,
        drop_counter: Rc<RefCell<usize>>,
    }

    impl Drop for Foo {
        fn drop(&mut self) {
            *self.drop_counter.borrow_mut() += 1;
        }
    }

    #[test]
    fn blob_vec() {
        let drop_counter = Rc::new(RefCell::new(0));
        {
            let item_layout = Layout::new::<Foo>();
            let drop = drop_ptr::<Foo>;
            // SAFETY: drop is able to drop a value of its `item_layout`
            let mut blob_vec = unsafe { BlobVec::new(item_layout, Some(drop), 2) };
            assert_eq!(blob_vec.capacity(), 2);
            // SAFETY: the following code only deals with values of type `Foo`, which satisfies the safety requirement of `push`, `get_mut` and `swap_remove` that the
            // values have a layout compatible to the blob vec's `item_layout`.
            // Every index is in range.
            unsafe {
                let foo1 = Foo {
                    a: 42,
                    b: "abc".to_string(),
                    drop_counter: drop_counter.clone(),
                };
                push(&mut blob_vec, foo1.clone());
                assert_eq!(blob_vec.len(), 1);
                assert_eq!(get_mut::<Foo>(&mut blob_vec, 0), &foo1);

                let mut foo2 = Foo {
                    a: 7,
                    b: "xyz".to_string(),
                    drop_counter: drop_counter.clone(),
                };
                push::<Foo>(&mut blob_vec, foo2.clone());
                assert_eq!(blob_vec.len(), 2);
                assert_eq!(blob_vec.capacity(), 2);
                assert_eq!(get_mut::<Foo>(&mut blob_vec, 0), &foo1);
                assert_eq!(get_mut::<Foo>(&mut blob_vec, 1), &foo2);

                get_mut::<Foo>(&mut blob_vec, 1).a += 1;
                assert_eq!(get_mut::<Foo>(&mut blob_vec, 1).a, 8);

                let foo3 = Foo {
                    a: 16,
                    b: "123".to_string(),
                    drop_counter: drop_counter.clone(),
                };

                push(&mut blob_vec, foo3.clone());
                assert_eq!(blob_vec.len(), 3);
                assert_eq!(blob_vec.capacity(), 4);

                let last_index = blob_vec.len() - 1;
                let value = swap_remove::<Foo>(&mut blob_vec, last_index);
                assert_eq!(foo3, value);

                assert_eq!(blob_vec.len(), 2);
                assert_eq!(blob_vec.capacity(), 4);

                let value = swap_remove::<Foo>(&mut blob_vec, 0);
                assert_eq!(foo1, value);
                assert_eq!(blob_vec.len(), 1);
                assert_eq!(blob_vec.capacity(), 4);

                foo2.a = 8;
                assert_eq!(get_mut::<Foo>(&mut blob_vec, 0), &foo2);
            }
        }

        assert_eq!(*drop_counter.borrow(), 6);
    }

    #[test]
    fn blob_vec_drop_empty_capacity() {
        let item_layout = Layout::new::<Foo>();
        let drop = drop_ptr::<Foo>;
        // SAFETY: drop is able to drop a value of its `item_layout`
        let _ = unsafe { BlobVec::new(item_layout, Some(drop), 0) };
    }

    #[test]
    #[should_panic(expected = "capacity overflow")]
    fn blob_vec_zst_size_overflow() {
        // SAFETY: no drop is correct drop for `()`.
        let mut blob_vec = unsafe { BlobVec::new(Layout::new::<()>(), None, 0) };

        assert_eq!(usize::MAX, blob_vec.capacity(), "Self-check");

        // SAFETY: Because `()` is a ZST trivial drop type, and because `BlobVec` capacity
        //   is always `usize::MAX` for ZSTs, we can arbitrarily set the length
        //   and still be sound.
        unsafe {
            blob_vec.set_len(usize::MAX);
        }

        // SAFETY: `BlobVec` was initialized for `()`, so it is safe to push `()` to it.
        unsafe {
            OwningPtr::make((), |ptr| {
                // This should panic because len is usize::MAX, remaining capacity is 0.
                blob_vec.push(ptr);
            });
        }
    }

    #[test]
    #[should_panic(expected = "capacity overflow")]
    fn blob_vec_capacity_overflow() {
        // SAFETY: no drop is correct drop for `u32`.
        let mut blob_vec = unsafe { BlobVec::new(Layout::new::<u32>(), None, 0) };

        assert_eq!(0, blob_vec.capacity(), "Self-check");

        OwningPtr::make(17u32, |ptr| {
            // SAFETY: we push the value of correct type.
            unsafe {
                blob_vec.push(ptr);
            }
        });

        blob_vec.reserve_exact(usize::MAX);
    }

    #[test]
    fn aligned_zst() {
        // NOTE: This test is explicitly for uncovering potential UB with miri.

        #[derive(Component)]
        #[repr(align(32))]
        struct Zst;

        let mut world = World::default();
        world.spawn(Zst);
        world.spawn(Zst);
        world.spawn(Zst);
        world.spawn_empty();

        let mut count = 0;

        let mut q = world.query::<&Zst>();
        for zst in q.iter(&world) {
            // Ensure that the references returned are properly aligned.
            assert_eq!(
                std::ptr::from_ref::<Zst>(zst) as usize % mem::align_of::<Zst>(),
                0
            );
            count += 1;
        }

        assert_eq!(count, 3);
    }
}
