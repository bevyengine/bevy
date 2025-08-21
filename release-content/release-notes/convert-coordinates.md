---
title: Allow importing glTFs with corrected model forward semantics
authors: ["@janhohenheim"]
pull_requests: [19633, 19685, 19816, 20131, 20122]
---

_CAUTION: This is an experimental feature with [known issues](https://github.com/bevyengine/bevy/issues/20621). Behavior may change in future versions._

Bevy uses the following coordinate system for all worldspace entities that have a `Transform`:

- forward: -Z
- up: Y
- right: X

But glTF is a bit more complicated. Models in glTF scenes use the following coordinate system:

- forward: Z
- up: Y
- right: -X

but cameras and lights in glTF scenes use the following coordinate system:

- forward: -Z
- up: Y
- right: X

As you can see, this clashes with Bevy assumption that everything in the world uses the same coordinate system.
In the past, we only imported glTFs using the camera / light coordinate system for everything, as that is already aligned with Bevy.
In other words, the glTF importer simply assumed that glTF models used -Z as their forward direction, even though they use +Z.

But that meant that on the Bevy side, a glTF model's `Transform::forward()` would actually point backwards from the point of view of the model,
which is counterintuitive and very annoying when working across different art pipelines.

To remedy this, users can now change the import behavior to instead favor correct `Transform::forward()` semantics for models.
The downside is that glTF cameras and lights that have a global identity transform in glTF will now look to +Z instead of -Z in Bevy.
This should not be a problem in many cases, as the whole scene is rotated so that the end result on your screen will be rendered the exact same way.

To globally opt into the behavior that favors glTF models over glTF cameras, you can set `GltfPlugin::use_model_forward_direction`:

```rust
App::new()
    .add_plugins(DefaultPlugins.set(GltfPlugin {
        use_model_forward_direction: true,
        ..default()
    }))
    .run();
```

You can also control this on a per-asset-level:

```rust
let handle = asset_server.load_with_settings(
    "fox.gltf#Scene0",
    |settings: &mut GltfLoaderSettings| {
        settings.use_model_forward_direction = Some(true);
    },
);
```

Setting the above to `None` will fall back to the global setting taken from `GltfPlugin::use_model_forward_direction`.
