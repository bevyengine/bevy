---
title: "`TrackedRenderPass::set_index_buffer` no longer takes buffer offset"
pull_requests: [20468]
---

`TrackedRenderPass::set_index_buffer` no longer takes a separate buffer offset argument, which wasn't actually forwarded to wgpu. You have already needed to pass a `BufferSlice` that is sliced to the desired offset/size.

```rust
// Before:
pass.set_index_buffer(indices.slice(1..), 1, IndexFormat::Uint32);
// After:
pass.set_index_buffer(indices.slice(1..), IndexFormat::Uint32);
```
