//! Provides various synchronization alternatives to language primitives.

#[cfg(feature = "alloc")]
pub use arc::{Arc, Weak};

pub mod atomic {
    //! Provides various atomic alternatives to language primitives.
    //!
    //! Certain platforms lack complete atomic support, requiring the use of a fallback
    //! such as `portable-atomic`.
    //! Using these types will ensure the correct atomic provider is used without the need for
    //! feature gates in your own code.

    pub use atomic::{
        AtomicBool, AtomicI16, AtomicI32, AtomicI64, AtomicI8, AtomicIsize, AtomicPtr, AtomicU16,
        AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering,
    };

    #[cfg(not(feature = "portable-atomic"))]
    use core::sync::atomic;

    #[cfg(feature = "portable-atomic")]
    use portable_atomic as atomic;
}

#[cfg(all(feature = "alloc", feature = "portable-atomic"))]
use portable_atomic_util as arc;

#[cfg(all(feature = "alloc", not(feature = "portable-atomic")))]
use alloc::sync as arc;
