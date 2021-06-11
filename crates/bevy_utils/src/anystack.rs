struct AnyStackMeta {
    offset: usize,
    func: unsafe fn(value: *mut u8, user_data: *mut u8),
}

pub struct AnyStack {
    bytes: Vec<u8>,
    metas: Vec<AnyStackMeta>,
}

impl Default for AnyStack {
    fn default() -> Self {
        Self {
            bytes: vec![],
            metas: vec![],
        }
    }
}

unsafe impl Send for AnyStack {}
unsafe impl Sync for AnyStack {}

impl AnyStack {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.metas.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clears in internal bytes and metas storage.
    ///
    /// # Safety
    ///
    /// This does not [`drop`] the pushed values.
    /// The pushed values must be dropped via [`AnyStack::apply`] and
    /// the provided `func` before calling this function.
    #[inline]
    pub unsafe fn clear(&mut self) {
        self.bytes.clear();
        self.metas.clear();
    }

    /// Push a new `value` onto the stack.
    ///
    /// # Safety
    ///
    /// `func` must safely handle the provided `value` bytes and `user_data` bytes.
    /// [`AnyStack`] does not drop the contained member.
    /// The user should manually call [`AnyStack::apply`] and in the implementation
    /// of the provided `func` function from [`AnyStack::push`], the element should
    /// be dropped.
    pub unsafe fn push<U>(
        &mut self,
        value: U,
        func: unsafe fn(value: *mut u8, user_data: *mut u8),
    ) {
        let align = std::mem::align_of::<U>();
        let size = std::mem::size_of::<U>();

        if self.is_empty() {
            self.bytes.reserve(size);
        }

        let old_len = self.bytes.len();

        let aligned_offset = loop {
            let aligned_offset = self.bytes.as_ptr().add(old_len).align_offset(align);

            if old_len + aligned_offset + size > self.bytes.capacity() {
                self.bytes.reserve(aligned_offset + size);
            } else {
                break aligned_offset;
            }
        };

        let offset = old_len + aligned_offset;
        let total_bytes = size + aligned_offset;
        self.bytes.set_len(old_len + total_bytes);

        self.metas.push(AnyStackMeta { offset, func });

        std::ptr::copy_nonoverlapping(
            &value as *const U as *const u8,
            self.bytes.as_mut_ptr().add(offset),
            size,
        );

        std::mem::forget(value);
    }

    /// Call each user `func` for each inserted value with `user_data`.
    ///
    /// # Safety
    ///
    /// It is up to the user to safely handle `user_data` in each of the initially
    /// provided `func` functions in [`AnyStack::push`].
    pub unsafe fn apply(&mut self, user_data: *mut u8) {
        let byte_ptr = self.bytes.as_mut_ptr();
        for meta in self.metas.iter() {
            (meta.func)(byte_ptr.add(meta.offset), user_data);
        }
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
    /// This function does not touch the `user_data` provided,
    /// and this is only used when the value is a `DropCheck`.
    /// Lastly, this drops the `DropCheck` value and is only
    /// every call once.
    unsafe fn drop_check_func(bytes: *mut u8, _: *mut u8) {
        assert_eq!(bytes.align_offset(std::mem::align_of::<DropCheck>()), 0);
        let _ = bytes.cast::<DropCheck>().read();
    }

    #[test]
    fn test_anystack_drop() {
        let mut stack = AnyStack::new();

        let (dropcheck_a, drops_a) = DropCheck::new();
        let (dropcheck_b, drops_b) = DropCheck::new();

        // SAFE: `noop_func` never reads/write from the provided bytes.
        unsafe {
            stack.push(dropcheck_a, drop_check_func);
            stack.push(dropcheck_b, drop_check_func);
        }

        assert_eq!(drops_a.load(Ordering::Relaxed), 0);
        assert_eq!(drops_b.load(Ordering::Relaxed), 0);

        // SAFE: The `drop_check_func` does not access the null `user_data`.
        unsafe {
            stack.apply(std::ptr::null_mut());
        }

        assert_eq!(drops_a.load(Ordering::Relaxed), 1);
        assert_eq!(drops_b.load(Ordering::Relaxed), 1);
    }

    struct FakeWorld(u32);

    /// # Safety
    /// `bytes` must point to a valid `u32`
    /// `world` must point to a mutable `FakeWorld`.
    /// Since `u32` is a primitive type, it does not require a `drop` call.
    unsafe fn increment_fake_world_u32(bytes: *mut u8, world: *mut u8) {
        let world = &mut *world.cast::<FakeWorld>();
        world.0 += *bytes.cast::<u32>();
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

        // SAFE: the given `user_data` is a `&mut FakeWorld` which is safely
        // handled in the invocation of `increment_fake_world_u32`
        unsafe {
            stack.apply(&mut world as *mut FakeWorld as *mut u8);
        }

        assert_eq!(world.0, 15);

        // SAFE: the data is only `u32` so they don't need to be dropped.
        unsafe {
            stack.clear();
        }

        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);
    }
}
