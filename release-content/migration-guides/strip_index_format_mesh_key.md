---
title: Mesh pipeline key requires strip index format bits
pull_requests: [22188]
---

`BaseMeshPipelineKey` and `Mesh2dPipelineKey` now have `STRIP_INDEX_FORMAT_*` bits because `strip_index_format` will be required by wgpu and primitive restart is always enabled.

The strip index format bits in mesh pipeline key must match mesh index format for indexed strip topologies (For non-indexed strip topologies, the bits don't matter), and must be `STRIP_INDEX_FORMAT_NONE` for non-strip topologies. The `from_primitive_topology` method of mesh pipeline key has been changed to `from_primitive_topology_and_strip_index` to handle it and `RenderMesh` now has `index_format` method.

Before:

```rust
let key = MeshPipelineKey::from_primitive_topology(render_mesh.primitive_topology());
```

After:

```rust
let key = MeshPipelineKey::from_primitive_topology_and_strip_index(
    render_mesh.primitive_topology(),
    render_mesh.index_format(),
);
```
