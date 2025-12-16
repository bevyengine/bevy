---
title: "Removed `dummy_white_gpu_image`"
pull_requests: [21572]
---

`MeshPipeline`, `Mesh2dPipeline` and `SpritePipeline`  no longer have `dummy_white_gpu_image`.

`MeshPipeline` and `Mesh2dPipeline` no longer have `get_image_texture()` in their `impl`.

The method `build_dummy_white_gpu_image()` and `get_image_texture()` can be used if needed.
