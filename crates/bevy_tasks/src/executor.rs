//! Provides a fundamental executor primitive appropriate for the target platform
//! and feature set selected.
//! By default, the `async_executor` feature will be enabled, which will rely on
//! [`async-executor`] for the underlying implementation. This requires `std`,
//! so is not suitable for `no_std` contexts. Instead, you must use `edge_executor`,
//! which relies on the alternate [`edge-executor`] backend.
//!
//! [`async-executor`]: https://crates.io/crates/async-executor
//! [`edge-executor`]: https://crates.io/crates/edge-executor

pub use async_task::Task;
use core::{
    fmt,
    panic::{RefUnwindSafe, UnwindSafe},
};
use derive_more::{Deref, DerefMut};

#[cfg(feature = "multi_threaded")]
pub use async_task::FallibleTask;

#[cfg(feature = "async_executor")]
type ExecutorInner<'a> = async_executor::Executor<'a>;

#[cfg(feature = "async_executor")]
type LocalExecutorInner<'a> = async_executor::LocalExecutor<'a>;

#[cfg(all(not(feature = "async_executor"), feature = "edge_executor"))]
type ExecutorInner<'a> = edge_executor::Executor<'a, 64>;

#[cfg(all(not(feature = "async_executor"), feature = "edge_executor"))]
type LocalExecutorInner<'a> = edge_executor::LocalExecutor<'a, 64>;

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
    #[allow(dead_code, reason = "not all feature flags require this function")]
    pub const fn new() -> Self {
        Self(ExecutorInner::new())
    }
}

impl LocalExecutor<'_> {
    /// Construct a new [`LocalExecutor`]
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
