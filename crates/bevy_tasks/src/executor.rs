//! Provides a fundamental executor primitive appropriate for the target platform
//! and feature set selected.
//! By default, the `async_executor` feature will be enabled, which will rely on
//! [`async-executor`] for the underlying implementation. This requires `std`,
//! so is not suitable for `no_std` contexts. Instead, you must use `edge_executor`,
//! which relies on the alternate [`edge-executor`] backend.
//!
//! [`async-executor`]: https://crates.io/crates/async-executor
//! [`edge-executor`]: https://crates.io/crates/edge-executor

use core::{
    fmt,
    panic::{RefUnwindSafe, UnwindSafe},
};
use derive_more::{Deref, DerefMut};

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        type ExecutorInner<'a> = crate::async_executor::Executor<'a>;
    } else {
        type ExecutorInner<'a> = crate::edge_executor::Executor<'a, 64>;
    }
}

#[cfg(all(feature = "multi_threaded", not(target_arch = "wasm32")))]
pub use async_task::FallibleTask;

/// Wrapper around a multi-threading-aware async executor.
/// spawning will generally require tasks to be `send` and `sync` to allow multiple
/// threads to send/receive/advance tasks.
#[derive(Deref, DerefMut)]
pub(crate) struct Executor<'a>(ExecutorInner<'a>);

impl Executor<'_> {
    /// Construct a new [`Executor`]
    #[expect(clippy::allow_attributes, reason = "This lint may not always trigger.")]
    #[allow(dead_code, reason = "not all feature flags require this function")]
    pub const fn new() -> Self {
        Self(ExecutorInner::new())
    }
}

impl UnwindSafe for Executor<'_> {}

impl RefUnwindSafe for Executor<'_> {}

impl fmt::Debug for Executor<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Executor").finish()
    }
}
