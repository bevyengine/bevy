---
title: "`bevy_material`"
pull_requests: [21543]
---

TODO: This is just a commit log, need to clean up

The following have moved:

- initial bevy_material
- Opaque move
- Move ShaderLabel, DrawFunctionLabel, wgpu exports from render_resource, bind_group_layout_entries
- Move DrawFunctionId
- Move SpecializedMeshPipelineError
- Move BindGroupLayoutDescriptor, RenderPipelineDescriptor, NoFragmentStateError, VertexState, FragmentState, ComputePipelineDescriptor
- Move MESH_PIPELINE_VIEW_LAYOUT_SAFE_MAX_TEXTURES, MeshPipelineKey
- Move MATERIAL_BIND_GROUP_INDEX, ErasedMaterialPipelineKey, ErasedMaterialKey, ErasedMaterialKeyVTable, RenderPhaseType
- Convert MeshPipelineViewLayouts and MeshPipeline to RenderStartup system
- mesh_pipeline_view_layout_key_from_msaa and mesh_pipeline_view_layout_key_from_view_prepass_textures
- Move MeshPipelineViewLayout, MeshPipelineViewLayoutKey, MeshPipelineViewLayouts
- Make impl MeshLayouts a trait
- Move MeshLayouts
- Deprecate get_image_texture for MeshPipeline and Mesh2dPipeline, change dummy to Image in MeshPipeline
- Move most of lightmap to bevy_render
- Move MaterialBindingId, MaterialBindGroupIndex, MaterialBindGroupSlot to bevy_render
- Move MeshTransforms, MeshUniform, MeshInputUniform, MeshFlags, RenderMeshInstanceFlags, RenderMeshInstanceCpu, RenderMeshInstanceGpu, PreviousGlobalTransform, RenderMeshInstanceShared, remove_mesh_input_uniform to bevy_render
- Move MAX_JOINTS, MAX_TOTAL_JOINTS, JOINTS_PER_ALLOCATION_UNIT, JOINT_EXTRACTION_THRESHOLD_FACTOR, SkinByteOffset, SkinUniforms, SkinUniformInfo to bevy_render
- Move util consts
- Move RenderMeshInstances, RenderMeshInstancesCpu, RenderMeshInstancesGpu, RenderMeshQueueData to bevy_render
- Move MeshPipeline to bevy_material, and its render impls to bevy_render
- Move MaterialProperties and MaterialPipeline to bevy_material
- Insert DummyImage resource
