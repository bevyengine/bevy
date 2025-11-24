#import bevy_sprite::{
    mesh2d_functions as mesh_functions,
    mesh2d_vertex_output::VertexOutput,
    mesh2d_view_bindings::view,
}

#ifdef TONEMAP_IN_SHADER
#import bevy_core_pipeline::tonemapping
#endif

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
    @location(0) position: vec3<f32>,
#endif
#ifdef VERTEX_NORMALS
#ifdef VERTEX_NORMALS_COMPRESSED
    @location(1) normal: vec2<f32>,
#else
    @location(1) normal: vec3<f32>,
#endif
#endif
#ifdef VERTEX_UVS
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
#ifdef VERTEX_TANGENTS_COMPRESSED
    @location(3) tangent: vec2<f32>,
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
#ifdef VERTEX_POSITIONS
    uncompressed_vertex.position = vertex_in.position;
#endif
#ifdef VERTEX_NORMALS
#ifdef VERTEX_NORMALS_COMPRESSED
    uncompressed_vertex.normal = bevy_pbr::utils::octahedral_decode(vertex_in.normal);
#else
    uncompressed_vertex.normal = vertex_in.normal;
#endif
#endif
#ifdef VERTEX_UVS
    uncompressed_vertex.uv = vertex_in.uv;
#endif
#ifdef VERTEX_TANGENTS
#ifdef VERTEX_TANGENTS_COMPRESSED
    let binormal_sign = sign(vertex_in.tangent.y);
    let tangent = vec4<f32>(bevy_pbr::utils::octahedral_decode(vec2<f32>(vertex_in.tangent.x, abs(vertex_in.tangent.y))), binormal_sign);
    uncompressed_vertex.tangent = tangent;
#else
    uncompressed_vertex.tangent = vertex_in.tangent;
#endif
#endif
#ifdef VERTEX_COLORS
    uncompressed_vertex.color = vertex_in.color;
#endif
    return uncompressed_vertex;
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let uncompressed_vertex = decompress_vertex(vertex);
#ifdef VERTEX_UVS
    out.uv = uncompressed_vertex.uv;
#endif

#ifdef VERTEX_POSITIONS
    var world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    out.world_position = mesh_functions::mesh2d_position_local_to_world(
        world_from_local,
        vec4<f32>(uncompressed_vertex.position, 1.0)
    );
    out.position = mesh_functions::mesh2d_position_world_to_clip(out.world_position);
#endif

#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh2d_normal_local_to_world(uncompressed_vertex.normal, vertex.instance_index);
#endif

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh2d_tangent_local_to_world(
        world_from_local,
        uncompressed_vertex.tangent
    );
#endif

#ifdef VERTEX_COLORS
    out.color = uncompressed_vertex.color;
#endif
    return out;
}

@fragment
fn fragment(
    in: VertexOutput,
) -> @location(0) vec4<f32> {
#ifdef VERTEX_COLORS
    var color = in.color;
#ifdef TONEMAP_IN_SHADER
    color = tonemapping::tone_mapping(color, view.color_grading);
#endif
    return color;
#else
    return vec4<f32>(1.0, 0.0, 1.0, 1.0);
#endif
}
