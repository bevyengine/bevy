use std::{
    alloc::{handle_alloc_error, Layout},
    cell::UnsafeCell,
    ptr::NonNull,
};

use bevy_ptr::{OwningPtr, Ptr, PtrMut};

/// A flat, type-erased data storage type
///
/// Used to densely store homogeneous ECS data.
pub(super) struct BlobVec {
    item_layout: Layout,
    capacity: usize,
    /// Number of elements, not bytes
    len: usize,
    data: NonNull<u8>,
    swap_scratch: NonNull<u8>,
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
            .field("swap_scratch", &self.swap_scratch)
            .finish()
    }
}

impl BlobVec {
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
            let swap_scratch = NonNull::new(std::alloc::alloc(item_layout))
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

    #[inline]
    pub fn layout(&self) -> Layout {
        self.item_layout
    }

    pub fn reserve_exact(&mut self, additional: usize) {
        let available_space = self.capacity - self.len;
        if available_space < additional {
            self.grow_exact(additional - available_space);
        }
    }

    // FIXME: this should probably be an unsafe fn as it shouldn't be called if the layout
    // is for a ZST
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
                    self.get_ptr_mut().as_ptr(),
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
    /// - the memory in the [`BlobVec`] starting at index `index`, of a size matching this [`BlobVec`]'s
    /// `item_layout`, must have been previously allocated.
    #[inline]
    pub unsafe fn initialize_unchecked(&mut self, index: usize, value: OwningPtr<'_>) {
        debug_assert!(index < self.len());
        let ptr = self.get_unchecked_mut(index);
        std::ptr::copy_nonoverlapping::<u8>(value.as_ptr(), ptr.as_ptr(), self.item_layout.size());
    }

    /// # Safety
    /// - index must be in-bounds
    /// - the memory in the [`BlobVec`] starting at index `index`, of a size matching this
    /// [`BlobVec`]'s `item_layout`, must have been previously initialized with an item matching
    /// this [`BlobVec`]'s `item_layout`
    /// - the memory at `*value` must also be previously initialized with an item matching this
    /// [`BlobVec`]'s `item_layout`
    pub unsafe fn replace_unchecked(&mut self, index: usize, value: OwningPtr<'_>) {
        debug_assert!(index < self.len());
        // If `drop` panics, then when the collection is dropped during stack unwinding, the
        // collection's `Drop` impl will call `drop` again for the old value (which is still stored
        // in the collection), so we get a double drop. To prevent that, we set len to 0 until we're
        // done.
        let old_len = self.len;
        let ptr = self.get_unchecked_mut(index).promote().as_ptr();
        self.len = 0;
        // Drop the old value, then write back, justifying the promotion
        if let Some(drop) = self.drop {
            (drop)(OwningPtr::new(NonNull::new_unchecked(ptr)));
        }
        std::ptr::copy_nonoverlapping::<u8>(value.as_ptr(), ptr, self.item_layout.size());
        self.len = old_len;
    }

    /// Pushes a value to the [`BlobVec`].
    ///
    /// # Safety
    /// `value` must be valid to add to this [`BlobVec`]
    #[inline]
    pub unsafe fn push(&mut self, value: OwningPtr<'_>) {
        self.reserve_exact(1);
        let index = self.len;
        self.len += 1;
        self.initialize_unchecked(index, value);
    }

    /// # Safety
    /// `len` must be <= `capacity`. if length is decreased, "out of bounds" items must be dropped.
    /// Newly added items must be immediately populated with valid values and length must be
    /// increased. For better unwind safety, call [`BlobVec::set_len`] _after_ populating a new
    /// value.
    pub unsafe fn set_len(&mut self, len: usize) {
        debug_assert!(len <= self.capacity());
        self.len = len;
    }

    /// Performs a "swap remove" at the given `index`, which removes the item at `index` and moves
    /// the last item in the [`BlobVec`] to `index` (if `index` is not the last item). It is the
    /// caller's responsibility to drop the returned pointer, if that is desirable.
    ///
    /// # Safety
    /// It is the caller's responsibility to ensure that `index` is < `self.len()`
    #[inline]
    #[must_use = "The returned pointer should be used to dropped the removed element"]
    pub unsafe fn swap_remove_and_forget_unchecked(&mut self, index: usize) -> OwningPtr<'_> {
        // FIXME: This should probably just use `core::ptr::swap` and return an `OwningPtr`
        //        into the underlying `BlobVec` allocation, and remove swap_scratch

