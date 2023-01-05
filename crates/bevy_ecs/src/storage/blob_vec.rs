use std::{
    alloc::{handle_alloc_error, Layout},
    cell::UnsafeCell,
    num::NonZeroUsize,
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
            let align = NonZeroUsize::new(item_layout.align()).expect("alignment must be > 0");
            BlobVec {
                data: bevy_ptr::dangling_with_align(align),
                capacity: usize::MAX,
                len: 0,
                item_layout,
                drop,
            }
        } else {
            let mut blob_vec = BlobVec {
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
        if available_space < additional && self.item_layout.size() > 0 {
            // SAFETY: `available_space < additional`, so `additional - available_space > 0`
            let increment = unsafe { NonZeroUsize::new_unchecked(additional - available_space) };
            // SAFETY: not called for ZSTs
            unsafe { self.grow_exact(increment) };
        }
    }

    // SAFETY: must not be called for a ZST item layout
    #[warn(unsafe_op_in_unsafe_fn)] // to allow unsafe blocks in unsafe fn
    unsafe fn grow_exact(&mut self, increment: NonZeroUsize) {
        debug_assert!(self.item_layout.size() != 0);

        let new_capacity = self.capacity + increment.get();
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
            // since the item size is always a multiple of its align, the rounding cannot happen
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
        // If the drop impl for the old value panics then we run the drop impl for `value` too.
        if let Some(drop) = self.drop {
            struct OnDrop<F: FnMut()>(F);
            impl<F: FnMut()> Drop for OnDrop<F> {
                fn drop(&mut self) {
                    (self.0)();
                }
            }
            let value = value.as_ptr();
            let on_unwind = OnDrop(|| (drop)(OwningPtr::new(NonNull::new_unchecked(value))));

            (drop)(OwningPtr::new(NonNull::new_unchecked(ptr)));

            core::mem::forget(on_unwind);
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
    /// It is the caller's responsibility to ensure that `index` is less than `self.len()`.
    #[inline]
    #[must_use = "The returned pointer should be used to dropped the removed element"]
    pub unsafe fn swap_remove_and_forget_unchecked(&mut self, index: usize) -> OwningPtr<'_> {
        debug_assert!(index < self.len());
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
        self.get_ptr_mut().byte_add(new_len * size).promote()
    }

    /// Removes the value at `index` and copies the value stored into `ptr`.
    /// Does not do any bounds checking on `index`.
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
                // SAFETY: `i * layout_size` is inbounds for the allocation, and the item is left unreachable so it can be safely promoted to an `OwningPtr`
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
        // SAFETY: `drop` fn is `None`, usize doesn't need dropping
        let mut blob_vec = unsafe { BlobVec::new(item_layout, None, 64) };
        // SAFETY: `i` is a usize, i.e. the type corresponding to `item_layout`
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
        // SAFETY: drop is able to drop a value of its `item_layout`
        let _ = unsafe { BlobVec::new(item_layout, Some(drop), 0) };
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
        for &Zst in q.iter(&world) {
            count += 1;
        }

        assert_eq!(count, 3);
    }
}
