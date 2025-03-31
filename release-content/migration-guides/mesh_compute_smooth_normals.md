---
title: Smooth normals implementation changed
pull_requests: [18552]
---

In Bevy 0.16, `Mesh` smooth normal calculation used a face area-weighted
algorithm. In 0.17, the area-weighted method was moved to separate methods,
the default implementation was switched to a corner angle-weighted algorithm,
and `Mesh::compute_custom_smooth_normals` was added for other cases.

The angle-weighted method is more suitable for growing or shrinking a mesh along
its vertex normals, such as for generating an outline mesh. It also results in
more expected lighting behavior in many cases. In most cases, the difference
will be small and no change is needed. However, the new default is somewhat
slower, and does not always produce the result desired by an artist. If you
preferred the lighting in 0.16, have a significant performance regression,
or needed face-weighted normals for any other reason, you can switch to the
new dedicated face-weighted methods.

```diff
// Only if the new smooth normals algorithm is unsatisfactory:

let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default())
-    .with_computed_smooth_normals();
+    .with_computed_face_weighted_normals;

- mesh.compute_smooth_normals();
+ mesh.compute_face_weighted_normals;
```
