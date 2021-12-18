use std::{
    alloc::{handle_alloc_error, Layout},
    ptr::NonNull,
};

/// A flat, type-erased data storage type
///
/// Used to densely store homogeneous ECS data.
#[derive(Debug)]
pub struct BlobVec {
    item_layout: Layout,
    capacity: usize,
    len: usize,
    data: NonNull<u8>,
    swap_scratch: NonNull<u8>,
    drop: unsafe fn(*mut u8),
}

impl BlobVec {
    pub fn new(item_layout: Layout, drop: unsafe fn(*mut u8), capacity: usize) -> BlobVec {
        if item_layout.size() == 0 {
            BlobVec {
                swap_scratch: NonNull::dangling(),
                data: NonNull::dangling(),
                capacity: usize::MAX,
                len: 0,
                item_layout,
                drop,
            }
        } else {
            let swap_scratch = NonNull::new(unsafe { std::alloc::alloc(item_layout) })
                .unwrap_or_else(|| std::alloc::handle_alloc_error(item_layout));
            let mut blob_vec = BlobVec {
                swap_scratch,
                data: NonNull::dangling(),
                capacity: 0,
                len: 0,
                item_layout,
                drop,
            };
            blob_vec.reserve_exact(capacity);
            blob_vec
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn reserve_exact(&mut self, additional: usize) {
        let available_space = self.capacity - self.len;
        if available_space < additional {
            self.grow_exact(additional - available_space);
        }
    }

    fn grow_exact(&mut self, increment: usize) {
        debug_assert!(self.item_layout.size() != 0);

        let new_capacity = self.capacity + increment;
        let new_layout =
            array_layout(&self.item_layout, new_capacity).expect("array layout should be valid");
        unsafe {
            let new_data = if self.capacity == 0 {
                std::alloc::alloc(new_layout)
            } else {
                std::alloc::realloc(
                    self.get_ptr().as_ptr(),
                    array_layout(&self.item_layout, self.capacity)
                        .expect("array layout should be valid"),
                    new_layout.size(),
                )
            };

            self.data = NonNull::new(new_data).unwrap_or_else(|| handle_alloc_error(new_layout));
        }
        self.capacity = new_capacity;
    }

    /// # Safety
    /// - index must be in bounds
    /// - the memory in the `BlobVec` starting at index `index`, of a size matching this `BlobVec`'s
    /// `item_layout`, must have been previously allocated, but not initialized yet
    /// - the memory at `*value` must be previously initialized with an item matching this
    /// `BlobVec`'s `item_layout`
    /// - the item that was stored in `*value` is left logically uninitialised/moved out of after
    /// calling this function, and as such should not be used or dropped by the caller.
    #[inline]
    pub unsafe fn initialize_unchecked(&mut self, index: usize, value: *mut u8) {
        debug_assert!(index < self.len());
        let ptr = self.get_unchecked(index);
        std::ptr::copy_nonoverlapping(value, ptr, self.item_layout.size());
    }

    /// # Safety
    /// - index must be in-bounds
    /// - the memory in the `BlobVec` starting at index `index`, of a size matching this `BlobVec`'s
    /// `item_layout`, must have been previously initialized with an item matching this `BlobVec`'s
    /// item_layout
    /// - the memory at `*value` must also be previously initialized with an item matching this
    /// `BlobVec`'s `item_layout`
    /// - the item that was stored in `*value` is left logically uninitialised/moved out of after
    /// calling this function, and as such should not be used or dropped by the caller.
    pub unsafe fn replace_unchecked(&mut self, index: usize, value: *mut u8) {
        debug_assert!(index < self.len());
        let ptr = self.get_unchecked(index);
        // If `drop` panics, then when the collection is dropped during stack unwinding, the
        // collection's `Drop` impl will call `drop` again for the old value (which is still stored
        // in the collection), so we get a double drop. To prevent that, we set len to 0 until we're
        // done.
        let old_len = std::mem::replace(&mut self.len, 0);
        (self.drop)(ptr);
        std::ptr::copy_nonoverlapping(value, ptr, self.item_layout.size());
        self.len = old_len;
    }

    /// Increases the length by one (and grows the vec if needed) with uninitialized memory and
    /// returns the index
    ///
    /// # Safety
    /// the newly allocated space must be immediately populated with a valid value
    #[inline]
    pub unsafe fn push_uninit(&mut self) -> usize {
        self.reserve_exact(1);
        let index = self.len;
        self.len += 1;
        index
    }

    /// # Safety
    /// len must be <= capacity. if length is decreased, "out of bounds" items must be dropped.
    /// Newly added items must be immediately populated with valid values and length must be
    /// increased. For better unwind safety, call [BlobVec::set_len] _after_ populating a new
    /// value.
    pub unsafe fn set_len(&mut self, len: usize) {
        debug_assert!(len <= self.capacity());
        self.len = len;
    }

    /// Performs a "swap remove" at the given `index`, which removes the item at `index` and moves
    /// the last item in the [BlobVec] to `index` (if `index` is not the last item). It is the
    /// caller's responsibility to drop the returned pointer, if that is desirable.
    ///
    /// # Safety
    /// It is the caller's responsibility to ensure that `index` is < self.len()
    /// Callers should _only_ access the returned pointer immediately after calling this function.
    #[inline]
    pub unsafe fn swap_remove_and_forget_unchecked(&mut self, index: usize) -> *mut u8 {
        debug_assert!(index < self.len());
        let last = self.len - 1;
        let swap_scratch = self.swap_scratch.as_ptr();
        std::ptr::copy_nonoverlapping(
            self.get_unchecked(index),
            swap_scratch,
            self.item_layout.size(),
        );
        std::ptr::copy(
            self.get_unchecked(last),
            self.get_unchecked(index),
            self.item_layout.size(),
        );
        self.len -= 1;
        swap_scratch
    }

    /// # Safety
    /// index must be in-bounds
    #[inline]
    pub unsafe fn swap_remove_and_drop_unchecked(&mut self, index: usize) {
        debug_assert!(index < self.len());
        let value = self.swap_remove_and_forget_unchecked(index);
        (self.drop)(value)
    }

    /// # Safety
    /// It is the caller's responsibility to ensure that `index` is < self.len()
    #[inline]
    pub unsafe fn get_unchecked(&self, index: usize) -> *mut u8 {
        debug_assert!(index < self.len());
        self.get_ptr().as_ptr().add(index * self.item_layout.size())
    }

    /// Gets a pointer to the start of the vec
    ///
    /// # Safety
    /// must ensure rust mutability rules are not violated
    #[inline]
    pub unsafe fn get_ptr(&self) -> NonNull<u8> {
        self.data
    }

    pub fn clear(&mut self) {
        let len = self.len;
        // We set len to 0 _before_ dropping elements for unwind safety. This ensures we don't
        // accidentally drop elements twice in the event of a drop impl panicking.
        self.len = 0;
        for i in 0..len {
            unsafe {
                // NOTE: this doesn't use self.get_unchecked(i) because the debug_assert on index
                // will panic here due to self.len being set to 0
                let ptr = self.get_ptr().as_ptr().add(i * self.item_layout.size());
                (self.drop)(ptr);
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
            unsafe {
                std::alloc::dealloc(self.get_ptr().as_ptr(), array_layout);
                std::alloc::dealloc(self.swap_scratch.as_ptr(), self.item_layout);
            }
        }
    }
}

/// From https://doc.rust-lang.org/beta/src/core/alloc/layout.rs.html
fn array_layout(layout: &Layout, n: usize) -> Option<Layout> {
    let (array_layout, offset) = repeat_layout(layout, n)?;
    debug_assert_eq!(layout.size(), offset);
    Some(array_layout)
}

// TODO: replace with Layout::repeat if/when it stabilizes
/// From https://doc.rust-lang.org/beta/src/core/alloc/layout.rs.html
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

/// From https://doc.rust-lang.org/beta/src/core/alloc/layout.rs.html
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
    use super::BlobVec;
    use std::{alloc::Layout, cell::RefCell, rc::Rc};

    // SAFETY: The pointer points to a valid value of type `T` and it is safe to drop this value.
    unsafe fn drop_ptr<T>(x: *mut u8) {
        x.cast::<T>().drop_in_place()
    }

    /// # Safety
    ///
    /// `blob_vec` must have a layout that matches Layout::new::<T>()
    unsafe fn push<T>(blob_vec: &mut BlobVec, mut value: T) {
        let index = blob_vec.push_uninit();
        blob_vec.initialize_unchecked(index, (&mut value as *mut T).cast::<u8>());
        std::mem::forget(value);
    }

    /// # Safety
    ///
    /// `blob_vec` must have a layout that matches Layout::new::<T>()
    unsafe fn swap_remove<T>(blob_vec: &mut BlobVec, index: usize) -> T {
        assert!(index < blob_vec.len());
        let value = blob_vec.swap_remove_and_forget_unchecked(index);
        value.cast::<T>().read()
    }

    /// # Safety
    ///
    /// `blob_vec` must have a layout that matches Layout::new::<T>(), it most store a valid T value
    /// at the given `index`
    unsafe fn get_mut<T>(blob_vec: &mut BlobVec, index: usize) -> &mut T {
        assert!(index < blob_vec.len());
        &mut *blob_vec.get_unchecked(index).cast::<T>()
    }

    #[test]
    fn resize_test() {
        let item_layout = Layout::new::<usize>();
        let drop = drop_ptr::<usize>;
        let mut blob_vec = BlobVec::new(item_layout, drop, 64);
        unsafe {
            for i in 0..1_000 {
                push(&mut blob_vec, i as usize);
            }
        }

        assert_eq!(blob_vec.len(), 1_000);
        assert_eq!(blob_vec.capacity(), 1_000);
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
            let mut blob_vec = BlobVec::new(item_layout, drop, 2);
            assert_eq!(blob_vec.capacity(), 2);
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
                assert_eq!(blob_vec.capacity(), 3);

                let last_index = blob_vec.len() - 1;
                let value = swap_remove::<Foo>(&mut blob_vec, last_index);
                assert_eq!(foo3, value);

                assert_eq!(blob_vec.len(), 2);
                assert_eq!(blob_vec.capacity(), 3);

                let value = swap_remove::<Foo>(&mut blob_vec, 0);
                assert_eq!(foo1, value);
                assert_eq!(blob_vec.len(), 1);
                assert_eq!(blob_vec.capacity(), 3);

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
        let _ = BlobVec::new(item_layout, drop, 0);
    }
}
