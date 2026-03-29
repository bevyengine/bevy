//! The objective here is to coordinate two participants that want to share World access:
//!
//! - The main Bevy schedule
//! - Futures and async tasks running on other threads
//!
//! This is done through the bridge primitive introduced in this crate
//!
//!
//! Invariants of this crate:
//!
//! - Normal rust safety invariants for &mut World (aliasing)
//! - At most one future has world access at a time
//! - Futures only access the world while the scoped pointer (managed by the bridge driver) is live
//! - `SystemState` is always initialized before use
//! - Deferred ops are only applied after every future finishes polling and releases world access
//! - The driver can't deadlock
//! - All futures that want world access can eventually complete (assuming fair scheduling by the async runtime)
//! - If the world is dropped, futures don't leak and eventually finish (in an error state)
//!
//!
//! The protocol:
//!
//! Futures (tasks on worker threads)
//! - enqueue requests (create signal guard clones: one kept, one sent)
//!
//! - Driver([`async_world_sync_point`]) (exclusive system, world-owning thread)
//!   1. Drain request queue for this sync point
//!   2. Publish World pointer (via `scoped_static_storage`). Future access scope begins
//!   3. Wake all drained futures
//!
//!  -> Futures race for locks (non-blocking)
//!
//!  -> Success: acquire both locks, do work, complete
//!
//!  -> Failure: signal driver (Drop signal guard), re-enqueue later
//!
//!  -> Direct access: non-queued future polled during scope,
//!  bypasses queue, acquires locks, completes (no signal)
//!   4. Wait for all signal guards to drop + scope mutex released
//!   5. Unpublish pointer, scope ends.
//!   6. Apply any deferred ops from `SystemState` of polled futures
//!   7. Loop (up to [`AsyncTickBudget`]) or return
//!   8. Schedule resumes (normal systems run)
//!
//!
//! Dual locking:
//!
//! The published World pointer lock is managed by the `ScopedStatic` primitive in `scoped_static_storage` (only one future can lock this at a time)
//! `SystemState` locks are managed by the `SystemStateCell` primitive of this crate (futures using different `SystemState` types can work in parallel)
//!
//!
//! Preventing driver deadlocks when futures panic:
//!
//! If a future panics while holding locks, rust's panic unwinding drops destructors in reverse scope order
//! - First, the `SystemState` `MutexGuard` drops (releasing the lock)
//! - Second, the World pointer's scope `MutexGuard` drops (releasing the lock)
//! - Finally, the guard signal constructed by the future during `poll()` drops, and the driver is notified
//!
//! How futures can fail cleanly:
//!
//! If the [`AsyncWorld`] cannot be reached ([`bevy_platform::sync::Weak::upgrade`] fails during `poll()`), the world has been dropped and the future cannot complete.
//!
//! If `SystemState`s are invalid, they can't be used and the future cannot complete
//!
//! Regardless, the future returns Ready(Err) and completes permanently
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://!bevy.org/assets/icon.png",
    html_favicon_url = "https://!bevy.org/assets/icon.png"
)]
#![no_std]

#[cfg(feature = "std")]
extern crate std;

mod bridge_future;
mod bridge_request;
mod plugin;
mod system_state;
mod wake_signal;

pub use crate::bridge_future::{AsyncSystemState, BridgeError};
pub use crate::bridge_request::async_world_sync_point;
pub use crate::plugin::{AsyncPlugin, AsyncTickBudget, AsyncWorld};

/// The async prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        async_world_sync_point, AsyncPlugin, AsyncSystemState, AsyncTickBudget, AsyncWorld,
        BridgeError,
    };
}
