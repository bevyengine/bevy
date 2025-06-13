---
title: Convert glTF coordinates to Bevy's coordinate system
pull_requests: [TODO]
---

glTF uses the following coordinate system:
- forward: Z
- up: Y
- right: -X

and Bevy uses:
- forward: -Z
- up: Y
- right: X

For the longest time, Bevy has simply ignored this distinction. That caused issues when working across programs, as most software respects the
glTF coordinate system when importing and exporting glTFs. Your scene might have looked correct in Blender, Maya, TrenchBroom, etc. but everything would be flipped when importing it into Bevy!

Starting in Bevy 0.17, we now support correctly converting the glTF coordinate system to Bevy's own.
For example a glTF vertex with the coordinates [1, 2, 3] will be imported into Bevy as [-1, 2, -3] instead of [1, 2, 3].

Changing the import behavior means that *all* imported glTFs of *all* users will now be rotated by 180 degrees around the Y axis.
This would be a massive breaking change if done all at once, so we are easing the transition. In Bevy 0.17, glTFs will still be loaded with
the old behavior by default. In this case, the following warning is presented:
> Warning: Starting from Bevy 0.18, all imported glTF models will be rotated by 180 degrees around the Y axis to align with Bevy's coordinate system.
> You are currently importing glTF files with the old behavior. To already opt into the new import behavior, enable the `convert_coordinates` feature. 
> If you want to continue using the old behavior, additionally set the corresponding option in the `GltfLoaderSettings`

As the warning says, we will wait with changing the default import behavior until Bevy 0.18. To opt into the new behavior, activate the `convert_coordinates` feature:

```toml
# Cargo.toml

[dependencies]
bevy = { version = "0.17", features = ["convert_coordinates"] }
```

As said before, this will result in all models looking rotated when imported. The correct way to deal with this depends on your situation.
If you have a static camera, you can simply negate its z coordinate:
```rust
// old
commands.spawn((Camera3d::default(), Transform::from_xyz(1.0, 10.0, 4.0)));

// new
commands.spawn((Camera3d::default(), Transform::from_xyz(1.0, 10.0, -4.0)));
```

If you manually fixed this issue by rotating your model, you can simply remove that workaround:
```rust
// old
commands.spawn(
  (
    SceneRoot(asset_server.load("foo.glb"),
    Transform::from_rotation(Quat::from_rotation_y(PI)))
  )
);

// new
commands.spawn(SceneRoot(asset_server.load("foo.glb")));
```

If both of these don't apply to you, your model itself is oriented the wrong way. Either rotate its `Transform` or, better yet, reexport it facing the right way. For example, Blender assumes -Y to be forward:

![Blender Coordinate System](blender-coords.png)

If you want to continue using the old behavior instead, you can supress the warning by enabling the `convert_coordinates`, but explicitly disabling the coordinate conversion in the `GltfLoaderSettings`:

```rust
let gltf = asset_server.load_with_settings("foo.glb", |settings: &mut GltfLoaderSettings| { settings.convert_coordinates = false; });
```
