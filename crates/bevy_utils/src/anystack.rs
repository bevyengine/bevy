struct AnyStackMeta<T> {
    offset: usize,
    func: unsafe fn(value: *mut u8, user_data: &mut T),
}

pub struct AnyStack<T> {
    bytes: Vec<u8>,
    metas: Vec<AnyStackMeta<T>>,
}

impl<T> Default for AnyStack<T> {
    fn default() -> Self {
        Self {
            bytes: vec![],
            metas: vec![],
        }
    }
}

// SAFE: All values pushed onto the stack are required to be [`Send`]
unsafe impl<T> Send for AnyStack<T> {}

// SAFE: All values pushed onto the stack are required to be [`Sync`]
unsafe impl<T> Sync for AnyStack<T> {}

impl<T> AnyStack<T> {
    ////Constructs a new, empty `AnyStack<T>`.
    /// The stack will not allocate until elements are pushed onto it.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of elements in the [`AnyStack`], also referred to as its ‘length’.
    #[inline]
    pub fn len(&self) -> usize {
        self.metas.len()
    }

    /// Returns true if the [`AnyStack`] contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Push a new `value` onto the stack.
    ///
    /// # Safety
    ///
    /// `func` must safely handle the provided `value` bytes.
    /// _NOTE_: the bytes provided to the given function may **NOT** be aligned. If you wish
    /// to read the value, consider using [`std::ptr::read_unalinged`].
    ///
    /// [`AnyStack`] does not drop the contained member.
    /// The user should manually call [`AnyStack::consume`] and in the implementation
    /// of the provided `func` the element should be dropped.
    #[inline]
    pub unsafe fn push<U>(&mut self, value: U, func: unsafe fn(value: *mut u8, user_data: &mut T))
    where
        U: Send + Sync,
    {
        let size = std::mem::size_of::<U>();

        let old_len = self.bytes.len();

        self.bytes.reserve(size);
        std::ptr::copy_nonoverlapping(
            &value as *const U as *const u8,
            self.bytes.as_mut_ptr().add(old_len),
            size,
        );
        self.bytes.set_len(old_len + size);

        self.metas.push(AnyStackMeta {
            offset: old_len,
            func,
        });

        std::mem::forget(value);
    }

    /// Call each user `func` for each inserted value with `user_data`
    /// and then clears the internal bytes/metas vectors.
    ///
    /// # Warning
    ///
    /// This does not [`drop`] the pushed values.
    /// If the value should be dropped, the initially provided `func`
    /// should ensure any necessary cleanup occurs.
    pub fn consume(&mut self, user_data: &mut T) {
        let byte_ptr = self.bytes.as_mut_ptr();
        for meta in self.metas.iter() {
            // SAFE: The safety guarantees are promised to be held by the caller
            // from [`AnyStack::push`].
            // Also each value has it's offset correctly stored in it's assocaited meta.
            unsafe {
                (meta.func)(byte_ptr.add(meta.offset), user_data);
            }
        }

        self.bytes.clear();
        self.metas.clear();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    };

    struct DropCheck(Arc<AtomicU32>);

    impl DropCheck {
        fn new() -> (Self, Arc<AtomicU32>) {
            let drops = Arc::new(AtomicU32::new(0));
            (Self(drops.clone()), drops)
        }
    }

    impl Drop for DropCheck {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// # Safety
    ///
    /// This function is only used when the value is a `DropCheck`.
    /// This also drops the `DropCheck` value and is only
    /// every call once.
    unsafe fn drop_check_func(bytes: *mut u8, _: &mut ()) {
        let _ = bytes.cast::<DropCheck>().read_unaligned();
    }

    #[test]
    fn test_anystack_drop() {
        let mut stack = AnyStack::<()>::new();

        let (dropcheck_a, drops_a) = DropCheck::new();
        let (dropcheck_b, drops_b) = DropCheck::new();

        // SAFE: `noop_func` never reads/write from the provided bytes.
        unsafe {
            stack.push(dropcheck_a, drop_check_func);
            stack.push(dropcheck_b, drop_check_func);
        }

        assert_eq!(drops_a.load(Ordering::Relaxed), 0);
        assert_eq!(drops_b.load(Ordering::Relaxed), 0);

        stack.consume(&mut ());

        assert_eq!(drops_a.load(Ordering::Relaxed), 1);
        assert_eq!(drops_b.load(Ordering::Relaxed), 1);
    }

    struct FakeWorld(u32);

    /// # Safety
    /// `bytes` must point to a valid `u32`
    /// Since `u32` is a primitive type, it does not require a `drop` call.
    unsafe fn increment_fake_world_u32(bytes: *mut u8, world: &mut FakeWorld) {
        world.0 += bytes.cast::<u32>().read_unaligned();
    }

    #[test]
    fn test_anystack() {
        let mut stack = AnyStack::new();

        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);

        // SAFE: the provided function safely handles the `bytes` and provided `user_data`.
        unsafe {
            stack.push(5u32, increment_fake_world_u32);
            stack.push(10u32, increment_fake_world_u32);
        }

        assert!(!stack.is_empty());
        assert_eq!(stack.len(), 2);

        let mut world = FakeWorld(0);

        stack.consume(&mut world);

        assert_eq!(world.0, 15);

        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);
    }
}
