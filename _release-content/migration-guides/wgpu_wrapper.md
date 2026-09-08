---
title: WgpuWrapper has been removed
pull_requests: [25512]
---

`WgpuWrapper` has been removed, and its uses have been replaced with either new types (e.g. `WgpuErrorSource`) or removed from the public API (e.g. `RenderQueue`).

For `RenderQueue`, `RenderAdapter`, `RenderInstance` and `RenderAdapterInfo` in particular their only field holding a `WgpuWrapper` is no longer public. However they still `Deref`/`DerefMut` to their `wgpu` type, and if you were constructing/deconstructing them directly you can instead call `new` and `into_inner` to do so.
