---
title: Unify `ObserverState` and `Observer` components
pull_requests: [18728]
---

`ObserverState` and `Observer` have been merged into a single component.
now you can use `Observer::with_dynamic_runner` to build custom Observe.

```rust
let observe = unsafe {
    Observer::with_dynamic_runner(|world, trigger_context, event_ptr, trigger_ptr| {
        // do something
    })
    .with_event(event_a)
};
world.spawn(observe);
```
