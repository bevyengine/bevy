---
title: "`Mesh3dVisibility` is the visibility class of 3D meshes"
pull_requests: []
---

3D mesh draws are grouped under the `Mesh3dVisibility` visibility class instead of `Mesh3d`, so that GPU-authored mesh draws without a `Mesh3d` share the same render phases. `Mesh3d` requires `Mesh3dVisibility`, so mesh entities are unaffected.

Code that looks up visible meshes by class must use the new key:

- `visible_entities.get::<Mesh3d>()` becomes `visible_entities.get::<Mesh3dVisibility>()`.
- `TypeId::of::<Mesh3d>()` used with `VisibilityClass` or `RenderVisibleEntities` becomes `TypeId::of::<Mesh3dVisibility>()`.

Components that draw as 3D meshes without a `Mesh3d` should require `Mesh3dVisibility` instead of pushing the `Mesh3d` class in an `on_add` hook.
