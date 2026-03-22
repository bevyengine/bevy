---
title: "`extra_buffer_usages` moved from `MeshAllocator` to `MeshAllocatorSettings`"
pull_requests: [23444]
---

`extra_buffer_usages` has been moved from `MeshAllocator` to `MeshAllocatorSettings`.
If you were accessing it on `MeshAllocator`, please do so on `MeshAllocatorSettings` now.
