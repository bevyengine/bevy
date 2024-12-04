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

#[derive(Deref, DerefMut, Default)]
pub struct Executor<'a>(ExecutorInner<'a>);

#[derive(Deref, DerefMut, Default)]
pub struct LocalExecutor<'a>(LocalExecutorInner<'a>);

impl Executor<'_> {
    #[allow(dead_code, reason = "not all feature flags require this function")]
    pub const fn new() -> Self {
        Self(ExecutorInner::new())
    }
}

impl LocalExecutor<'_> {
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
