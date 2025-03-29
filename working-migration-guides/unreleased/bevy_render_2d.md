# Create `bevy_render_2d` crates

prs = [18467]

This is part of 18423 tracking issue

* `Mesh2dPlugin`, `Material2d`, `Material2dKey`, `MeshMaterial2d`, and `AlphaMode2d` are now located in `bevy_render_2d::material`
* `Mesh2dRenderPlugin`, `Material2dBindGroupId`, `Mesh2dBindGroup`, `Mesh2dViewBindGroup`, `DrawMesh2d`,
    `SetMesh2dBindGroup`, `SetMesh2dViewBindGroup`, `RenderMesh2dInstance`, `RenderMesh2dInstances`,
    `Mesh2dTransforms`, `MeshFlags`, `Mesh2dPipeline`, `Mesh2dPipelineKey`, `ViewCache`, and `ViewSpecializationTicks`
    are now located in `bevy_render_2d::mesh_pipeline` or one of it's submodules
* `bevy_render_2d`'s prelude contains `Material2d`, `MeshMaterial2d`, and `AlphaMode2d`
