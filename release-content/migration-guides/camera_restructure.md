---
title: Camera Restructure
pull_requests: [18873, 19302]
---

As part of the rendering crate reorganization, we've been working to simplify Bevy `Camera`s:

- `Camera.hdr` has been split out into a new marker component, `Hdr`
  - before: `commands.spawn((Camera3d, Camera { hdr: true, ..default() });`
  - after: `commands.spawn((Camera3d, Hdr));`
  - rendering effects can now `#[require(Hdr)]` if they only function with an HDR camera.
    This is currently implemented for `Bloom`, `AutoExposure`, and `Atmosphere`
- `SubCameraView` has been made into a new math primitive: `SubRect`
  - new import path: `bevy::render::primitives::SubRect`
  - `Camera.sub_camera_view` has been renamed to `Camera.crop`, to better represent its
    use for limiting the extents of the scene rendered by a camera.
