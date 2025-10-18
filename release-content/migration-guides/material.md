---
title: "`bevy_material`"
pull_requests: [21543]
---

Materials can now be defined without a renderer, with the new `bevy_material` crate.

As such, the following moves have occurred:

Move from `bevy_render` to `bevy_material`:

- `AlphaMode`
- `ShaderLabel`, `DrawFunctionLabel`, wgpu exports from `render_resource`, `bind_group_layout_entries`
- `DrawFunctionId`
- `SpecializedMeshPipelineError`
- `BindGroupLayoutDescriptor`, `RenderPipelineDescriptor`, `NoFragmentStateError`, `VertexState`, `FragmentState`, `ComputePipelineDescriptor`

Move from `bevy_pbr` to `bevy_material`:

- `Opaque`
- `MESH_PIPELINE_VIEW_LAYOUT_SAFE_MAX_TEXTURES`, `MeshPipelineKey`
- `MeshPipeline`
- `MATERIAL_BIND_GROUP_INDEX`, `ErasedMaterialPipelineKey`, `ErasedMaterialKey`, `ErasedMaterialKeyVTable`, `RenderPhaseType`
- `MeshPipelineViewLayout`, `MeshPipelineViewLayoutKey`, `MeshPipelineViewLayouts`
- `MeshLayouts`
- `MaterialProperties` and `MaterialPipeline`

Move from `bevy_pbr` to `bevy_render`:

- `LIGHTMAPS_PER_SLAB`, `Lightmap`, `RenderLightmap`, `RenderLightmaps`, `LightmapSlab`, `AllocatedLightmap`, `LightmapSlabIndex`, `LightmapSlotIndex`
- `MaterialBindingId`, `MaterialBindGroupIndex`, `MaterialBindGroupSlot`
- `MeshTransforms`, `MeshUniform`, `MeshInputUniform`, `MeshFlags`, `RenderMeshInstanceFlags`, `RenderMeshInstanceCpu`, `RenderMeshInstanceGpu`, `PreviousGlobalTransform`, `RenderMeshInstanceShared`, `remove_mesh_input_uniform`
- `MAX_JOINTS`, `MAX_TOTAL_JOINTS`, `JOINTS_PER_ALLOCATION_UNIT`, `JOINT_EXTRACTION_THRESHOLD_FACTOR`, `SkinByteOffset`, `SkinUniforms`, `SkinUniformInfo`
- `RenderMeshInstances`, `RenderMeshInstancesCpu`, `RenderMeshInstancesGpu`, `RenderMeshQueueData`
- `TONEMAPPING_LUT_TEXTURE_BINDING_INDEX`, `TONEMAPPING_LUT_SAMPLER_BINDING_INDEX`, `IRRADIANCE_VOLUMES_ARE_USABLE`

`CORE_3D_DEPTH_FORMAT` from `bevy_core_pipeline` to `bevy_render`

`Into<MeshPipelineView_LayoutKey>` has been removed, use `mesh_pipeline_view_layout_key_from_msaa()` (for `Msaa`) and `mesh_pipeline_view_layout_key_from_view_prepass_textures()` (for `ViewPrepassTextures`) instead
