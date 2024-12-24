// Types needed for GPU mesh uniform building.

#define_import_path bevy_pbr::mesh_preprocess_types

// Per-frame data that the CPU supplies to the GPU.
struct MeshInput {
    // The model transform.
    world_from_local: mat3x4<f32>,
    // The lightmap UV rect, packed into 64 bits.
    lightmap_uv_rect: vec2<u32>,
    // A set of bitflags corresponding to `MeshFlags` on the Rust side. See the
    // `MESH_FLAGS_` flags in `mesh_types.wgsl` for a list of these.
    flags: u32,
    // The index of this mesh's `MeshInput` in the `previous_input` array, if
    // applicable. If not present, this is `u32::MAX`.
    previous_input_index: u32,
    // The index of the first vertex in the vertex slab.
    first_vertex_index: u32,
    // The index of the first vertex index in the index slab.
    //
    // If this mesh isn't indexed, this value is ignored.
    first_index_index: u32,
    // For indexed meshes, the number of indices that this mesh has; for
    // non-indexed meshes, the number of vertices that this mesh consists of.
    index_count: u32,
    current_skin_index: u32,
    previous_skin_index: u32,
    // Low 16 bits: index of the material inside the bind group data.
    // High 16 bits: index of the lightmap in the binding array.
    material_and_lightmap_bind_group_slot: u32,
}

// The `wgpu` indirect parameters structure for indexed meshes.
//
// The `build_indirect_params.wgsl` shader generates these.
struct IndirectParametersIndexed {
    // The number of indices that this mesh has.
    index_count: u32,
    // The number of instances we are to draw.
    instance_count: u32,
    // The offset of the first index for this mesh in the index buffer slab.
    first_index: u32,
    // The offset of the first vertex for this mesh in the vertex buffer slab.
    base_vertex: u32,
    // The index of the first mesh instance in the `Mesh` buffer.
    first_instance: u32,
}

// The `wgpu` indirect parameters structure for non-indexed meshes.
//
// The `build_indirect_params.wgsl` shader generates these.
struct IndirectParametersNonIndexed {
    // The number of vertices that this mesh has.
    vertex_count: u32,
    // The number of instances we are to draw.
    instance_count: u32,
    // The offset of the first vertex for this mesh in the vertex buffer slab.
    base_vertex: u32,
    // The index of the first mesh instance in the `Mesh` buffer.
    first_instance: u32,
}

// Information needed to generate the `IndirectParametersIndexed` and
// `IndirectParametersNonIndexed` draw commands.
struct IndirectParametersMetadata {
    // The index of the mesh in the `MeshInput` buffer.
    mesh_index: u32,
    // The index of the first instance corresponding to this batch in the `Mesh`
    // buffer.
    base_output_index: u32,
    // The index of the batch set in the `IndirectBatchSet` buffer.
    batch_set_index: u32,
    // The number of instances that are to be drawn.
    //
    // The `mesh_preprocess.wgsl` shader determines this, and the
    // `build_indirect_params.wgsl` shader copies this value into the indirect
    // draw command.
    instance_count: atomic<u32>,
}

// Information about each batch set.
//
// A *batch set* is a set of meshes that might be multi-drawn together.
//
// The CPU creates this structure, and the `build_indirect_params.wgsl` shader
// modifies it. If `multi_draw_indirect_count` is in use, the GPU reads this
// value when multi-drawing a batch set in order to determine how many commands
// make up the batch set.
struct IndirectBatchSet {
    // The number of commands that make up this batch set.
    //
    // The CPU initializes this value to zero. The `build_indirect_params.wgsl`
    // shader increments this value as it processes batches.
    indirect_parameters_count: atomic<u32>,
    // The offset of the first batch corresponding to this batch set within the
    // `IndirectParametersIndexed` or `IndirectParametersNonIndexed` arrays.
    indirect_parameters_base: u32,
}
