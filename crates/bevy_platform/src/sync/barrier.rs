//! Provides `Barrier` and `BarrierWaitResult`

pub use implementation::{Barrier, BarrierWaitResult};

#[cfg(feature = "std")]
use std::sync as implementation;

#[cfg(not(feature = "std"))]
mod implementation {
    use core::fmt;

    /// Fallback implementation of `Barrier` from the standard library.
    pub struct Barrier {
        inner: spin::Barrier,
    }

    impl Barrier {
        /// Creates a new barrier that can block a given number of threads.
        ///
        /// See the standard library for further details.
        #[must_use]
        pub const fn new(n: usize) -> Self {
            Self {
                inner: spin::Barrier::new(n),
            }
        }

        /// Blocks the current thread until all threads have rendezvoused here.
        ///
        /// See the standard library for further details.
        pub fn wait(&self) -> BarrierWaitResult {
            BarrierWaitResult {
                inner: self.inner.wait(),
            }
        }
    }

    impl fmt::Debug for Barrier {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Barrier").finish_non_exhaustive()
        }
    }

    /// Fallback implementation of `BarrierWaitResult` from the standard library.
    pub struct BarrierWaitResult {
        inner: spin::barrier::BarrierWaitResult,
    }

    impl BarrierWaitResult {
        /// Returns `true` if this thread is the "leader thread" for the call to [`Barrier::wait()`].
        ///
        /// See the standard library for further details.
        #[must_use]
        pub fn is_leader(&self) -> bool {
            self.inner.is_leader()
        }
    }

    impl fmt::Debug for BarrierWaitResult {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("BarrierWaitResult")
                .field("is_leader", &self.is_leader())
                .finish()
        }
    }
}
