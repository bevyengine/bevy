// Types needed for GPU mesh uniform building.

#define_import_path bevy_pbr::mesh_preprocess_types

// Per-frame data that the CPU supplies to the GPU.
struct MeshInput {
    // The model transform.
    world_from_local: mat3x4<f32>,
    // The lightmap UV rect, packed into 64 bits.
    lightmap_uv_rect: vec2<u32>,
    // Various flags.
    flags: u32,
    previous_input_index: u32,
    first_vertex_index: u32,
    first_index_index: u32,
    index_count: u32,
    current_skin_index: u32,
    // Low 16 bits: index of the material inside the bind group data.
    // High 16 bits: index of the lightmap in the binding array.
    material_and_lightmap_bind_group_slot: u32,
    timestamp: u32,
    // User supplied index to identify the mesh instance
    tag: u32,
    // The index of the morph descriptor for this mesh instance in the
    // `morph_descriptors` table.
    //
    // If the mesh has no morph targets, this is `u32::MAX`.
    morph_descriptor_index: u32,
}

// The `wgpu` indirect parameters structure. This is a union of two structures.
// For more information, see the corresponding comment in
// `gpu_preprocessing.rs`.
struct IndirectParametersIndexed {
    // `vertex_count` or `index_count`.
    index_count: u32,
    // `instance_count` in both structures.
    instance_count: u32,
    // `first_vertex` or `first_index`.
    first_index: u32,
    // `base_vertex` or `first_instance`.
    base_vertex: u32,
    // A read-only copy of `instance_index`.
    first_instance: u32,
}

struct IndirectParametersNonIndexed {
    vertex_count: u32,
    instance_count: u32,
    base_vertex: u32,
    first_instance: u32,
}

// Information needed to construct indirect draw parameters for a single draw.
//
// Note that is per-*draw* (i.e. per-mesh), not per-mesh-instance or
// per-batch-set. A single multi-draw indirect call can perform multiple draws.
//
// Typically, the uniform allocation and mesh preprocessing phases fill in this
// structure. However, parts of it may be filled in on the CPU for objects that
// aren't multidrawn.
struct IndirectParametersMetadata {
    // The index of the first `MeshUniform` for this draw in the mesh uniform
    // buffer.
    //
    // `MeshUniform`s for all instances are stored consecutively.
    //
    // This is filled in in the `allocate_uniforms` shader, or on the CPU when
    // multidraw isn't in use.
    base_output_index: u32,

    // The index of this batch set in the `IndirectBatchSet` array.
    //
    // This is filled in in the `allocate_uniforms` shader, or on the CPU when
    // multidraw isn't in use.
    batch_set_index: u32,

    // The index of the mesh in the `MeshInput` buffer.
    //
    // The mesh preprocessing shader fills this in.
    mesh_index: u32,

#ifdef WRITE_INDIRECT_PARAMETERS_METADATA
    // The number of instances that were visible last frame (if occlusion
    // culling is in use) or that were visible at all (if occlusion culling
    // isn't in use).
    early_instance_count: atomic<u32>,
    // The number of instances that were visible this frame if occlusion culling
    // is in use.
    late_instance_count: atomic<u32>,
#else   // WRITE_INDIRECT_PARAMETERS_METADATA
    // The number of instances that were visible last frame (if occlusion
    // culling is in use) or that were visible at all (if occlusion culling
    // isn't in use).
    early_instance_count: u32,
    // The number of instances that were visible this frame if occlusion culling
    // is in use.
    late_instance_count: u32,
#endif  // WRITE_INDIRECT_PARAMETERS_METADATA
}

struct IndirectBatchSet {
    indirect_parameters_count: atomic<u32>,
    indirect_parameters_base: u32,
}

// One invocation of this compute shader: i.e. one mesh instance in a view.
struct PreprocessWorkItem {
    // The index of the `MeshInput` in the `current_input` buffer that we read
    // from.
    input_index: u32,
    // In direct mode, the index of the `Mesh` in `output` that we write to. In
    // indirect mode, the index of the `IndirectParameters` in
    // `indirect_parameters` that we write to.
    output_or_indirect_parameters_index: u32,
}

// Information about each bin in a batch set.
//
// This is maintained by the CPU and cached for bins that don't change from
// frame to frame.
struct BinMetadata {
    // The index of the indirect parameters for this bin, relative to the first
    // indirect parameter index for the batch set.
    //
    // That is, the final indirect parameters index for this bin is
    // `first_indirect_parameters_index` in the `UniformAllocationMetadata` plus
    // this value.
    indirect_parameters_offset: u32,

    // The index of the bin that this metadata corresponds to.
    //
    // The GPU doesn't use this, but the CPU does in order to perform the
    // reverse mapping from bin metadata index back to the bin. We could store
    // this in a non-GPU-accessible buffer, but I figured the extra complexity
    // wasn't worth it.
    bin_index: u32,

    // The number of mesh instances in this bin.
    instance_count: u32,
};
