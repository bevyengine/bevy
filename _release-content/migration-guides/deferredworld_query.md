---
title: Deprecate `DeferredWorld::query`
pull_requests: []
---

For consistency with other `QueryState` methods,
`DeferredWorld::query` has been deprecated.
Instead, `QueryState::query_mut` now takes `impl Into<DeferredWorld>`,
so it can be called with `&mut World` or `&mut DeferredWorld` or `DeferredWorld`.

```rust
let world: DeferredWorld = ...;
let query_state: QueryState<D, F> = ...;
// 0.19
let query: Query<D, F> = world.query(&mut query_state);
// 0.20
let query: Query<D, F> = query_state.query_mut(&mut world);
```
