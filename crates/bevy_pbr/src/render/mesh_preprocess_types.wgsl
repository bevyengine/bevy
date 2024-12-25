// Types needed for GPU mesh uniform building.

#define_import_path bevy_pbr::mesh_preprocess_types

// The `wgpu` indirect parameters structure. This is a union of two structures.
// For more information, see the corresponding comment in
// `gpu_preprocessing.rs`.
struct IndirectParameters {
    // `vertex_count` or `index_count`.
    vertex_count_or_index_count: u32,
    // `instance_count` in both structures.
    instance_count: atomic<u32>,
    // `first_vertex` or `first_index`.
    first_vertex_or_first_index: u32,
    // `base_vertex` or `first_instance`.
    base_vertex_or_first_instance: u32,
    // A read-only copy of `instance_index`.
    first_instance: u32,
}
