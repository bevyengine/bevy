# Create `bevy_render_2d` and `bevy_color_material` crates

prs = [18467]

This is part of 18423 tracking issue

* `Mesh2dRenderPlugin`, `Material2dPlugin`, and `Wireframe2dPlugin` are re-exported
    in `bevy_render_2d`
* `Mesh2dPlugin`, `Material2d`, and `MeshMaterial2d` are now located in `bevy_render_2d::material`
* `AlphaMode2d`, `Mesh2dBindGroupId`, and `Mesh2dKey` are now located in `bevy_render_2d::material::rendering`
* `Mesh2dRenderPlugin`, `Mesh2dPipeline`, `Mesh2dPipelineKey`, `DrawMesh2d`
    `Mesh2dTransforms`, `RenderMesh2dInstance`, `RenderMesh2dInstances`, `SetMesh2dBindGroup`, `SetMesh2dViewBindGroup`
    `ViewCache`, `ViewSpecializationTicks` are now located in `bevy_render_2d::mesh`
* `Wireframe2dPlugin`, `NoWireframe2d`, `Wireframe2d`, `Wireframe2dColor`, `Wireframe2dConfig`
    are now located in `bevy_render_2d::material`
* `ColorMaterialPlugin` and `ColorMaterial` are now located in `bevy_color_material`