#define_import_path bevy_pbr::forward_io

struct UncompressedVertex {
    @builtin(instance_index) instance_index: u32,
#ifdef VERTEX_POSITIONS
    @location(0) position: vec3<f32>,
#endif
#ifdef VERTEX_NORMALS
    @location(1) normal: vec3<f32>,
#endif
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_UVS_B
    @location(3) uv_b: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(4) tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(5) color: vec4<f32>,
#endif
#ifdef SKINNED
    @location(6) joint_indices: vec4<u32>,
    @location(7) joint_weights: vec4<f32>,
#endif
#ifdef MORPH_TARGETS
    @builtin(vertex_index) index: u32,
#endif
};

struct Vertex {
    @builtin(instance_index) instance_index: u32,
#ifdef VERTEX_POSITIONS
#ifdef VERTEX_POSITIONS_COMPRESSED
    @location(0) compressed_position: vec4<f32>,
#else
    @location(0) position: vec3<f32>,
#endif
#endif
#ifdef VERTEX_NORMALS
#ifdef VERTEX_NORMALS_COMPRESSED
    @location(1) compressed_normal: vec2<f32>,
#else
    @location(1) normal: vec3<f32>,
#endif
#endif
#ifdef VERTEX_UVS_A
#ifdef VERTEX_UVS_A_COMPRESSED
    @location(2) compressed_uv: vec2<f32>,
#else
    @location(2) uv: vec2<f32>,
#endif
#endif
#ifdef VERTEX_UVS_B
#ifdef VERTEX_UVS_B_COMPRESSED
    @location(3) compressed_uv_b: vec2<f32>,
#else
    @location(3) uv_b: vec2<f32>,
#endif
#endif
#ifdef VERTEX_TANGENTS
#ifdef VERTEX_TANGENTS_COMPRESSED
    @location(4) compressed_tangent: vec2<f32>,
#else
    @location(4) tangent: vec4<f32>,
#endif
#endif
#ifdef VERTEX_COLORS
    @location(5) color: vec4<f32>,
#endif
#ifdef SKINNED
    @location(6) joint_indices: vec4<u32>,
    @location(7) joint_weights: vec4<f32>,
#endif
#ifdef MORPH_TARGETS
    @builtin(vertex_index) index: u32,
#endif
};

// The instance_index parameter must match vertex_in.instance_index. This is a work around for a wgpu dx12 bug.
// See https://github.com/gfx-rs/naga/issues/2416
fn decompress_vertex(vertex_in: Vertex, instance_index: u32) -> UncompressedVertex {
    let mesh_metadata = bevy_pbr::mesh_functions::get_metadata(instance_index);
    var uncompressed_vertex: UncompressedVertex;
    uncompressed_vertex.instance_index = instance_index;
#ifdef VERTEX_POSITIONS
#ifdef VERTEX_POSITIONS_COMPRESSED
    uncompressed_vertex.position = bevy_render::utils::decompress_vertex_position(vertex_in.compressed_position, mesh_metadata.aabb_center, mesh_metadata.aabb_half_extents);
#else
    uncompressed_vertex.position = vertex_in.position;
#endif
#endif
#ifdef VERTEX_NORMALS
#ifdef VERTEX_NORMALS_COMPRESSED
    uncompressed_vertex.normal = bevy_render::utils::decompress_vertex_normal(vertex_in.compressed_normal);
#else
    uncompressed_vertex.normal = vertex_in.normal;
#endif
#endif
#ifdef VERTEX_UVS_A
#ifdef VERTEX_UVS_A_COMPRESSED
    let uv_min_and_extents_a = mesh_metadata.uv_channels_min_and_extents[0];
    uncompressed_vertex.uv = bevy_render::utils::decompress_vertex_uv(vertex_in.compressed_uv, uv_min_and_extents_a);
#else
    uncompressed_vertex.uv = vertex_in.uv;
#endif
#endif
#ifdef VERTEX_UVS_B
#ifdef VERTEX_UVS_B_COMPRESSED
    let uv_min_and_extents_b = mesh_metadata.uv_channels_min_and_extents[1];
    uncompressed_vertex.uv_b = bevy_render::utils::decompress_vertex_uv(vertex_in.compressed_uv_b, uv_min_and_extents_b);
#else
    uncompressed_vertex.uv_b = vertex_in.uv_b;
#endif
#endif
#ifdef VERTEX_TANGENTS
#ifdef VERTEX_TANGENTS_COMPRESSED
    uncompressed_vertex.tangent = bevy_render::utils::decompress_vertex_tangent(vertex_in.compressed_tangent);
#else
    uncompressed_vertex.tangent = vertex_in.tangent;
#endif
#endif
#ifdef VERTEX_COLORS
    uncompressed_vertex.color = vertex_in.color;
#endif
#ifdef SKINNED
    uncompressed_vertex.joint_indices = vertex_in.joint_indices;
    uncompressed_vertex.joint_weights = vertex_in.joint_weights;
#endif
#ifdef MORPH_TARGETS
    uncompressed_vertex.index = vertex_in.index;
#endif
    return uncompressed_vertex;
}

struct VertexOutput {
    // This is `clip position` when the struct is used as a vertex stage output
    // and `frag coord` when used as a fragment stage input
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_UVS_B
    @location(3) uv_b: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(4) world_tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(5) color: vec4<f32>,
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(6) @interpolate(flat) instance_index: u32,
#endif
#ifdef VISIBILITY_RANGE_DITHER
    @location(7) @interpolate(flat) visibility_range_dither: i32,
#endif
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
}
