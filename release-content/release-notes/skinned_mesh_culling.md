---
title: Improved Skinned Mesh Culling
authors: ["@greeble-dev"]
pull_requests: [21837]
---

Skinned meshes can now calculate their bounds from their joint positions. This
mostly fixes issues like [#4971](https://github.com/bevyengine/bevy/issues/4971)
where skinned meshes would disappear at certain camera angles due to incorrect
bounds.

*TODO: Maybe add a video here? The [PR video](https://github.com/user-attachments/assets/d67b2137-aedc-43a8-a141-fc7523074eea)
could be used, but I can make a nicer one.*

The new behavior is reliable for meshes that only use skinning, although the
bounds can be larger than optimal. However, the new behavior doesn't account
for morph targets, vertex shaders, or anything else that modifies vertex
positions.

If you load your skinned meshes from glTFs then you don't need to make any
changes - the new behavior is automatically enabled. But if you'd prefer to
disable it, set `GltfPlugin::skinned_mesh_bounds_policy` to
`GltfSkinnedMeshBoundsPolicy::BindPose`.

```rust
app.add_plugins(DefaultPlugins.set(GltfPlugin {
    skinned_mesh_bounds_policy: GltfSkinnedMeshBoundsPolicy::BindPose,
    ..default()
}))
```

There's also a `GltfSkinnedMeshBoundsPolicy::NoFrustumCulling` option if you
prefer to entirely disable culling for skinned meshes.

If you create your skinned meshes without using the glTF loader, you'll need to
call `Mesh::generate_skinned_mesh_bounds` or
`Mesh::with_generated_skinned_bounds` and add a `DynamicSkinnedMeshBounds`
component to your mesh entity.

```diff
 let mut mesh = ...;
+mesh.generate_skinned_mesh_bounds()?;
 
 entity.insert((
     Mesh3d(meshes.add(mesh)),
+    DynamicSkinnedMeshBounds,
 ));
```

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
enable the visualization.

```sh
cargo run --example scene_viewer --features "free_camera" -- "path/to/your.gltf"
```

If you were using [`bevy_mod_skinned_aabb`](https://github.com/greeble-dev/bevy_mod_skinned_aabb),
see [Bevy 0.18 and `bevy_mod_skinned_aabb`](https://github.com/greeble-dev/bevy_mod_skinned_aabb/blob/main/notes/bevy_upgrade.md).
