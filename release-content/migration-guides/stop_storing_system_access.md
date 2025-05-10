---
title: Stop storing access in systems
pull_requests: [TODO]
---

Bevy used to store component access in all systems,
even though it was only used for top-level systems in schedules.
To reduce memory usage, the component access is now stored in the schedule instead.

`SystemMeta` no longer stores `FilteredAccessSet<ComponentId>`.
It is instead passed as a separate parameter when initializing a `SystemParam`.

To better share logic between `SystemParam` and `SystemParamBuilder`,
`SystemParam::init_state` has been split into `init_state`, which creates the state value, and `init_access`, which calculates the access.
`SystemParamBuilder::build` now only creates the state, and `SystemParam::init_access` will be called to calculate the access for built parameters.

If you were implementing `SystemParam` manually, you will need to separate the logic into two methods
and change any uses of `system_meta.component_access_set(_mut)` to the new `component_access_set` parameter.
Note that `init_state` no longer has access to `SystemMeta` or `component_access_set`, and `init_access` only has `&state`, so the state can no longer depend on the system.

If you were calling `init_state` manually, you will need to call `init_access` afterwards.

```rust
// 0.16
let param_state = P::init_state(world, &mut meta);
// 0.17
let param_state = P::init_state(world);
let mut component_access_set = FilteredAccessSet::new();
P::init_access(&param_state, &mut meta, &mut component_access_set, world);
```
