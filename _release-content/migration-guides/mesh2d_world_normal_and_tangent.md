---
title: "`Mesh2d` world normal and tangent computation"
pull_requests: [25369]
---

`mesh2d_tangent_local_to_world` in `bevy_sprite_render::mesh2d::functions` now takes the
instance index as a third argument, so that it can read the mesh flags to correct the
tangent's sign for mirrored transforms.

It also now normalizes the world tangent, and `mesh2d_normal_local_to_world` now normalizes
the world normal. Previously neither was normalized, so a scaled `Transform` produced a
`world_normal` scaled by the inverse of the scale and a `world_tangent` scaled by the scale.
This matches the 3D behavior in `bevy_pbr`.

If you call `mesh2d_tangent_local_to_world` from a custom 2D vertex shader, pass the vertex's
instance index:

```wgsl
// Before
out.world_tangent = mesh_functions::mesh2d_tangent_local_to_world(
    world_from_local,
    vertex.tangent
);

// After
out.world_tangent = mesh_functions::mesh2d_tangent_local_to_world(
    world_from_local,
    vertex.tangent,
    vertex.instance_index
);
```

If you were compensating for the missing normalization by normalizing `world_normal` or
`world_tangent` yourself in a fragment shader, that is now redundant but harmless.

`MeshFlags::SIGN_DETERMINANT_MODEL_3X3` is new, but you do not need to set it. It is derived
from the transform in `Mesh2dUniform::from_components`, so custom extraction systems that
build `Mesh2dTransforms` with `MeshFlags::empty()` keep working unchanged.
