//! A panic in a scope on a pool with no worker threads, which `cfg(test)` can't build.

#![cfg(all(not(target_arch = "wasm32"), feature = "multi_threaded"))]

use core::panic::AssertUnwindSafe;
use std::panic::catch_unwind;

use bevy_tasks::TaskPoolBuilder;

// This test hangs if the scope's drop doesn't drive the pool's executor.
#[test]
fn scope_panic_propagates_on_a_pool_with_no_worker_threads() {
    let pool = TaskPoolBuilder::new().num_threads(0).build();
    assert_eq!(pool.thread_num(), 0);

    let result = catch_unwind(AssertUnwindSafe(|| {
        pool.scope(|scope| {
            scope.spawn(async {});
            panic!("the scope panics before its task runs");
        });
    }));

    assert!(result.is_err());
}
