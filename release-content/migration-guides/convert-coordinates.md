---
title: Allow importing glTFs with a corrected coordinate system
authors: ["@janhohenheim"]
pull_requests: [19633, 19685, 19816]
---

If you're loading a glTF, you will be greeted by the following deprecation warning:

> Starting from Bevy 0.18, by default all imported glTF scenes will be rotated by 180 degrees around the Y axis to align with Bevy's coordinate system.
> You are currently importing glTF scenes using the old behavior. Consider opting-in to the new import behavior by enabling the `gltf_convert_coordinates_default` feature.
> If you encounter any issues please file a bug!
> If you want to continue using the old behavior going forward (even when the default changes in 0.18), manually set the corresponding option in the `GltfPlugin` or `GltfLoaderSettings`.
> See the migration guide for more details.

As the warning says, this means that from now on glTF scenes will imported with a 180 degree rotation around the Y axis when compared to the old behavior.
To understand why this is desirable, we need to take a look at coordinate systems.

Bevy uses the following coordinate system:

- forward: -Z
- up: Y
- right: X

Even though we never explicitly stated this anywhere, it was implicitly accepted that this coordinate system was used for all things that have a `Transform`,
as indicated by e.g. `Transform::forward()` returning the local -Z direction. In contrast, glTF is a bit more complicated. Models loaded from glTF scenes use the following coordinate system:

- forward: Z
- up: Y
- right: -X

but cameras and lights loaded from glTFs use the following coordinate system:

- forward: -Z
- up: Y
- right: X

As you can see, this clashes with how Bevy assumes that everything in the world uses the same coordinate system.
In the past, we have imported glTFs using the camera / light coordinate system for everything, as it is already aligned with Bevy.
In other words, the glTF imported simply assumed that glTF models used -Z as their forward direction, even though they use +Z.

But that meant that a glTF model's `Transform::forward()` would actually point backwards from the point of view of the model,
which is counterintuitive and very annoying when working across different art pipelines.

To remedy this, we want to change the default glTF import behavior so that the coordinate system of glTF *models* instead of glTF *cameras* is aligned with Bevy.
In practice, this means rotating the scene as described above.
The downside is that glTF cameras that have an identity transform in glTF will now look to +Z instead of -Z in Bevy.
This should not be a problem, as the whole scene rotated anyways, so the end result on your screen will look the exact same.

But, since most users load only models and not cameras through glTF,
changing the import behavior in one big swoop would mean that most imported glTF models would suddenly look different, breaking users' scenes!
Not to mention that any bugs in the conversion code would be incredibly frustrating for users.

This is why we are now gradually rolling out support for corrected glTF imports.
As the warning says, you can opt into the new behavior by enabling the `gltf_convert_coordinates_default` feature in your `Cargo.toml`:

```toml
# old behavior, ignores glTF's coordinate system
[dependencies]
bevy = "0.17.0"

# new behavior, converts the coordinate system of all glTF assets into Bevy's coordinate system
[dependencies]
bevy = { version = "0.17.0", features = ["gltf_convert_coordinates_default"] }
```

If you prefer, you can also do this in code by setting `convert_coordinates` on `GltfPlugin`:

```rust
// old behavior, ignores glTF's coordinate system
App::new()
    .add_plugins(DefaultPlugins)
    .run();

// new behavior, converts the coordinate system of all glTF assets into Bevy's coordinate system
App::new()
    .add_plugins(DefaultPlugins.set(GltfPlugin {
        convert_coordinates: true,
        ..default()
    }))
    .run();
```

If you want to continue using the old behavior in the future, you can silence the warning by enabling the `gltf_convert_coordinates_default` feature
and explicitly setting `convert_coordinates: false` on `GltfPlugin`.

You can also control this on a per-asset-level:

```rust
// Use the global default
let handle = asset_server.load("fox.gltf#Scene0");

// Manually opt in or out of coordinate conversion for an individual asset
let handle = asset_server.load_with_settings(
    "fox.gltf#Scene0",
    |settings: &mut GltfLoaderSettings| {
        settings.convert_coordinates = Some(true);
    },
);
```

After opting into the new behavior, your scene will be oriented such that other software's model forward direction correctly corresponds to Bevy's forward direction.

For example, Blender assumes -Y to be forward for models, so exporting the following model to glTF and loading it in Bevy with the new settings will ensure that the fox looks to -Z in Bevy:

<!-- TODO: Add png from PR description -->
![Blender Coordinate System](blender-coords.png)

If you opt into this, please let us know how it's working out! Is your scene looking like you expected? Are the animations playing correctly? Is the camera at the right place? Are the lights shining from the right spots?
