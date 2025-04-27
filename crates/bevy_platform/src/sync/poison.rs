//! Provides `LockResult`, `PoisonError`, `TryLockError`, `TryLockResult`

pub use implementation::{LockResult, PoisonError, TryLockError, TryLockResult};

#[cfg(feature = "std")]
use std::sync as implementation;

#[cfg(not(feature = "std"))]
mod implementation {
    use core::{error::Error, fmt};

    /// Fallback implementation of `PoisonError` from the standard library.
    pub struct PoisonError<T> {
        guard: T,
    }

    impl<T> fmt::Debug for PoisonError<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("PoisonError").finish_non_exhaustive()
        }
    }

    impl<T> fmt::Display for PoisonError<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            "poisoned lock: another task failed inside".fmt(f)
        }
    }

    impl<T> Error for PoisonError<T> {}

    impl<T> PoisonError<T> {
        /// Creates a `PoisonError`.
        ///
        /// See the standard library for further details.
        #[cfg(panic = "unwind")]
        pub fn new(guard: T) -> PoisonError<T> {
            PoisonError { guard }
        }

        /// Consumes this error indicating that a lock is poisoned, returning the
        /// underlying guard to allow access regardless.
        ///
        /// See the standard library for further details.
        pub fn into_inner(self) -> T {
            self.guard
        }

        /// Reaches into this error indicating that a lock is poisoned, returning a
        /// reference to the underlying guard to allow access regardless.
        ///
        /// See the standard library for further details.
        pub fn get_ref(&self) -> &T {
            &self.guard
        }

        /// Reaches into this error indicating that a lock is poisoned, returning a
        /// mutable reference to the underlying guard to allow access regardless.
        ///
        /// See the standard library for further details.
        pub fn get_mut(&mut self) -> &mut T {
            &mut self.guard
        }
    }

    /// Fallback implementation of `TryLockError` from the standard library.
    pub enum TryLockError<T> {
        /// The lock could not be acquired because another thread failed while holding
        /// the lock.
        Poisoned(PoisonError<T>),
        /// The lock could not be acquired at this time because the operation would
        /// otherwise block.
        WouldBlock,
    }

    impl<T> From<PoisonError<T>> for TryLockError<T> {
        fn from(err: PoisonError<T>) -> TryLockError<T> {
            TryLockError::Poisoned(err)
        }
    }

    impl<T> fmt::Debug for TryLockError<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match *self {
                TryLockError::Poisoned(..) => "Poisoned(..)".fmt(f),
                TryLockError::WouldBlock => "WouldBlock".fmt(f),
            }
        }
    }

    impl<T> fmt::Display for TryLockError<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match *self {
                TryLockError::Poisoned(..) => "poisoned lock: another task failed inside",
                TryLockError::WouldBlock => "try_lock failed because the operation would block",
            }
            .fmt(f)
        }
    }

    impl<T> Error for TryLockError<T> {}

    /// Fallback implementation of `LockResult` from the standard library.
    pub type LockResult<Guard> = Result<Guard, PoisonError<Guard>>;

    /// Fallback implementation of `TryLockResult` from the standard library.
    pub type TryLockResult<Guard> = Result<Guard, TryLockError<Guard>>;
}
