---
title: Create `bevy_render_2d` crate
pull_requests: [18467]
---

## Goal

Extract from `bevy_sprite` code that is not exclusive to sprites and move it to
new crate `bevy_render_2d`. New locations for symbols are as follows:

## Relocations

### Structs

Struct | `0.16` Path | `0.17` Path
--- | --- | ---
`MeshMaterial2d` | | `bevy_render_2d::material`
`AlphaMode2d` | | `bevy_render_2d::material`
`Material2dKey` | | `bevy_render_2d::material::key`
`DrawMesh2d` | | `bevy_render_2d::mesh_pipeline::commands`
`SetMesh2dBindGroup` | | `bevy_render_2d::mesh_pipeline::commands`
`SetMesh2dViewBindGroup` | | `bevy_render_2d::mesh_pipeline::commands`
`Mesh2dPipeline` | | `bevy_render_2d::mesh_pipeline::pipeline`
`Mesh2dPipelineKey` | | `bevy_render_2d::mesh_pipeline::key`
`Material2dBindGroupId` | | `bevy_render_2d::mesh_pipeline::render`
`Mesh2dBindGroup` | | `bevy_render_2d::mesh_pipeline::render`
`Mesh2dTransforms` | | `bevy_render_2d::mesh_pipeline::render`
`Mesh2dUniform` | | `bevy_render_2d::mesh_pipeline::render`
`Mesh2dViewBindGroup` | | `bevy_render_2d::mesh_pipeline::render`
`Material2dBindGroupId` | | `bevy_render_2d::mesh_pipeline::render`
`MeshFlags` | | `bevy_render_2d::mesh_pipeline::render`
`RenderMesh2dInstance` | | `bevy_render_2d::mesh_pipeline::render`
`RenderMesh2dInstances` | | `bevy_render_2d::mesh_pipeline::render`
`ViewKeyCache` | | `bevy_render_2d::mesh_pipeline::render`
`ViewSpecializationTicks` | | `bevy_render_2d::mesh_pipeline::render`

### Traits

Trait | `0.16` Path | `0.17` Path
--- | --- | ---
`Material2d` | | `bevy_render_2d::material`

### Plugins

Trait | `0.16` Path | `0.17` Path
--- | --- | ---
`Material2dPlugin` | | `bevy_render_2d::material::plugin`
`Mesh2dRenderPlugin` | | `bevy_render_2d::mesh_pipeline`

### Methods

Method  | `0.16` Path | `0.17` Path
--- | --- | ---
tonemapping_pipeline_key | | `bevy_render_2d::mesh_pipeline::key`

## Prelude

`bevy_render_2d`'s prelude contains:
* `Material2d`
* `MeshMaterial2d`
* `AlphaMode2d`
