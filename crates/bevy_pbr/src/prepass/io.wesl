// Most of these attributes are not used in the default prepass fragment shader, but they are still needed so we can
// pass them to custom prepass shaders like pbr_prepass.wesl.
struct UncompressedVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,

    @if(VERTEX_UVS_A)
    @location(1) uv: vec2<f32>,

    @if(VERTEX_UVS_B)
    @location(2) uv_b: vec2<f32>,

    @if(NORMAL_PREPASS_OR_DEFERRED_PREPASS && VERTEX_NORMALS)
    @location(3) normal: vec3<f32>,
    @if(NORMAL_PREPASS_OR_DEFERRED_PREPASS && VERTEX_TANGENTS)
    @location(4) tangent: vec4<f32>,

    @if(SKINNED)
    @location(5) joint_indices: vec4<u32>,
    @if(SKINNED)
    @location(6) joint_weights: vec4<f32>,

    @if(VERTEX_COLORS)
    @location(7) color: vec4<f32>,

    @if(MORPH_TARGETS)
    @builtin(vertex_index) index: u32,
};

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @if(VERTEX_POSITIONS_COMPRESSED)
    @location(0) compressed_position: vec4<f32>,
    @else
    @location(0) position: vec3<f32>,
    @if(VERTEX_UVS_A && VERTEX_UVS_A_COMPRESSED)
    @location(1) compressed_uv: vec2<f32>,
    @if(VERTEX_UVS_A && !VERTEX_UVS_A_COMPRESSED)
    @location(1) uv: vec2<f32>,
    @if(VERTEX_UVS_B && VERTEX_UVS_B_COMPRESSED)
    @location(2) compressed_uv_b: vec2<f32>,
    @if(VERTEX_UVS_B && !VERTEX_UVS_B_COMPRESSED)
    @location(2) uv_b: vec2<f32>,

    @if(NORMAL_PREPASS_OR_DEFERRED_PREPASS && VERTEX_NORMALS && VERTEX_NORMALS_COMPRESSED)
    @location(3) compressed_normal: vec2<f32>,
    @if(NORMAL_PREPASS_OR_DEFERRED_PREPASS && VERTEX_NORMALS && !VERTEX_NORMALS_COMPRESSED)
    @location(3) normal: vec3<f32>,
    @if(NORMAL_PREPASS_OR_DEFERRED_PREPASS && VERTEX_TANGENTS && VERTEX_TANGENTS_COMPRESSED)
    @location(4) compressed_tangent: vec2<f32>,
    @if(NORMAL_PREPASS_OR_DEFERRED_PREPASS && VERTEX_TANGENTS && !VERTEX_TANGENTS_COMPRESSED)
    @location(4) tangent: vec4<f32>,

    @if(SKINNED)
    @location(5) joint_indices: vec4<u32>,
    @if(SKINNED)
    @location(6) joint_weights: vec4<f32>,

    @if(VERTEX_COLORS)
    @location(7) color: vec4<f32>,

    @if(MORPH_TARGETS)
    @builtin(vertex_index) index: u32,
}

