---
title: Remove `bevy::ptr::dangling_with_align()`
pull_requests: [21822]
---

`bevy::ptr::dangling_with_align()` has been removed. Use `NonNull::without_provenance()` instead:

```rust
// 0.17
let ptr = dangling_with_align(align);

// 0.18
let ptr = NonNull::without_provenance(align);
```
