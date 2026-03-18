#define_import_path bevy_sprite::mesh2d_vertex_input

#import bevy_sprite::mesh2d_functions as mesh_functions

struct UncompressedVertex {
    @builtin(instance_index) instance_index: u32,
#ifdef VERTEX_POSITIONS
    @location(0) position: vec3<f32>,
#endif
#ifdef VERTEX_NORMALS
    @location(1) normal: vec3<f32>,
#endif
#ifdef VERTEX_UVS
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(3) tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(4) color: vec4<f32>,
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
#ifdef VERTEX_UVS
#ifdef VERTEX_UVS_COMPRESSED
    @location(2) compressed_uv: vec2<f32>,
#else
    @location(2) uv: vec2<f32>,
#endif
#endif
#ifdef VERTEX_TANGENTS
#ifdef VERTEX_TANGENTS_COMPRESSED
    @location(3) compressed_tangent: vec2<f32>,
#else
    @location(3) tangent: vec4<f32>,
#endif
#endif
#ifdef VERTEX_COLORS
    @location(4) color: vec4<f32>,
#endif
};

fn decompress_vertex(vertex_in: Vertex) -> UncompressedVertex {
    var uncompressed_vertex: UncompressedVertex;
    uncompressed_vertex.instance_index = vertex_in.instance_index;
#ifdef VERTEX_POSITIONS_COMPRESSED
    uncompressed_vertex.position = mesh_functions::decompress_vertex_position(vertex_in.instance_index, vertex_in.compressed_position);
#else
    uncompressed_vertex.position = vertex_in.position;
#endif
#ifdef VERTEX_NORMALS
#ifdef VERTEX_NORMALS_COMPRESSED
    uncompressed_vertex.normal = mesh_functions::decompress_vertex_normal(vertex_in.compressed_normal);
#else
    uncompressed_vertex.normal = vertex_in.normal;
#endif
#endif
#ifdef VERTEX_UVS
#ifdef VERTEX_UVS_COMPRESSED
    uncompressed_vertex.uv = mesh_functions::decompress_vertex_uv_channel(vertex_in.instance_index, vertex_in.compressed_uv, 0);
#else
    uncompressed_vertex.uv = vertex_in.uv;
#endif
#endif
#ifdef VERTEX_TANGENTS
#ifdef VERTEX_TANGENTS_COMPRESSED
    uncompressed_vertex.tangent = mesh_functions::decompress_vertex_tangent(vertex_in.compressed_tangent);
#else
    uncompressed_vertex.tangent = vertex_in.tangent;
#endif
#endif
#ifdef VERTEX_COLORS
    uncompressed_vertex.color = vertex_in.color;
#endif
    return uncompressed_vertex;
}
