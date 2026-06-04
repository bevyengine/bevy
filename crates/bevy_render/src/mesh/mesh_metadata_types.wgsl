#define_import_path bevy_render::mesh_metadata_types

struct MeshMetadata {
    // AABB for decompressing positions.
    aabb_center: vec3<f32>,
    pad_a: u32,
    aabb_half_extents: vec3<f32>,
    pad_b: u32,
    // UV channels range for decompressing UVs coordinates.
    uv_channels_min_and_extents: array<vec4<f32>, 2>,
};
