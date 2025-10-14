---
title: CORE_3D_DEPTH_FORMAT remove
authors: ["@raketenben"]
pull_requests: [21493]
---

The `CORE_3D_DEPTH_FORMAT` constant has been removed.

Instead, the depth/stencil TextureFormat is now part of the `MeshPipelineKey`, and should be retrieved from the key using the `depth_stencil_format` function.

The desired depth/stencil TextureFormat can be set on a camera using the new `DepthStencilComponent`.

```rust
/// before
DepthStencilState {
	format: CORE_3D_DEPTH_FORMAT,
	...
}

/// after
DepthStencilState {
	format: key.depth_stencil_format(),
	...
}
```
