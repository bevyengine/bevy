---
title: Allow importing glTFs with coordinate conversion
authors: ["@janhohenheim", "@greeble-dev"]
pull_requests: [19633, 19685, 19816, 20131, 20122, 20394]
---

Bevy 0.17 adds options for coordinate conversion of glTF files. These options are disabled by default - users
who are already happy with their glTFs do not need to change anything.

Bevy uses the following coordinate system for all worldspace entities that have a `Transform`:

- forward: -Z
- up: Y
- right: X

But the [glTF standard](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#coordinate-system-and-units)
uses a different coordinate system for scenes:

- forward: Z
- up: Y
- right: -X

(glTF cameras and lights are an exception - they use the Bevy coordinate system. Some glTF files may ignore
the glTF scene standard and use their own convention.)

These differences mean that the common case of spawning a glTF scene on a Bevy entity can have surprising behavior:

```rust
let gltf_scene = asset_server.load("fox.gltf#Scene0");
let entity_transform = Transform::IDENTITY;
commands.spawn((SceneRoot(gltf_scene), entity_transform))
```

Some users might expect the glTF's forward to face `entity_transform.forward()`, and be surprised to find it faces
`entity_transform.back()`.

Bevy 0.17 adds some optional settings that can help these users. The recommended option is scene conversion (`GltfConvertCoordinates::scenes`).
If enabled, this would make the example face `spawn_transform.forward()` as expected.

Scene conversion works by applying a corrective transform to the root entity of the loaded scene. The local transforms of
child nodes and meshes within the scene are not changed.

To globally opt into glTF scene conversion, you can set `GltfPlugin::convert_coordinates` during app setup:

```rust
App::new()
    .add_plugins(DefaultPlugins.set(GltfPlugin {
        convert_coordinates: GltfConvertCoordinates { scenes: true, ..default() },
        ..default()
    }))
    .run();
```

You can also choose the option per-asset with `GltfLoaderSettings::convert_coordinates`. This will override
the global option.

```rust
let handle = asset_server.load_with_settings(
    "fox.gltf#Scene0",
    |settings: &mut GltfLoaderSettings| {
        settings.convert_coordinates = Some(GltfConvertCoordinates { scenes: true, ..default() });
    },
);
```

As well as scene conversion, there's also an option to convert the `Mesh` assets within the glTF
(`GltfConvertCoordinates::meshes`). This can be useful for users who spawn their meshes directly rather than through the
scene system. Mesh conversion and scene conversion can be enabled together or on their own.
