---
title: "New crate `bevy_material`"
pull_requests: [22426]
---

Various material-related machinery where extracted from `bevy_pbr` and `bevy_render` into a new crate called `bevy_material`.

The following were moved from `bevy_render` to `bevy_material`:

- `AlphaMode`
- `SpecializedMeshPipelineError`

The following were moved from `bevy_pbr` to `bevy_material`:

- `OpaqueRendererMethod`
- `ErasedMeshPipelineKey`, `ErasedMaterialPipelineKey`, `ErasedMaterialKey`, `ErasedMaterialKeyVTable`, `RenderPhaseType`
- `MaterialProperties`

The following were moved from `bevy_render` to `bevy_material` but do not require migration thanks to re-exports:

- `BindGroupLayoutDescriptor`, `RenderPipelineDescriptor`, `NoFragmentStateError`, `VertexState`, `FragmentState`, `ComputePipelineDescriptor`
- `ShaderLabel`, `DrawFunctionLabel`, `DrawFunctionId`
- `BindGroupLayoutEntryBuilder`, `BindGroupLayoutEntries`, `DynamicBindGroupLayoutEntries`, `IntoBindGroupLayoutEntryBuilder`, `IntoIndexedBindGroupLayoutEntryBuilderArray`
