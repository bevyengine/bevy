#import bevy_sprite::{
    mesh2d_functions as mesh_functions,
    mesh2d_vertex_output::VertexOutput,
    mesh2d_view_bindings::view,
    mesh2d_vertex_input::{Vertex, decompress_vertex}
}

#ifdef TONEMAP_IN_SHADER
#import bevy_core_pipeline::tonemapping
#endif

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let uncompressed_vertex = decompress_vertex(vertex, vertex.instance_index);
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
