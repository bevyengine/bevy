//! Provides various synchronization alternatives to language primitives.
//!
//! Currently missing from this module are the following items:
//! * `Condvar`
//! * `WaitTimeoutResult`
//! * `mpsc`
//!
//! Otherwise, this is a drop-in replacement for `std::sync`.

pub use barrier::{Barrier, BarrierWaitResult};
pub use lazy_lock::LazyLock;
pub use mutex::{Mutex, MutexGuard};
pub use once::{Once, OnceLock, OnceState};
pub use poison::{LockResult, PoisonError, TryLockError, TryLockResult};
pub use rwlock::{RwLock, RwLockReadGuard, RwLockWriteGuard};

#[cfg(feature = "alloc")]
pub use arc::{Arc, Weak};

pub mod atomic;

mod barrier;
mod lazy_lock;
mod mutex;
mod once;
mod poison;
mod rwlock;

#[cfg(all(feature = "alloc", feature = "portable-atomic"))]
use portable_atomic_util as arc;

#[cfg(all(feature = "alloc", not(feature = "portable-atomic")))]
use alloc::sync as arc;
