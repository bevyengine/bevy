use std::{cell::UnsafeCell, rc::Rc};

use rand_chacha::ChaCha12Rng;
use rand_core::{RngCore, SeedableRng};

thread_local! {
    // We require `Rc` to avoid premature freeing when `ThreadLocalEntropy` is used within thread-local destructors.
    static SOURCE: Rc<UnsafeCell<ChaCha12Rng>> = Rc::new(UnsafeCell::new(ChaCha12Rng::from_entropy()));
}

pub(crate) struct ThreadLocalEntropy;

impl ThreadLocalEntropy {
    /// Inspired by `rand`'s approach to `ThreadRng` as well as `turborand`'s instantiation methods. The `Rc`
    /// prevents the Rng instance from being cleaned up, giving it a `'static` lifetime. However, it does not
    /// allow mutable access without a cell, so using `UnsafeCell` to bypass overheads associated with
    /// `RefCell`. There's no direct access to the pointer or mutable reference, so we control how long it
    /// lives and can ensure no multiple mutable references exist.
    #[inline]
    fn get_rng(&mut self) -> &'static mut ChaCha12Rng {
        // Obtain pointer to thread local instance of PRNG which with Rc, should be !Send & !Sync as well
        // as 'static.
        let rng = SOURCE.with(|source| source.get());

        // SAFETY: We must make sure to stop using `rng` before anyone else creates another
        // mutable reference
        unsafe { &mut *rng }
    }
}

impl RngCore for ThreadLocalEntropy {
    #[inline]
    fn next_u32(&mut self) -> u32 {
        self.get_rng().next_u32()
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        self.get_rng().next_u64()
    }

    #[inline]
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.get_rng().fill_bytes(dest);
    }

    #[inline]
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        self.get_rng().try_fill_bytes(dest)
    }
}
