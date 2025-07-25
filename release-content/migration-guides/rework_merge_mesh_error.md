---
title: Rework `MergeMeshError`
pull_requests: [18561]
---

`MergeMeshError` was reworked to account for the possibility of the meshes being merged having two different `PrimitiveTopology`'s, and was renamed to `MeshMergeError` to align with the naming of other mesh errors.

- Users will need to rename `MergeMeshError` to `MeshMergeError`
- When handling `MergeMeshError` (now `MeshMergeError`), users will need to account for the new `IncompatiblePrimitiveTopology` variant, as it has been changed from a struct to an enum
- `Mesh::merge` now returns `Result<(), MeshMergeError>` instead of the previous `Result<(), MergeMeshError>`
