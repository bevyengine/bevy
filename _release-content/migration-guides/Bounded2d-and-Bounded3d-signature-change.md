---
title: Bounded2d and Bounded3d signature change
pull_requests: [23623]
---

The signature of `Bounded2d` and `Bounded3d` has changed. All methods now
accept `Isometry2d` and `Isometry3d` objects, rather than `impl Into<IsometryXd>`
type argument.

The new signatures allow `Bounded2d` and `Bounded3d` to be dyn-compatible.

When calling `Bounded2d::aabb_2d`, `Bounded2d::bounding_circle`,
`Bounded3d::aabb_3d`, and `Bounded3d::bounding_sphere`, you may need to call
`into()` on the object sent to the function. e.g.,

```diff
-            let aabb2d = self.aabb_2d(Rot2::radians(angle));
+            let aabb2d = self.aabb_2d(Rot2::radians(angle).into());
```

When implementing these traits for an object, you will need to update the signature.
You may also be able to remove `let isometry = isometry.into();` lines.
