import bevy_sprite_render::mesh2d::{
    functions as mesh_functions,
    vertex_output::VertexOutput,
    view_bindings::view,
    vertex_input::{Vertex, decompress_vertex}
};

@if(TONEMAP_IN_SHADER)
import bevy_core_pipeline::tonemapping;
@if(SRGB_OUTPUT)
import bevy_render::color_operations::linear_to_srgb;
@if(OKLAB_OUTPUT)
import bevy_render::color_operations::linear_rgb_to_oklab;

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let uncompressed_vertex = decompress_vertex(vertex, vertex.instance_index);
@if(VERTEX_UVS)
    out.uv = uncompressed_vertex.uv;

@if(VERTEX_POSITIONS)
    var world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
@if(VERTEX_POSITIONS)
    out.world_position = mesh_functions::mesh2d_position_local_to_world(
        world_from_local,
        vec4<f32>(uncompressed_vertex.position, 1.0)
    );
@if(VERTEX_POSITIONS)
    out.position = mesh_functions::mesh2d_position_world_to_clip(out.world_position);

@if(VERTEX_NORMALS)
    out.world_normal = mesh_functions::mesh2d_normal_local_to_world(uncompressed_vertex.normal, vertex.instance_index);

@if(VERTEX_TANGENTS)
    out.world_tangent = mesh_functions::mesh2d_tangent_local_to_world(
        world_from_local,
        uncompressed_vertex.tangent
    );

@if(VERTEX_COLORS)
    out.color = uncompressed_vertex.color;

    out.instance_index = vertex.instance_index;

    return out;
}

@fragment
fn fragment(
    in: VertexOutput,
) -> @location(0) vec4<f32> {
@if(VERTEX_COLORS) {
    var color = in.color;
@if(TONEMAP_IN_SHADER)
    color = tonemapping::tone_mapping(color, view.color_grading);
@if(SRGB_OUTPUT)
    color = vec4(linear_to_srgb(color.rgb), color.a);
@if(OKLAB_OUTPUT)
    color = vec4(linear_rgb_to_oklab(color.rgb), color.a);
    return color;
} @else {
    return vec4<f32>(1.0, 0.0, 1.0, 1.0);
}
}
