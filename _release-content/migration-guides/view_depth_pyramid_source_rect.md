---
title: "`ViewDepthPyramid::new` takes a `URect`"
pull_requests: [26239]
---

`ViewDepthPyramid::new` takes the part of the depth buffer to build the pyramid from as a `URect`, instead of the depth buffer's size.
To build it from the whole depth buffer, pass `URect::from_corners(UVec2::ZERO, size)`.

`ViewDepthPyramid::downsample_depth_with_ctx` no longer takes a size. It uses the `URect` passed to `new`:

```rust
// 0.20
pyramid.downsample_depth_with_ctx(label, &mut ctx, size, &bind_group, &first, &second);

// 0.21
pyramid.downsample_depth_with_ctx(label, &mut ctx, &bind_group, &first, &second);
```

Pipelines that use the `downsample_depth` shader set `immediate_size` to `DOWNSAMPLE_DEPTH_IMMEDIATE_SIZE` instead of `4`.

`OcclusionCullingSubview::depth_texture_size` is removed. The pyramid gets its size from the `URect` passed to `ViewDepthPyramid::new`.
