//! Provides cell primitives.
//!
//! This is a drop-in replacement for `std::cell::SyncCell`/`std::cell::SyncUnsafeCell`.

mod sync_cell;
mod sync_unsafe_cell;

pub use sync_cell::SyncCell;
pub use sync_unsafe_cell::SyncUnsafeCell;
