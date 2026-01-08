---
title: Automatic `Aabb` updates for sprites and meshes
pull_requests: [18742]
---

Bevy automatically creates an `Aabb` component for entities containing a mesh
or sprite - the `Aabb` is then used for visibility and picking.

In 0.17 the `Aabb` [was not updated](https://github.com/bevyengine/bevy/issues/4294)
if the mesh or sprite was modified. This has been fixed in 0.18. If you were working around the issue by manually updating or removing the
`Aabb`, then the workaround is no longer needed.

```rust
// 0.17: Modify the mesh, and remove the `Aabb` so that it's automatically
// recreated from the modified mesh.
mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, new_positions);
entity.remove::<Aabb>();

// 0.18: The `Aabb` will be automatically updated.
mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, new_positions);
```

For users who want more control, 0.18 also adds a `NoAutoAabb` component. This
will disable both automatic creation and automatic update of `Aabb` components.

```rust
entity.insert(NoAutoAabb);
```
