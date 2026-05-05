---
title: PlaneMeshBuilder, allow for different number of subdivisions in X and Z directions
pull_requests: [19479]
---

It is now possible to assign a different number of subdivisions on the X and Z axis.

The `subdivisions` field of `PlaneMeshBuilder` has been split into `subdivisions_x` and `subdivisions_z`.

```rust
// Before:
builder.subdivisions = 4
// After:
builder.subdivisions(4)
```
