---
title: "`MorphWeights` and `MeshMorphWeights` have been restructured"
pull_requests: [18465]
---

Mesh morph target weights have been restructured to improve flexibility and
performance. Users who manually create `MeshMorphWeights` or `MorphWeights`
components may need to make changes.

In Bevy 0.18, entities with a `Mesh3d` component could have a `MeshMorphWeights`
component containing morph weight values. In addition, if a parent of the mesh
entity had a `MorphWeights` component then its values would be automatically
copied to the `MeshMorphWeights` component - this allowed multiple meshes to
share a single set of weight values.

In Bevy 0.19, `MeshMorphWeights` has been changed. It can now be either a set of
weight values as before, or a reference to an entity containing a `MorphWeights`
component. Referencing replaces the previous automatic copying.

```rust
// 0.18
struct MeshMorphWeights { weights: Vec<f32> }

// 0.19
enum MeshMorphWeights {
    Value { weights: Vec<f32> },
    Reference(Entity),
}
```

If you were using `MeshMorphWeights` on its own, then you just need to
use `MeshMorphWeights::Value`.

If you were using `MorphWeights` and `MeshMorphWeights` and relying on the
automatic copying, then you need to use `MeshMorphWeights::Reference` and point
it to the entity with `MorphWeights`.

```rust
// 0.18
parent_entity.insert(MorphWeights::new(...));
mesh_entity.insert((mesh, MeshMorphWeights::new(...)));

// 0.19
parent_entity.insert(MorphWeights::new(...));
mesh_entity.insert((mesh, MeshMorphWeights::Reference(parent_entity)));
```

These changes improve performance due to less copying. They also add
flexibility - a `MeshMorphWeights` component can reference a `MorphWeights`
component on any entity, not just its parent.

As a result of these changes, `MorphPlugin` was no longer needed and has been
removed.
