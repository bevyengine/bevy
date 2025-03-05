//! Provides `LazyLock`

pub use lazy_lock::LazyLock;

#[cfg(feature = "std")]
use std::sync as lazy_lock;

#[cfg(not(feature = "std"))]
mod lazy_lock {
    pub use spin::Lazy as LazyLock;
}
