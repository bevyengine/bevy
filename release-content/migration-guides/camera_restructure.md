---
title: Camera Restructure
pull_requests: [18873]
---

As part of the rendering crate reorganization, we've been working to simplify Bevy `Camera`s:

- `Camera.hdr` has been split out into a new marker component, `Hdr`
  - before: `commands.spawn((Camera3d, Camera { hdr: true, ..default() });`
  - after: `commands.spawn((Camera3d, Hdr));`
  - rendering effects can now `#[require(Hdr)]` if they only function with an HDR camera.
    This is currently implemented for `Bloom`, `AutoExposure`, and `Atmosphere`
