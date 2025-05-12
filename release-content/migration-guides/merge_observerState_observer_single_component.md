---
title: Integrate `ObserverState` component into `Observer`.
pull_requests: [18728]
---

`ObserverState` and `Observer` have been merged into a single component.
now you can use `Observer::with_dynamic_runner` to build custom Observe.

```rust
let observe = unsafe {
    Observer::with_dynamic_runner(|mut world, trigger, ptr, propagate| {
        // do something
    })
    .with_event(event_a)
};
world.spawn(observe);
```
