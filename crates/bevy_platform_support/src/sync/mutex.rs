//! Provides `Mutex` and `MutexGuard`

pub use implementation::{Mutex, MutexGuard};

#[cfg(feature = "std")]
use std::sync as implementation;

#[cfg(not(feature = "std"))]
mod implementation {
    use crate::sync::{LockResult, TryLockError, TryLockResult};
    use core::fmt;

    pub use spin::MutexGuard;

    /// Fallback implementation of `Mutex` from the standard library.
    pub struct Mutex<T: ?Sized> {
        inner: spin::Mutex<T>,
    }

    impl<T> Mutex<T> {
        /// Creates a new mutex in an unlocked state ready for use.
        ///
        /// See the standard library for further details.
        pub const fn new(t: T) -> Self {
            Self {
                inner: spin::Mutex::new(t),
            }
        }
    }

    impl<T: ?Sized> Mutex<T> {
        /// Acquires a mutex, blocking the current thread until it is able to do so.
        ///
        /// See the standard library for further details.
        pub fn lock(&self) -> LockResult<MutexGuard<'_, T>> {
            Ok(self.inner.lock())
        }

        /// Attempts to acquire this lock.
        ///
        /// See the standard library for further details.
        pub fn try_lock(&self) -> TryLockResult<MutexGuard<'_, T>> {
            self.inner.try_lock().ok_or(TryLockError::WouldBlock)
        }

        /// Determines whether the mutex is poisoned.
        ///
        /// See the standard library for further details.
        pub fn is_poisoned(&self) -> bool {
            false
        }

        /// Clear the poisoned state from a mutex.
        ///
        /// See the standard library for further details.
        pub fn clear_poison(&self) {
            // no-op
        }

        /// Consumes this mutex, returning the underlying data.
        ///
        /// See the standard library for further details.
        pub fn into_inner(self) -> LockResult<T>
        where
            T: Sized,
        {
            Ok(self.inner.into_inner())
        }

        /// Returns a mutable reference to the underlying data.
        ///
        /// See the standard library for further details.
        pub fn get_mut(&mut self) -> LockResult<&mut T> {
            Ok(self.inner.get_mut())
        }
    }

    impl<T> From<T> for Mutex<T> {
        fn from(t: T) -> Self {
            Mutex::new(t)
        }
    }

    impl<T: Default> Default for Mutex<T> {
        fn default() -> Mutex<T> {
            Mutex::new(Default::default())
        }
    }

    impl<T: ?Sized + fmt::Debug> fmt::Debug for Mutex<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let mut d = f.debug_struct("Mutex");
            match self.try_lock() {
                Ok(guard) => {
                    d.field("data", &&*guard);
                }
                Err(TryLockError::Poisoned(err)) => {
                    d.field("data", &&**err.get_ref());
                }
                Err(TryLockError::WouldBlock) => {
                    d.field("data", &format_args!("<locked>"));
                }
            }
            d.field("poisoned", &false);
            d.finish_non_exhaustive()
        }
    }
}
