//! Provides `LazyLock`

pub use implementation::LazyLock;

#[cfg(feature = "std")]
use std::sync as implementation;

#[cfg(not(feature = "std"))]
mod implementation {
    pub use spin::Lazy as LazyLock;
}
