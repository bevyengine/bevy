---
title: Changes to glTF coordinate conversion
pull_requests: [20394]
---

Bevy 0.17 added an option for coordinate conversion of glTF files -
`GltfPlugin::convert_coordinates` and `GltfLoaderSettings::convert_coordinates`.
The goal was to ensure that objects facing forward in the glTF matched the
direction of Bevy's `Transform::forward`.

The conversion was disabled by default, so if you didn't enable the option then
you don't have to make any changes - your glTFs will work the same as before.

Conversion is useful because glTF's standard forward is +Z, while Bevy's is -Z
(although not all glTF files follow the standard). Cameras and lights are an
exception - they have the correct `Transform::forward` with or without
conversion. This is because both glTF and Bevy use -Z forward for cameras and
lights.

The Bevy 0.17 conversion was applied to scenes, nodes and meshes within the
glTF. This worked well for some users, but had
[bugs](https://github.com/bevyengine/bevy/issues/20621) and didn't work well for
other users.

In Bevy 0.18, parts of the conversion have been removed or rearranged to avoid
bugs. The `convert_coordinates` boolean has been changed to a struct with
separate options to convert scenes and/or meshes.

```diff
 struct GltfPlugin {
     ...
-    bool convert_coordinates,
+    GltfConvertCoordinates convert_coordinates,
 }
```

```rust
struct GltfConvertCoordinates {
    scenes: bool,
    meshes: bool,
}
```

Whether the changes affect you will depend on how you're using glTFs.

If you simply spawn your glTF as a scene and want it to visually match the
`Transform::forward` of the entity it's spawned on, then you're supported by the
`GltfConvertCoordinates::scenes` option.

If you want the `Mesh` assets within the glTF to be converted, then you're
supported by the `GltfConvertCoordinates::meshes` option. This can be combined
with the `scenes` option if you want both.

There is no longer a way to enable conversion of nodes within the glTF scene.
This change was made to avoid bugs and give other users more options. If you
only needed scene and/or mesh conversion then you're not affected by this
change.

If you want to start using conversion, the easiest way is to enable it for your
entire app through the `GltfPlugin` settings. This example enables scene
conversion:

```rust
App::new()
    .add_plugins(DefaultPlugins.set(GltfPlugin {
        convert_coordinates: GltfConvertCoordinates { scenes: true, ..default() },
        ..default()
    }))
    .run();
```

You can also choose the option per-asset through `GltfLoaderSettings`. This will
override the settings in `GltfPlugin`.

```rust
let handle = asset_server.load_with_settings(
    "fox.gltf#Scene0",
    |settings: &mut GltfLoaderSettings| {
        settings.convert_coordinates = Some(GltfConvertCoordinates { scenes: true, ..default() });
    },
);
```
