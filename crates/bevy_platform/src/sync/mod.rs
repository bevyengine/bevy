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

crate::cfg::alloc! {
    pub use arc::{Arc, Weak};

    crate::cfg::arc! {
        if {
            use alloc::sync as arc;
        } else {
            use portable_atomic_util as arc;
        }
    }
}

pub mod atomic;

mod barrier;
mod lazy_lock;
mod mutex;
mod once;
mod poison;
mod rwlock;
