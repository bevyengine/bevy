---
title: "`SystemBuffer` requires `queue()` to be implemented"
pull_requests: [22832]
---

`SystemBuffer` now requires `queue()` to be implemented, instead of `apply().`
`apply()`'s default implementation now delegates to `queue()`.

This is to ensure that a `SystemBuffer` used in an Observer context applies its changes.
In most cases, if `apply()` does not change the `World` structurally,
`apply()` and `queue()` can mutate the `World` directly in the same way.

If `apply()` does not change the `World` structurally, `apply()` should be changed to `queue()`:

```rust
// 0.18
impl SystemBuffer for MySystemBuffer {
  fn apply(&mut self, system_meta: &SystemMeta, world: &mut World) {
    // your impl here
  }
}

// 0.19
impl SystemBuffer for MySystemBuffer {
  fn queue(&mut self, system_meta: &SystemMeta, mut world: DeferredWorld) {
    // your impl here, using a DeferredWorld instead
  }
}
```

If `apply()` does change the `World` structurally, implement both `apply()` and `queue()`.
To queue structural changes to a `DeferredWorld`, add the structural changes to its command queue,
accessible via `world.commands()`.
