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
    pad: u32,
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

struct IndirectParametersCpuMetadata {
    base_output_index: u32,
    batch_set_index: u32,
}

struct IndirectParametersGpuMetadata {
    mesh_index: u32,
#ifdef WRITE_INDIRECT_PARAMETERS_METADATA
    early_instance_count: atomic<u32>,
    late_instance_count: atomic<u32>,
#else   // WRITE_INDIRECT_PARAMETERS_METADATA
    early_instance_count: u32,
    late_instance_count: u32,
#endif  // WRITE_INDIRECT_PARAMETERS_METADATA
}

struct IndirectBatchSet {
    indirect_parameters_count: atomic<u32>,
    indirect_parameters_base: u32,
}
