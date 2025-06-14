---
title: Convert glTF coordinates to Bevy's coordinate system
pull_requests: [19633]
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

The most general and portable fix is to make sure the model is oriented the right way by reexporing it.
For example, Blender assumes -Y to be forward:

<!-- TODO: Add png from PR description -->
![Blender Coordinate System](blender-coords.png)

If you cannot or do not want to rotate the glTF itself, you can also rotate it on spawn:

```rust
// old
commands.spawn(SceneRoot(asset_server.load("foo.glb#Scene0")));

// new 
commands.spawn(
  (
    SceneRoot(asset_server.load("foo.glb#Scene0"),
    Transform::from_rotation(Quat::from_rotation_y(PI)))
  )
);
```

If you have a static camera looking at a fixed position, you can negate its X and Z coordinates to move it to the other side of the model:

```rust
// old
commands.spawn((Camera3d::default(), Transform::from_xyz(1.0, 10.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y)));

// new
commands.spawn((Camera3d::default(), Transform::from_xyz(-1.0, 10.0, -4.0).looking_at(Vec3::ZERO, Vec3::Y)));
```

If you have a static camera looking in the default direction, you must additionally flip its rotation to look to +Z:

```rust
// old
commands.spawn((Camera3d::default(), Transform::from_xyz(1.0, 10.0, 4.0)));

// new
commands.spawn((Camera3d::default(), Transform::from_xyz(1.0, 10.0, -4.0).looking_to(Vec3::Z, Vec3::Y)));
```

If you want to continue using the old glTF loading behavior instead, you can suppress the warning by enabling the `convert_coordinates`, but explicitly disabling the coordinate conversion in the `GltfLoaderSettings`:

```rust
let gltf = asset_server.load_with_settings("foo.glb#Scene0", |settings: &mut GltfLoaderSettings| { settings.convert_coordinates = false; });
```