        debug_assert!(index < self.len());
        let last = self.len - 1;
        let swap_scratch = self.swap_scratch.as_ptr();
        std::ptr::copy_nonoverlapping::<u8>(
            self.get_unchecked_mut(index).as_ptr(),
            swap_scratch,
            self.item_layout.size(),
        );
        std::ptr::copy::<u8>(
            self.get_unchecked_mut(last).as_ptr(),
            self.get_unchecked_mut(index).as_ptr(),
            self.item_layout.size(),
        );
        self.len -= 1;
        OwningPtr::new(self.swap_scratch)
    }

    /// # Safety
    /// It is the caller's responsibility to ensure that `index` is < self.len()
    #[inline]
    pub unsafe fn swap_remove_and_drop_unchecked(&mut self, index: usize) {
        debug_assert!(index < self.len());
        let drop = self.drop;
        let value = self.swap_remove_and_forget_unchecked(index);
        if let Some(drop) = drop {
            (drop)(value);
        }
    }

    /// # Safety
    /// It is the caller's responsibility to ensure that `index` is < self.len()
    #[inline]
    pub unsafe fn get_unchecked(&self, index: usize) -> Ptr<'_> {
        debug_assert!(index < self.len());
        self.get_ptr().byte_add(index * self.item_layout.size())
    }

    /// # Safety
    /// It is the caller's responsibility to ensure that `index` is < self.len()
    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> PtrMut<'_> {
        debug_assert!(index < self.len());
        let layout_size = self.item_layout.size();
        self.get_ptr_mut().byte_add(index * layout_size)
    }

    /// Gets a [`Ptr`] to the start of the vec
    #[inline]
    pub fn get_ptr(&self) -> Ptr<'_> {
        // SAFE: the inner data will remain valid for as long as 'self.
        unsafe { Ptr::new(self.data) }
    }

    /// Gets a [`PtrMut`] to the start of the vec
    #[inline]
    pub fn get_ptr_mut(&mut self) -> PtrMut<'_> {
        // SAFE: the inner data will remain valid for as long as 'self.
        unsafe { PtrMut::new(self.data) }
    }

    /// Get a reference to the entire [`BlobVec`] as if it were an array with elements of type `T`
    ///
    /// # Safety
    /// The type `T` must be the type of the items in this [`BlobVec`].
    pub unsafe fn get_slice<T>(&self) -> &[UnsafeCell<T>] {
        // SAFE: the inner data will remain valid for as long as 'self.
        std::slice::from_raw_parts(self.data.as_ptr() as *const UnsafeCell<T>, self.len)
    }

    pub fn clear(&mut self) {
        let len = self.len;
        // We set len to 0 _before_ dropping elements for unwind safety. This ensures we don't
        // accidentally drop elements twice in the event of a drop impl panicking.
        self.len = 0;
        if let Some(drop) = self.drop {
            let layout_size = self.item_layout.size();
            for i in 0..len {
                unsafe {
                    // NOTE: this doesn't use self.get_unchecked(i) because the debug_assert on index
                    // will panic here due to self.len being set to 0
                    let ptr = self.get_ptr_mut().byte_add(i * layout_size).promote();
                    (drop)(ptr);
                }
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
                std::alloc::dealloc(self.get_ptr_mut().as_ptr(), array_layout);
                std::alloc::dealloc(self.swap_scratch.as_ptr(), self.item_layout);
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
    use crate::ptr::OwningPtr;

    use super::BlobVec;
    use std::{alloc::Layout, cell::RefCell, rc::Rc};

    // SAFETY: The pointer points to a valid value of type `T` and it is safe to drop this value.
    unsafe fn drop_ptr<T>(x: OwningPtr<'_>) {
        x.drop_as::<T>();
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
        // usize doesn't need dropping
        let mut blob_vec = unsafe { BlobVec::new(item_layout, None, 64) };
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
            let mut blob_vec = unsafe { BlobVec::new(item_layout, Some(drop), 2) };
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
        let _ = unsafe { BlobVec::new(item_layout, Some(drop), 0) };
    }
}
