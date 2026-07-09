---
title: WorldQuery trait no longer contains default implementations.
pull_requests: []
---

In previous Bevy versions, `WorldQuery::init_nested_access` and `WorldQuery::update_archetypes` had
default implementations. Most `WorldQuery` implementations do not need these methods, so defaulting
them seems reasonable. However, these methods play an important role in the correctness and even
soundness of `WorldQuery` implementations.

To reduce the chances of invalid `WorldQuery` implementations, `WorldQuery` now requires users to
implement these two methods manually. To maintain existing behavior, just implement the two methods
with empty bodies, like:

```rust
impl WorldQuery {
    // ... the previous WorldQuery implementation, no changes

    // Add these two methods.

    fn init_nested_access(
        _state: &Self::State,
        _system_name: Option<&str>,
        _component_access_set: &mut FilteredAccessSet,
        _world: UnsafeWorldCell,
    ) {
    }

    fn update_archetypes(_state: &mut Self::State, _world: UnsafeWorldCell) {}
}
```
