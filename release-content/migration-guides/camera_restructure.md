---
title: Camera Restructure
pull_requests: [19302]
---

As part of the rendering crate reorganization, we've been working to simplify Bevy `Camera`s:

- `SubCameraView` has been made into a new math primitive: `SubRect`
  - new import path: `bevy::render::primitives::SubRect`
  - `Camera.sub_camera_view` has been renamed to `Camera.crop`, to better represent its
    use for limiting the extents of the scene rendered by a camera.
