//! Provides `RwLock`, `RwLockReadGuard`, `RwLockWriteGuard`

pub use implementation::{RwLock, RwLockReadGuard, RwLockWriteGuard};

#[cfg(feature = "std")]
use std::sync as implementation;

#[cfg(not(feature = "std"))]
mod implementation {
    use crate::sync::{LockResult, TryLockError, TryLockResult};
    use core::fmt;

    pub use spin::rwlock::{RwLockReadGuard, RwLockWriteGuard};

    /// Fallback implementation of `RwLock` from the standard library.
    pub struct RwLock<T: ?Sized> {
        inner: spin::RwLock<T>,
    }

    impl<T> RwLock<T> {
        /// Creates a new instance of an `RwLock<T>` which is unlocked.
        ///
        /// See the standard library for further details.
        pub const fn new(t: T) -> RwLock<T> {
            Self {
                inner: spin::RwLock::new(t),
            }
        }
    }

    impl<T: ?Sized> RwLock<T> {
        /// Locks this `RwLock` with shared read access, blocking the current thread
        /// until it can be acquired.
        ///
        /// See the standard library for further details.
        pub fn read(&self) -> LockResult<RwLockReadGuard<'_, T>> {
            Ok(self.inner.read())
        }

        /// Attempts to acquire this `RwLock` with shared read access.
        ///
        /// See the standard library for further details.
        pub fn try_read(&self) -> TryLockResult<RwLockReadGuard<'_, T>> {
            self.inner.try_read().ok_or(TryLockError::WouldBlock)
        }

        /// Locks this `RwLock` with exclusive write access, blocking the current
        /// thread until it can be acquired.
        ///
        /// See the standard library for further details.
        pub fn write(&self) -> LockResult<RwLockWriteGuard<'_, T>> {
            Ok(self.inner.write())
        }

        /// Attempts to lock this `RwLock` with exclusive write access.
        ///
        /// See the standard library for further details.
        pub fn try_write(&self) -> TryLockResult<RwLockWriteGuard<'_, T>> {
            self.inner.try_write().ok_or(TryLockError::WouldBlock)
        }

        /// Determines whether the lock is poisoned.
        ///
        /// See the standard library for further details.
        pub fn is_poisoned(&self) -> bool {
            false
        }

        /// Clear the poisoned state from a lock.
        ///
        /// See the standard library for further details.
        pub fn clear_poison(&self) {
            // no-op
        }

        /// Consumes this `RwLock`, returning the underlying data.
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

    impl<T: ?Sized + fmt::Debug> fmt::Debug for RwLock<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let mut d = f.debug_struct("RwLock");
            match self.try_read() {
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

    impl<T: Default> Default for RwLock<T> {
        fn default() -> RwLock<T> {
            RwLock::new(Default::default())
        }
    }

    impl<T> From<T> for RwLock<T> {
        fn from(t: T) -> Self {
            RwLock::new(t)
        }
    }
}
