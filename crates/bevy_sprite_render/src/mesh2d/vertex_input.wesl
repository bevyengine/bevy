import bevy_sprite_render::mesh2d::functions as mesh_functions;

struct UncompressedVertex {
    @builtin(instance_index) instance_index: u32,
    @if(VERTEX_POSITIONS)
    @location(0) position: vec3<f32>,
    @if(VERTEX_NORMALS)
    @location(1) normal: vec3<f32>,
    @if(VERTEX_UVS)
    @location(2) uv: vec2<f32>,
    @if(VERTEX_TANGENTS)
    @location(3) tangent: vec4<f32>,
    @if(VERTEX_COLORS)
    @location(4) color: vec4<f32>,
};

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @if(VERTEX_POSITIONS && VERTEX_POSITIONS_COMPRESSED)
    @location(0) compressed_position: vec4<f32>,
    @if(VERTEX_POSITIONS && !VERTEX_POSITIONS_COMPRESSED)
    @location(0) position: vec3<f32>,
    @if(VERTEX_NORMALS && VERTEX_NORMALS_COMPRESSED)
    @location(1) compressed_normal: vec2<f32>,
    @if(VERTEX_NORMALS && !VERTEX_NORMALS_COMPRESSED)
    @location(1) normal: vec3<f32>,
    @if(VERTEX_UVS && VERTEX_UVS_COMPRESSED)
    @location(2) compressed_uv: vec2<f32>,
    @if(VERTEX_UVS && !VERTEX_UVS_COMPRESSED)
    @location(2) uv: vec2<f32>,
    @if(VERTEX_TANGENTS && VERTEX_TANGENTS_COMPRESSED)
    @location(3) compressed_tangent: vec2<f32>,
    @if(VERTEX_TANGENTS && !VERTEX_TANGENTS_COMPRESSED)
    @location(3) tangent: vec4<f32>,
    @if(VERTEX_COLORS)
    @location(4) color: vec4<f32>,
};

// The instance_index parameter must match vertex_in.instance_index. This is a work around for a wgpu dx12 bug.
// See https://github.com/gfx-rs/naga/issues/2416
fn decompress_vertex(vertex_in: Vertex, instance_index: u32) -> UncompressedVertex {
    let mesh_metadata = mesh_functions::get_metadata(instance_index);
    var uncompressed_vertex: UncompressedVertex;
    uncompressed_vertex.instance_index = instance_index;
    @if(VERTEX_POSITIONS_COMPRESSED)
    uncompressed_vertex.position = bevy_render::utils::decompress_vertex_position(vertex_in.compressed_position, mesh_metadata.aabb_center, mesh_metadata.aabb_half_extents);
    @else
    uncompressed_vertex.position = vertex_in.position;
    @if(VERTEX_NORMALS && VERTEX_NORMALS_COMPRESSED)
    uncompressed_vertex.normal = bevy_render::utils::decompress_vertex_normal(vertex_in.compressed_normal);
    @if(VERTEX_NORMALS && !VERTEX_NORMALS_COMPRESSED)
    uncompressed_vertex.normal = vertex_in.normal;
    @if(VERTEX_UVS && VERTEX_UVS_COMPRESSED)
    let uv_min_and_extents = mesh_metadata.uv_channels_min_and_extents[0];
    @if(VERTEX_UVS && VERTEX_UVS_COMPRESSED)
    uncompressed_vertex.uv = bevy_render::utils::decompress_vertex_uv(vertex_in.compressed_uv, uv_min_and_extents);
    @if(VERTEX_UVS && !VERTEX_UVS_COMPRESSED)
    uncompressed_vertex.uv = vertex_in.uv;
    @if(VERTEX_TANGENTS && VERTEX_TANGENTS_COMPRESSED)
    uncompressed_vertex.tangent = bevy_render::utils::decompress_vertex_tangent(vertex_in.compressed_tangent);
    @if(VERTEX_TANGENTS && !VERTEX_TANGENTS_COMPRESSED)
    uncompressed_vertex.tangent = vertex_in.tangent;
    @if(VERTEX_COLORS)
    uncompressed_vertex.color = vertex_in.color;
    return uncompressed_vertex;
}
