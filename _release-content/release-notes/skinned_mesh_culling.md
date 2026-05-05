---
title: Improved Skinned Mesh Culling
authors: ["@greeble-dev"]
pull_requests: [21837]
---

Skinned meshes can now update their bounds from joint positions. This mostly
fixes issues like [#4971](https://github.com/bevyengine/bevy/issues/4971), where
meshes would disappear at certain camera angles.

*TODO: Maybe add the [video from the PR](https://github.com/bevyengine/bevy/pull/21837) here?*

If you load your skinned meshes from glTFs then you don't need to make any
changes - the new behavior is automatically enabled.

If you create your own skinned meshes, you'll need to call
`Mesh::generate_skinned_mesh_bounds` or `Mesh::with_generated_skinned_bounds`
and add a `DynamicSkinnedMeshBounds` component to your mesh entity.

```rust
let mut mesh: Mesh = ...;
mesh.generate_skinned_mesh_bounds()?;
 
entity.insert((
    Mesh3d(meshes.add(mesh)),
    DynamicSkinnedMeshBounds,
));
```

The new behavior is reliable for meshes that only use skinning. But it doesn't
account for morph targets, vertex shaders, or anything else that modifies vertex
positions.

If you don't want the new behavior and you load your skinned meshes from glTFs,
set `GltfPlugin::skinned_mesh_bounds_policy` to
`GltfSkinnedMeshBoundsPolicy::BindPose`.

```rust
app.add_plugins(DefaultPlugins.set(GltfPlugin {
    skinned_mesh_bounds_policy: GltfSkinnedMeshBoundsPolicy::BindPose,
    ..default()
}))
```

There's also a `GltfSkinnedMeshBoundsPolicy::NoFrustumCulling` option if you
prefer to entirely disable culling for skinned meshes.

If you want to visualize the bounds, enable these gizmos:

```rust
fn toggle_skinned_mesh_bounds(mut config: ResMut<GizmoConfigStore>) {
    // Toggle drawing of the per-mesh `Aabb` component that's used for culling.
    config.config_mut::<AabbGizmoConfigGroup>().1.draw_all ^= true;
    // Toggle drawing of the per-joint AABBs used to update the `Aabb` component.
    config.config_mut::<SkinnedMeshBoundsGizmoConfigGroup>().1.draw_all ^= true;
}
```

Or you can load a glTF into the scene viewer example and press `j` and `b` to
enable the visualizations.

```sh
cargo run --example scene_viewer --features "free_camera" -- "path/to/your.gltf"
```

If you were using [`bevy_mod_skinned_aabb`](https://github.com/greeble-dev/bevy_mod_skinned_aabb),
see [Bevy 0.19 and `bevy_mod_skinned_aabb`](https://github.com/greeble-dev/bevy_mod_skinned_aabb/blob/main/notes/bevy_0_19.md)
for how to upgrade.
