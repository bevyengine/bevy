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

crate::cfg::async_executor! {
    if {
        type ExecutorInner<'a> = async_executor::Executor<'a>;
        type LocalExecutorInner<'a> = async_executor::LocalExecutor<'a>;
    } else {
        type ExecutorInner<'a> = crate::edge_executor::Executor<'a, 64>;
        type LocalExecutorInner<'a> = crate::edge_executor::LocalExecutor<'a, 64>;
    }
}

crate::cfg::multi_threaded! {
    pub use async_task::FallibleTask;
}

/// Wrapper around a multi-threading-aware async executor.
/// Spawning will generally require tasks to be `Send` and `Sync` to allow multiple
/// threads to send/receive/advance tasks.
///
/// If you require an executor _without_ the `Send` and `Sync` requirements, consider
/// using [`LocalExecutor`] instead.
#[derive(Deref, DerefMut, Default)]
pub struct Executor<'a>(ExecutorInner<'a>);

/// Wrapper around a single-threaded async executor.
/// Spawning wont generally require tasks to be `Send` and `Sync`, at the cost of
/// this executor itself not being `Send` or `Sync`. This makes it unsuitable for
/// global statics.
///
/// If need to store an executor in a global static, or send across threads,
/// consider using [`Executor`] instead.
#[derive(Deref, DerefMut, Default)]
pub struct LocalExecutor<'a>(LocalExecutorInner<'a>);

impl Executor<'_> {
    /// Construct a new [`Executor`]
    #[expect(clippy::allow_attributes, reason = "This lint may not always trigger.")]
    #[allow(dead_code, reason = "not all feature flags require this function")]
    pub const fn new() -> Self {
        Self(ExecutorInner::new())
    }
}

impl LocalExecutor<'_> {
    /// Construct a new [`LocalExecutor`]
    #[expect(clippy::allow_attributes, reason = "This lint may not always trigger.")]
    #[allow(dead_code, reason = "not all feature flags require this function")]
    pub const fn new() -> Self {
        Self(LocalExecutorInner::new())
    }
}

impl UnwindSafe for Executor<'_> {}

impl RefUnwindSafe for Executor<'_> {}

impl UnwindSafe for LocalExecutor<'_> {}

impl RefUnwindSafe for LocalExecutor<'_> {}

impl fmt::Debug for Executor<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Executor").finish()
    }
}

impl fmt::Debug for LocalExecutor<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalExecutor").finish()
    }
}
