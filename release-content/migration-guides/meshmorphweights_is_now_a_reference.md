---
title: `MeshMorphWeights` is now a reference
pull_requests: [18465]
---

`MeshMorphWeights` is now a reference to an entity with a `MorphWeights`
component. Previously it contained a copy of the weights.

```diff
- struct MeshMorphWeights { weights: Vec<f32> }
+ struct MeshMorphWeights(Entity);
```

This change was made to improve runtime and compile-time performance. See the
`MorphWeights` documentation for examples of how to set up morph targets with
the new convention.