// The instance_index parameter must match vertex_in.instance_index. This is a work around for a wgpu dx12 bug.
// See https://github.com/gfx-rs/naga/issues/2416
fn decompress_vertex(vertex_in: Vertex, instance_index: u32) -> UncompressedVertex {
    let mesh_metadata = bevy_pbr::render::mesh_functions::get_metadata(instance_index);
    var uncompressed_vertex: UncompressedVertex;
    uncompressed_vertex.instance_index = instance_index;
@if(VERTEX_POSITIONS_COMPRESSED)
    uncompressed_vertex.position = bevy_render::utils::decompress_vertex_position(vertex_in.compressed_position, mesh_metadata.aabb_center, mesh_metadata.aabb_half_extents);
@else
    uncompressed_vertex.position = vertex_in.position;
@if(NORMAL_PREPASS_OR_DEFERRED_PREPASS && VERTEX_NORMALS && VERTEX_NORMALS_COMPRESSED)
    uncompressed_vertex.normal = bevy_render::utils::decompress_vertex_normal(vertex_in.compressed_normal);
@if(NORMAL_PREPASS_OR_DEFERRED_PREPASS && VERTEX_NORMALS && !VERTEX_NORMALS_COMPRESSED)
    uncompressed_vertex.normal = vertex_in.normal;
@if(NORMAL_PREPASS_OR_DEFERRED_PREPASS && VERTEX_TANGENTS && VERTEX_TANGENTS_COMPRESSED)
    uncompressed_vertex.tangent = bevy_render::utils::decompress_vertex_tangent(vertex_in.compressed_tangent);
@if(NORMAL_PREPASS_OR_DEFERRED_PREPASS && VERTEX_TANGENTS && !VERTEX_TANGENTS_COMPRESSED)
    uncompressed_vertex.tangent = vertex_in.tangent;
@if(VERTEX_UVS_A && VERTEX_UVS_A_COMPRESSED) {
    let uv_min_and_extents_a = mesh_metadata.uv_channels_min_and_extents[0];
    uncompressed_vertex.uv = bevy_render::utils::decompress_vertex_uv(vertex_in.compressed_uv, uv_min_and_extents_a);
}
@if(VERTEX_UVS_A && !VERTEX_UVS_A_COMPRESSED)
    uncompressed_vertex.uv = vertex_in.uv;
@if(VERTEX_UVS_B && VERTEX_UVS_B_COMPRESSED) {
    let uv_min_and_extents_b = mesh_metadata.uv_channels_min_and_extents[1];
    uncompressed_vertex.uv_b = bevy_render::utils::decompress_vertex_uv(vertex_in.compressed_uv_b, uv_min_and_extents_b);
}
@if(VERTEX_UVS_B && !VERTEX_UVS_B_COMPRESSED)
    uncompressed_vertex.uv_b = vertex_in.uv_b;
@if(VERTEX_COLORS)
    uncompressed_vertex.color = vertex_in.color;
@if(SKINNED) {
    uncompressed_vertex.joint_indices = vertex_in.joint_indices;
    uncompressed_vertex.joint_weights = vertex_in.joint_weights;
}
@if(MORPH_TARGETS)
    uncompressed_vertex.index = vertex_in.index;
    return uncompressed_vertex;
}

struct VertexOutput {
    // This is `clip position` when the struct is used as a vertex stage output
    // and `frag coord` when used as a fragment stage input
    @builtin(position) position: vec4<f32>,

    @if(VERTEX_UVS_A)
    @location(0) uv: vec2<f32>,

    @if(VERTEX_UVS_B)
    @location(1) uv_b: vec2<f32>,

    @if(NORMAL_PREPASS_OR_DEFERRED_PREPASS)
    @location(2) world_normal: vec3<f32>,
    @if(NORMAL_PREPASS_OR_DEFERRED_PREPASS && VERTEX_TANGENTS)
    @location(3) world_tangent: vec4<f32>,

    @location(4) world_position: vec4<f32>,
    @if(MOTION_VECTOR_PREPASS)
    @location(5) previous_world_position: vec4<f32>,

    @if(UNCLIPPED_DEPTH_ORTHO_EMULATION)
    @location(6) unclipped_depth: f32,
    @if(VERTEX_OUTPUT_INSTANCE_INDEX)
    @location(7) @interpolate(flat) instance_index: u32,

    @if(VERTEX_COLORS)
    @location(8) color: vec4<f32>,

    @if(VISIBILITY_RANGE_DITHER)
    @location(9) @interpolate(flat) visibility_range_dither: i32,
}

@if(PREPASS_FRAGMENT)
struct FragmentOutput {
    @if(NORMAL_PREPASS)
    @location(0) normal: vec4<f32>,

    @if(MOTION_VECTOR_PREPASS)
    @location(1) motion_vector: vec2<f32>,

    @if(DEFERRED_PREPASS)
    @location(2) deferred: vec4<u32>,
    @if(DEFERRED_PREPASS)
    @location(3) deferred_lighting_pass_id: u32,

    @if(UNCLIPPED_DEPTH_ORTHO_EMULATION)
    @builtin(frag_depth) frag_depth: f32,
}
