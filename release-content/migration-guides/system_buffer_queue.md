---
title: "`SystemBuffer` requires `queue()` to be implemented"
pull_requests: [22832]
---

`SystemBuffer` now requires `queue()` to be implemented, instead of `apply().` `apply()`'s default implementation now delegates to `queue()`.

This is to ensure that a `SystemBuffer` used in an Observer context applies its changes. In most cases, `apply()` and `queue()` should mutate the `World` in the same way.

For most cases:
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

If `apply()` and `queue()` should mutate the `World` differently, implement both `apply()` and `queue()`.
