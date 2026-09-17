---
title: Pan Orbit Camera
authors: ["@aevyrie, @taishi-sama, @laundmo"]
pull_requests: [25434, 25567]
---
The `bevy_camera_controller` crate now includes a upstreamed version of the awesome
[`bevy_editor_cam`](https://github.com/aevyrie/bevy_editor_cam) crate made by [@aevyrie](https://github.com/aevyrie).

To better support editor workflows where the camera input might be skipped,
the default input plugin was rewritten from scratch to use picking observers.
This allows blocking camera movement simply by setting `event.propagate(false)`
in entity-specific picking observers.

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
