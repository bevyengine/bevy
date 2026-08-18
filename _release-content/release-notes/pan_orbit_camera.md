---
title: Pan Orbit Camera
authors: ["@taishi-sama"]
pull_requests: [25434]
---

Upstream of awesome crate [`bevy_editor_cam`](https://github.com/aevyrie/bevy_editor_cam) made by [@aevyrie](https://github.com/aevyrie) as part of `bevy_camera_controller` crate!

## Usage

Add `MeshPickingPlugin` and `DefaultPanOrbitCameraPlugins` plugin.
```rust
app.add_plugins((
    MeshPickingPlugin,
    DefaultPanOrbitCameraPlugins,
))
```
Then add `PanOrbitCamera` component on any 3D camera.
```rust
commands.spawn((
    Camera3d::default(),
    PanOrbitCamera::default(),
))
```

Full functionality is shown in the `camera/pan_orbit_camera_cad` example
```sh
cargo run --example pan_orbit_camera_cad --features='pan_orbit_camera https 3d_api jpeg'
```