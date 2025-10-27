---
title: Render Assets diagnostics
authors: ["@hukasu"]
pull_requests: [19311]
---

## Goals

Create diagnostics plugins `MeshAllocatorDiagnosticPlugin`, `MaterialAllocatorDiagnosticPlugin`,
`RenderAssetDiagnosticPlugin`, and `ErasedRenderAssetDiagnosticPlugin`, that collect measurements
related to `MeshAllocator`s, `MaterialBindGroupAllocator`, `RenderAssets`, and `ErasedRenderAssets`
respectively.

`MeshAllocatorDiagnosticPlugin` and `MaterialDiagnosticPlugin` measure the number of slabs, the total size of memory
allocated by the slabs, and the number of objects allocated in the slabs. Only bindless materials use slabs for their
allocations, non-bindless materials return 0 for all of them.

`RenderAssetDiagnosticsPlugin<RA>` and `ErasedAssetDiagnosticsPlugin<ERA>` measure the number of
assets in `RenderAssets<RA>` and `ErasedRenderAssets<ERA::ErasedAsset>`. `ErasedAssetDiagnosticsPlugin<ERA>`
will report the same number of assets for all `ERA` that share the same `ERA::ErasedAsset`.

## Showcase

```rust
app.add_plugins(DefaultPlugins)
    .add_plugins((
        MeshAllocatorDiagnosticPlugin,
        MaterialAllocatorDiagnosticPlugin::<StandardMaterial>::default(),
        RenderAssetDiagnosticPlugin::<RenderMesh>::new(" render meshes"),
        RenderAssetDiagnosticPlugin::<GpuImage>::new(" gpu images"),
        // ImageMaterial is the name of the manual material used on the `manual_material` example
        ErasedRenderAssetDiagnosticPlugin::<ImageMaterial>::new(" image materials"),
    ));
```

If you also have `LogDiagnosticsPlugin`, the output looks something like this:

```ignore
INFO bevy_diagnostic: mesh_allocator_allocations                                             :    4.000000 meshes (avg 4.000000 meshes)
INFO bevy_diagnostic: mesh_allocator_slabs                                                   :    4.000000 slabs (avg 4.000000 slabs)
INFO bevy_diagnostic: mesh_allocator_slabs_size                                              : 4194360.000000 bytes (avg 4194360.000000 bytes)
INFO bevy_diagnostic: material_allocator_allocations/bevy_pbr::pbr_material::StandardMaterial:   14.000000 materials (avg 14.000000 materials)
INFO bevy_diagnostic: material_allocator_slabs/bevy_pbr::pbr_material::StandardMaterial      :    1.000000 slabs (avg 1.000000 slabs)
INFO bevy_diagnostic: material_allocator_slabs_size/bevy_pbr::pbr_material::StandardMaterial :  576.000000 bytes (avg 576.000000 bytes)
INFO bevy_diagnostic: render_asset/bevy_render::mesh::RenderMesh                             :    5.000000 render meshes (avg 5.000000 render meshes)
INFO bevy_diagnostic: render_asset/bevy_render::texture::gpu_image::GpuImage                 :   10.000000 gpu images (avg 10.000000 gpu images)
INFO bevy_diagnostic: erased_render_asset/manual_material::ImageMaterial                     :    2.000000 image materials (avg 2.000000 image materials)
```
