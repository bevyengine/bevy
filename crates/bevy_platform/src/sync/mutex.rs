//! Provides `Mutex` and `MutexGuard`

pub use implementation::{Mutex, MutexGuard};

#[cfg(feature = "std")]
mod implementation {
    use crate::sync::{TryLockError, TryLockResult};
    use core::fmt;
    use std::sync::PoisonError;

    pub use std::sync::MutexGuard;

    /// Fallback implementation of `Mutex` from the standard library.
    #[repr(transparent)]
    pub struct Mutex<T: ?Sized> {
        inner: std::sync::Mutex<T>,
    }

    impl<T> Mutex<T> {
        /// Creates a new mutex in an unlocked state ready for use.
        ///
        /// See the standard library for further details.
        pub const fn new(t: T) -> Self {
            Self {
                inner: std::sync::Mutex::new(t),
            }
        }
    }

    impl<T: ?Sized> Mutex<T> {
        /// Acquires a mutex, blocking the current thread until it is able to do so.
        ///
        /// See the standard library for further details.
        pub fn lock(&self) -> MutexGuard<'_, T> {
            match self.inner.lock() {
                Ok(guard) => guard,
                Err(err) => {
                    self.inner.clear_poison();
                    err.into_inner()
                }
            }
        }

        /// Attempts to acquire this lock.
        ///
        /// See the standard library for further details.
        pub fn try_lock(&self) -> TryLockResult<MutexGuard<'_, T>> {
            match self.inner.try_lock() {
                Ok(guard) => Ok(guard),
                Err(std::sync::TryLockError::Poisoned(err)) => {
                    self.inner.clear_poison();
                    Ok(err.into_inner())
                }
                Err(std::sync::TryLockError::WouldBlock) => Err(TryLockError::WouldBlock),
            }
        }

        /// Consumes this mutex, returning the underlying data.
        ///
        /// See the standard library for further details.
        pub fn into_inner(self) -> T
        where
            T: Sized,
        {
            self.inner
                .into_inner()
                .unwrap_or_else(PoisonError::into_inner)
        }

        /// Returns a mutable reference to the underlying data.
        ///
        /// See the standard library for further details.
        pub fn get_mut(&mut self) -> &mut T {
            self.inner.get_mut().unwrap_or_else(PoisonError::into_inner)
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
                Err(TryLockError::WouldBlock) => {
                    d.field("data", &format_args!("<locked>"));
                }
            }
            d.field("poisoned", &false);
            d.finish_non_exhaustive()
        }
    }
}

#[cfg(not(feature = "std"))]
mod implementation {
    use crate::sync::{TryLockError, TryLockResult};
    use core::fmt;

    pub use spin::MutexGuard;

    /// Fallback implementation of `Mutex` from the standard library.
    #[repr(transparent)]
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
        pub fn lock(&self) -> MutexGuard<'_, T> {
            self.inner.lock()
        }

        /// Attempts to acquire this lock.
        ///
        /// See the standard library for further details.
        pub fn try_lock(&self) -> TryLockResult<MutexGuard<'_, T>> {
            self.inner.try_lock().ok_or(TryLockError::WouldBlock)
        }

        /// Consumes this mutex, returning the underlying data.
        ///
        /// See the standard library for further details.
        pub fn into_inner(self) -> T
        where
            T: Sized,
        {
            self.inner.into_inner()
        }

        /// Returns a mutable reference to the underlying data.
        ///
        /// See the standard library for further details.
        pub fn get_mut(&mut self) -> &mut T {
            self.inner.get_mut()
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
                Err(TryLockError::WouldBlock) => {
                    d.field("data", &format_args!("<locked>"));
                }
            }
            d.field("poisoned", &false);
            d.finish_non_exhaustive()
        }
    }
}
