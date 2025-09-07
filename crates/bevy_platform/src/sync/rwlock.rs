//! Provides `RwLock`, `RwLockReadGuard`, `RwLockWriteGuard`

pub use implementation::{RwLock, RwLockReadGuard, RwLockWriteGuard};

#[cfg(feature = "std")]
mod implementation {
    use crate::sync::{TryLockError, TryLockResult};
    use core::fmt;

    use std::sync::PoisonError;
    pub use std::sync::{RwLockReadGuard, RwLockWriteGuard};

    /// Fallback implementation of `RwLock` from the standard library.
    #[repr(transparent)]
    pub struct RwLock<T: ?Sized> {
        inner: std::sync::RwLock<T>,
    }

    impl<T> RwLock<T> {
        /// Creates a new instance of an `RwLock<T>` which is unlocked.
        ///
        /// See the standard library for further details.
        pub const fn new(t: T) -> RwLock<T> {
            Self {
                inner: std::sync::RwLock::new(t),
            }
        }
    }

    impl<T: ?Sized> RwLock<T> {
        /// Locks this `RwLock` with shared read access, blocking the current thread
        /// until it can be acquired.
        ///
        /// See the standard library for further details.
        #[inline]
        pub fn read(&self) -> RwLockReadGuard<'_, T> {
            match self.inner.read() {
                Ok(guard) => guard,
                Err(err) => {
                    self.inner.clear_poison();
                    err.into_inner()
                }
            }
        }

        /// Attempts to acquire this `RwLock` with shared read access.
        ///
        /// See the standard library for further details.
        #[inline]
        pub fn try_read(&self) -> TryLockResult<RwLockReadGuard<'_, T>> {
            match self.inner.try_read() {
                Ok(guard) => Ok(guard),
                Err(std::sync::TryLockError::Poisoned(err)) => {
                    self.inner.clear_poison();
                    Ok(err.into_inner())
                }
                Err(std::sync::TryLockError::WouldBlock) => Err(TryLockError::WouldBlock),
            }
        }

        /// Locks this `RwLock` with exclusive write access, blocking the current
        /// thread until it can be acquired.
        ///
        /// See the standard library for further details.
        #[inline]
        pub fn write(&self) -> RwLockWriteGuard<'_, T> {
            match self.inner.write() {
                Ok(guard) => guard,
                Err(err) => {
                    self.inner.clear_poison();
                    err.into_inner()
                }
            }
        }

        /// Attempts to lock this `RwLock` with exclusive write access.
        ///
        /// See the standard library for further details.
        #[inline]
        pub fn try_write(&self) -> TryLockResult<RwLockWriteGuard<'_, T>> {
            match self.inner.try_write() {
                Ok(guard) => Ok(guard),
                Err(std::sync::TryLockError::Poisoned(err)) => {
                    self.inner.clear_poison();
                    Ok(err.into_inner())
                }
                Err(std::sync::TryLockError::WouldBlock) => Err(TryLockError::WouldBlock),
            }
        }

        /// Consumes this `RwLock`, returning the underlying data.
        ///
        /// See the standard library for further details.
        #[inline]
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
        #[inline]
        pub fn get_mut(&mut self) -> &mut T {
            self.inner.get_mut().unwrap_or_else(PoisonError::into_inner)
        }
    }

    impl<T: ?Sized + fmt::Debug> fmt::Debug for RwLock<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let mut d = f.debug_struct("RwLock");
            match self.try_read() {
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

#[cfg(not(feature = "std"))]
mod implementation {
    use crate::sync::{TryLockError, TryLockResult};
    use core::fmt;

    pub use spin::rwlock::{RwLockReadGuard, RwLockWriteGuard};

    /// Fallback implementation of `RwLock` from the standard library.
    #[repr(transparent)]
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
        pub fn read(&self) -> RwLockReadGuard<'_, T> {
            self.inner.read()
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
        pub fn write(&self) -> RwLockWriteGuard<'_, T> {
            self.inner.write()
        }

        /// Attempts to lock this `RwLock` with exclusive write access.
        ///
        /// See the standard library for further details.
        pub fn try_write(&self) -> TryLockResult<RwLockWriteGuard<'_, T>> {
            self.inner.try_write().ok_or(TryLockError::WouldBlock)
        }

        /// Consumes this `RwLock`, returning the underlying data.
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

    impl<T: ?Sized + fmt::Debug> fmt::Debug for RwLock<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let mut d = f.debug_struct("RwLock");
            match self.try_read() {
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
