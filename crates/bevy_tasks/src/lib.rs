#![doc = include_str!("../README.md")]

use rayon_core::Yield;
use std::future::Future;
use std::task::{Context, Poll};

mod slice;
pub use slice::{ParallelSlice, ParallelSliceMut};

mod task;
pub use task::Task;

#[cfg(all(not(target_arch = "wasm32"), feature = "multi-threaded"))]
mod task_pool;
#[cfg(all(not(target_arch = "wasm32"), feature = "multi-threaded"))]
pub use task_pool::{Scope, TaskPool, TaskPoolBuilder};

#[cfg(any(target_arch = "wasm32", not(feature = "multi-threaded")))]
mod single_threaded_task_pool;
#[cfg(any(target_arch = "wasm32", not(feature = "multi-threaded")))]
pub use single_threaded_task_pool::{FakeTask, Scope, TaskPool, TaskPoolBuilder, ThreadExecutor};

mod usages;
#[cfg(not(target_arch = "wasm32"))]
pub use usages::tick_global_task_pools_on_main_thread;
pub use usages::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool};

#[cfg(all(not(target_arch = "wasm32"), feature = "multi-threaded"))]
mod thread_executor;
#[cfg(all(not(target_arch = "wasm32"), feature = "multi-threaded"))]
pub use thread_executor::{ThreadExecutor, ThreadExecutorTicker};

#[cfg(feature = "async-io")]
pub use async_io::block_on;
pub use futures_lite::future::poll_once;

mod iter;
pub use iter::ParallelIterator;

pub use futures_lite;

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        block_on,
        iter::ParallelIterator,
        slice::{ParallelSlice, ParallelSliceMut},
        usages::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool},
    };
}

use std::num::NonZeroUsize;

/// Gets the logical CPU core count available to the current process.
///
/// This is identical to [`std::thread::available_parallelism`], except
/// it will return a default value of 1 if it internally errors out.
///
/// This will always return at least 1.
pub fn available_parallelism() -> usize {
    std::thread::available_parallelism()
        .map(NonZeroUsize::get)
        .unwrap_or(1)
}

/// Blocks the current thread on a future.
///
/// # Examples
///
/// ```
/// use futures_lite::future;
///
/// let val = future::block_on(async {
///     1 + 2
/// });
///
/// assert_eq!(val, 3);
/// ```
#[cfg(not(feature = "async-io"))]
pub fn block_on<T>(future: impl Future<Output = T>) -> T {
    use core::cell::RefCell;
    use core::task::Waker;

    use parking::Parker;

    // Pin the future on the stack.
    futures_lite::pin!(future);

    // Creates a parker and an associated waker that unparks it.
    fn parker_and_waker() -> (Parker, Waker) {
        let parker = Parker::new();
        let unparker = parker.unparker();
        let waker = Waker::from(unparker);
        (parker, waker)
    }

    thread_local! {
        // Cached parker and waker for efficiency.
        static CACHE: RefCell<(Parker, Waker)> = RefCell::new(parker_and_waker());
    }

    CACHE.with(|cache| {
        // Try grabbing the cached parker and waker.
        let tmp_cached;
        let tmp_fresh;
        let (parker, waker) = match cache.try_borrow_mut() {
            Ok(cache) => {
                // Use the cached parker and waker.
                tmp_cached = cache;
                &*tmp_cached
            }
            Err(_) => {
                // Looks like this is a recursive `block_on()` call.
                // Create a fresh parker and waker.
                tmp_fresh = parker_and_waker();
                &tmp_fresh
            }
        };

        let cx = &mut Context::from_waker(waker);
        // Keep polling until the future is ready.
        loop {
            match future.as_mut().poll(cx) {
                Poll::Ready(output) => return output,
                Poll::Pending => match rayon_core::yield_now() {
                    Some(Yield::Executed) => continue,
                    Some(Yield::Idle) | None => parker.park(),
                },
            }
        }
    })
}
