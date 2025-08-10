---
title: `MeshMorphWeights` is now a reference, and `MorphPlugin` has been removed
pull_requests: [18465]
---

Some morph target components have been changed to improve performance. Users who
let the glTF loader set up their components do not need to make any changes.
Users who set up their morph target components manually or modify the glTF
loader's components may need to make changes.

`MeshMorphWeights` is now a reference to an entity with a `MorphWeights`
component. Previously it contained a copy of the weights.

```diff
- struct MeshMorphWeights { weights: Vec<f32> }
+ struct MeshMorphWeights(Entity);
```

See the `MorphWeights` documentation for examples of how to set up morph targets
with the new structure.

`MorphPlugin` has been removed as it was no longer necessary. Users who added
this plugin manually can remove it.

```diff
- app.add_plugins(MorphPlugin);
```
