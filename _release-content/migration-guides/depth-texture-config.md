---
title: Depth texture config has moved from Camera3d to a new CameraDepthTexture component
pull_requests: [26005]
---

`depth_texture_usages` and `depth_load_op` have moved from `Camera3d` to a new required
component: `CameraTextureUsages`.

`Camera3d` is now a unit struct. If a depth texture usage other than the default is
desired, configure `CameraTextureUsages` and add it alongside `Camera3d`.
