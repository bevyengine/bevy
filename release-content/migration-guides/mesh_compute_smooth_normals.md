---
title: Smooth normals implementation changed
pull_requests: [18552]
---

In Bevy 0.16, `Mesh` smooth normal calculation used a triangle area-weighted
algorithm. In 0.17, the area-weighted algorithm was moved to separate methods,
the default implementation was switched to a corner angle-weighted algorithm,
and `Mesh::compute_custom_smooth_normals` was added for other cases.

The angle-weighted method is more suitable for growing or shrinking a mesh along
its vertex normals, such as when generating an outline mesh. It also results in
more expected lighting behavior for some meshes. In most cases, the difference
will be small and no change is needed. However, the new default is somewhat
slower, and does not always produce the result desired by an artist. If you
preferred the lighting in 0.16, or have a significant performance regression,
or needed area-weighted normals for any other reason, you can switch to the
new dedicated area-weighted methods.

```diff
// Only if the new smooth normals algorithm is unsatisfactory:

let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default())
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
-    .with_computed_smooth_normals();
+    .with_computed_area_weighted_normals;

- mesh.compute_smooth_normals();
+ mesh.compute_area_weighted_normals();
```

As part of this change, the helper functions `face_normal` and
`face_area_normal`, were renamed to `triangle_normal` and `triangle_area_normal`
respectively to better reflect the fact that they do not take an entire
geometric face into account.

```diff
- use bevy::render::mesh::face_normal;
+ use bevy::render::mesh::triangle_normal;
- let normal = face_normal(a, b, c);
+ let normal = triangle_normal(a, b, c);

- use bevy::render::mesh::face_area_normal;
+ use bevy::render::mesh::triangle_area_normal;
- let normal = face_area_normal(a, b, c);
+ let normal = triangle_area_normal(a, b, c);
```
