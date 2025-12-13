//! Provides various atomic alternatives to language primitives.
//!
//! Certain platforms lack complete atomic support, requiring the use of a fallback
//! such as `portable-atomic`.
//! Using these types will ensure the correct atomic provider is used without the need for
//! feature gates in your own code.

pub use atomic_16::{AtomicI16, AtomicU16};
pub use atomic_32::{AtomicI32, AtomicU32};
pub use atomic_64::{AtomicI64, AtomicU64};
pub use atomic_8::{AtomicBool, AtomicI8, AtomicU8};
pub use atomic_ptr::{AtomicIsize, AtomicPtr, AtomicUsize};
pub use core::sync::atomic::Ordering;

#[cfg(target_has_atomic = "8")]
use core::sync::atomic as atomic_8;

#[cfg(not(target_has_atomic = "8"))]
use portable_atomic as atomic_8;

#[cfg(target_has_atomic = "16")]
use core::sync::atomic as atomic_16;

#[cfg(not(target_has_atomic = "16"))]
use portable_atomic as atomic_16;

#[cfg(target_has_atomic = "32")]
use core::sync::atomic as atomic_32;

#[cfg(not(target_has_atomic = "32"))]
use portable_atomic as atomic_32;

#[cfg(target_has_atomic = "64")]
use core::sync::atomic as atomic_64;

#[cfg(not(target_has_atomic = "64"))]
use portable_atomic as atomic_64;

#[cfg(target_has_atomic = "ptr")]
use core::sync::atomic as atomic_ptr;

#[cfg(not(target_has_atomic = "ptr"))]
use portable_atomic as atomic_ptr;
