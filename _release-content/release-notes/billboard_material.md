---
title: Billboard Material
authors: ["@barsoosayque"]
pull_requests: [26411]
---

Billboards are meshes that always face the camera.
They're a classic way to bring 2D art into a 3D scene, such as character sprites, trees in the distance, particles, or icons above a character's head.

Bevy now has a built-in `BillboardMaterial`.
It's an extension over `StandardMaterial` using a `BillboardExtension`, so billboards are lit, shadowed and textured like any other mesh:

```rust
commands.spawn((
    Mesh3d(meshes.add(Rectangle::new(1.0, 1.0))),
    MeshMaterial3d(billboard_materials.add(BillboardMaterial {
        base: StandardMaterial {
            base_color_texture: Some(asset_server.load("character.png")),
            alpha_mode: AlphaMode::Mask(0.5),
            ..default()
        },
        extension: BillboardExtension {
            lock_y: true,
            ..default()
        },
    })),
    Transform::from_xyz(0.0, 1.0, 0.0),
));
```

`BillboardExtension` has two settings:
- `lock_y`: keeps the billboard upright, rotating it only around the world Y axis.
- `depth_test` can be turned off to draw the billboard over the geometry in front of it.

The rotation happens in the vertex shader, so billboards need no per-frame `Transform` updates, and the ones that share a mesh and settings are drawn together.

Frustum culling bounds are automatically calculated using meshes `Aabb`, scaled into a cube.
You can disable this and use your own frustum culling or `Aabb` by using `NoFrustumCulling`/`NoAutoAabb`.

This is initial implementation of the feature, and it has few limitations for the time being:
- Billboards don't cast shadows (but receive shadows fine);
- Skinned or morphed meshes aren't supported;
- Picking and raycasts don't work well yet either, because they test against the unrotated mesh.

