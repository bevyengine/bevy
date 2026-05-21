---
title: Improved Skinned Mesh Culling
authors: ["@greeble-dev"]
pull_requests: [21837]
---

In earlier Bevy versions, animated characters and creatures would sometimes vanish mid-animation.
This happened because Bevy used the skeleton's resting position to decide which meshes were on-screen, rather than their actual animated pose.
A character raising their arms could have those arms literally outside the bounding box Bevy used for culling.

Skinned meshes now compute their bounds from actual joint positions each frame, fixing disappearing meshes like those reported in [#4971](https://github.com/bevyengine/bevy/issues/4971). If you load skinned meshes from glTFs, this is automatic — no changes needed.

For hand-crafted skinned meshes, call `Mesh::generate_skinned_mesh_bounds` and add `DynamicSkinnedMeshBounds` to the entity:

```rust
let mut mesh: Mesh = ...;
mesh.generate_skinned_mesh_bounds()?;

entity.insert((
    Mesh3d(meshes.add(mesh)),
    DynamicSkinnedMeshBounds,
));
```

## Limitations

Joint-based bounds only account for skinning. If your mesh uses morph targets, vertex shaders, or anything else that moves vertices independently of joints, the computed bounds may still be wrong and meshes may still cull incorrectly. You should precompute a permissive bounding box,
or disable culling completely.

## Opting out

If you load skinned meshes from glTFs and want the old behavior, set `GltfPlugin::skinned_mesh_bounds_policy`:

```rust
app.add_plugins(DefaultPlugins.set(GltfPlugin {
    skinned_mesh_bounds_policy: GltfSkinnedMeshBoundsPolicy::BindPose,
    ..default()
}))
```

There's also `GltfSkinnedMeshBoundsPolicy::NoFrustumCulling` if you'd rather disable culling for skinned meshes entirely.

## Debugging

To visualize the bounds, enable these gizmos:

```rust
fn toggle_skinned_mesh_bounds(mut config: ResMut<GizmoConfigStore>) {
    config.config_mut::<AabbGizmoConfigGroup>().1.draw_all ^= true;
    config.config_mut::<SkinnedMeshBoundsGizmoConfigGroup>().1.draw_all ^= true;
}
```

Or load a glTF in the scene viewer and press `j` and `b`:

```sh
cargo run --example scene_viewer --features "free_camera" -- "path/to/your.gltf"
```

If you were using [`bevy_mod_skinned_aabb`](https://github.com/greeble-dev/bevy_mod_skinned_aabb), see [Bevy 0.19 and `bevy_mod_skinned_aabb`](https://github.com/greeble-dev/bevy_mod_skinned_aabb/blob/main/notes/bevy_0_19.md) for migration notes.
