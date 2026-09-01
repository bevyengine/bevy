---
title: "Render resource IDs are now backed by wgpu resource and `define_atomic_id!` is removed"
pull_requests: [25630]
---

The `define_atomic_id!` macro and the unused `ShaderId` type have been removed. Render resource IDs are no longer 32-bit atomic counters; each `XxxId` is now an opaque newtype that holds a strong reference to the wrapped wgpu resource:

- `BufferId`
- `TextureId`
- `TextureViewId`
- `SamplerId`
- `BindGroupId`
- `BindGroupLayoutId`
- `RenderPipelineId`
- `ComputePipelineId`

They are `Clone`, `Eq`, `Ord` and `Hash`, but not `Copy`.

To migrate:

- The ID types are no longer `Copy`. Replace `.copied()` with `.cloned()`, and clone where you relied on `Copy`.
- `XxxId::new()` no longer exists. Get the id from the resource via `resource.id()`.
- An ID holds a strong reference to its resource, so keeping an ID alive keeps the GPU resource allocated and prevents it from being reclaimed. Drop IDs you no longer need.
- Two IDs now compare equal when they wrap the same wgpu resource.
