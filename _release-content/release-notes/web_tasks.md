---
title: Cancellable Web Tasks
authors: ["@NthTensor", "@Gingeh"]
pull_requests: [21795]
---

When a `Task` in Bevy is dropped, it's supposed to be cancelled, stopping the underlying work at the next yield point.
On web, this never worked. `wasm_bindgen_futures::spawn_local` hands your future directly to the JS event loop with no handle to take it back, so Bevy's task wrapper was just a receipt with no power to cancel.

As a result, code that correctly managed task lifetimes on native desktop and mobile platforms silently leaked work on web.

```rust
fn update_background_task(mut task_handle: ResMut<CurrentTask>) {
    // Replaces the old task. On native: the old task is dropped and cancelled.
    // On web: the old task kept running. Oops.
    task_handle.0 = AsyncComputeTaskPool::get().spawn(async { do_work().await });
}
```

Fixing this required a new approach to the WASM executor. The [`web-task`](https://crates.io/crates/web-task) crate, built by our very own `@NthTensor`, builds cooperative cancellation on top of the JS event loop: spawned tasks check an abort flag at every yield point.
Bevy now uses it on WASM, so `Task` drop semantics are finally identical on all platforms.
