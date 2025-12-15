---
title: Automatic `Aabb` updates for sprites and meshes
pull_requests: [18742]
---

Previously, the `Aabb` component that was automatically assigned to mesh and
sprite entities would only be calculated once. This meant the `Aabb`
[could be wrong](https://github.com/bevyengine/bevy/issues/4294) if the asset
was later modified, or the entity was changed to use a different asset. Now, the
`Aabb` component is automatically updated.

If you were manually updating the `Aabb` or removing it to trigger an update,
this is no longer needed.

```diff
 // Update vertex positions.
 mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

-// Trigger an AABB update.
-entity.remove::<Aabb>();
```
