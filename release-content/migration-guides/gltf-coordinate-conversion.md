---
title: glTF Coordinate Conversion
pull_requests: [20394]
---

**Bevy 0.17** added experimental options for coordinate conversion of glTF
files - `GltfPlugin::use_model_forward_direction` and
`GltfLoaderSettings::use_model_forward_direction`. In **Bevy 0.18** these
options have changed. The options are disabled by default, so if you haven't
enabled them then your glTFs will work the same as before.

The goal of coordinate conversion is to take objects that face forward in the
glTF and change them to match the direction of Bevy's `Transform::forward`.
Conversion is necessary because glTF's standard scene forward is +Z, while
Bevy's is -Z (although not all glTF files follow the standard, and there are
exceptions for cameras and lights).

In 0.17 the conversion was applied to nodes and meshes within glTF scenes.
This worked well for some users, but had
[bugs](https://github.com/bevyengine/bevy/issues/20621) and didn't work well for
other users. In particular, node conversion caused issues with cameras and
lights.

In 0.18 there are two changes. Firstly, the `use_model_forward_direction` option
has been renamed to `convert_coordinates`, and is now a struct with two separate
options.

```rust
// 0.17
pub struct GltfPlugin {
    use_model_forward_direction: bool,
    // ...
}

// 0.18
pub struct GltfPlugin {
    convert_coordinates: GltfConvertCoordinates,
    // ...
}

pub struct GltfConvertCoordinates {
    rotate_scene_entity: bool,
    rotate_meshes: bool,
}
```

Secondly, the conversion behavior has changed. Nodes within the glTF scene are
no longer converted - instead a new conversion is applied to the scene entity
and mesh primitive entities. Whether these changes affect you will depend on how
you're using glTFs.

- If you never enabled the 0.17 conversion then you don't need to change
  anything - conversion remains disabled by default in 0.18. To check if you
  enabled the conversion, search for `use_model_forward_direction`.

- If you simply spawn your glTF via `SceneRoot` and want it to visually match
  the `Transform::forward` of the entity it's spawned on, then you're still
  supported. The internals of the scene will be different in 0.18, but the
  visual result will be the same. The only option you need to enable is `GltfConvertCoordinates::rotate_scene_entity`.

- If you want the `Mesh` assets in your glTF to be converted then you're
  supported by the `GltfConvertCoordinates::rotate_meshes` option. This can be
  combined with the `rotate_scene_entity` option if you want both.

- If you enabled the 0.17 conversion and aren't sure what to enable in 0.18,
  try enabling both the `rotate_scene_entity` and `rotate_meshes` options. This
  will be closest to the 0.17 behavior.

- If you tried the 0.17 conversion but found it caused issues with cameras or
  lights, then the 0.18 conversion should fix these issues.

- If you relied on node conversion, you'll find that 0.18 no longer applies that
  conversion. This change was made to avoid bugs and give other users more
  options.

If you want to try out glTF coordinate conversion, the simplest method is to
set `GltfPlugin::convert_coordinates` - this option can be set on app startup,
and is applied to all glTFs when they're loaded. For an app that uses
`DefaultPlugins`, the example below shows how to enable just scene conversion.

```rust
App::new()
    .add_plugins(DefaultPlugins.set(GltfPlugin {
        convert_coordinates: GltfConvertCoordinates { rotate_scene_entity: true, ..default() },
        ..default()
    }))
    .run();
```

If you want finer control, you can choose the option per-glTF with
`GltfLoaderSettings`.

```rust
let handle = asset_server.load_with_settings(
    "fox.gltf#Scene0",
    |settings: &mut GltfLoaderSettings| {
        settings.convert_coordinates = Some(GltfConvertCoordinates { rotate_scene_entity: true, ..default() });
    },
);
```
