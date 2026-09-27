---
title: "Solari `Mesh::enable_raytracing` is now `Mesh::raytracing`"
pull_requests: [25951]
---

`Mesh::enable_raytracing: bool` has been replaced by `Mesh::raytracing: MeshRaytracingFlags`, which declares which BLAS (if any) `bevy_solari` builds for the mesh.

`MeshRaytracingFlags::OPAQUE` is the new default, replacing the old default of `true`.

```rust
// 0.20
mesh1.enable_raytracing = false;
mesh2.enable_raytracing = true;

// 0.21
mesh1.raytracing = MeshRaytracingFlags::empty();
mesh2.raytracing = MeshRaytracingFlags::OPAQUE; // or NON_OPAQUE
```
