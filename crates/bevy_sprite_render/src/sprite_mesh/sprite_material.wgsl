#import bevy_sprite::{
    mesh2d_functions as mesh_functions,
    mesh2d_vertex_output::VertexOutput,
    mesh2d_view_bindings::view,
    mesh2d_vertex_input::{Vertex, decompress_vertex},
    mesh2d_bindings,
    sprite_bindings::{material, material_indices},
    sprite_functions,
}

#ifdef TONEMAP_IN_SHADER
#import bevy_core_pipeline::tonemapping
#endif
#ifdef SRGB_OUTPUT
#import bevy_render::color_operations::linear_to_srgb
#endif
#ifdef OKLAB_OUTPUT
#import bevy_render::color_operations::linear_rgb_to_oklab
#endif

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let instance_index = vertex.instance_index;

#ifdef BINDLESS
    let slot = mesh2d_bindings::mesh[instance_index].material_bind_group_slot;
    let vertex_scale = material[material_indices[slot].material].vertex_scale;
    let vertex_offset = material[material_indices[slot].material].vertex_offset;
#else   // BINDLESS
    let vertex_scale = material.vertex_scale;
    let vertex_offset = material.vertex_offset;
#endif  // BINDLESS

    var out: VertexOutput;
    let uncompressed_vertex = decompress_vertex(vertex, instance_index);
#ifdef VERTEX_UVS
    out.uv = uncompressed_vertex.uv;
#endif

#ifdef VERTEX_POSITIONS
    var world_from_local = mesh_functions::get_world_from_local(instance_index);
    let position = vec4<f32>(uncompressed_vertex.position * vec3<f32>(vertex_scale, 1.0) + vec3<f32>(vertex_offset, 0.0), 1.0);

    out.world_position = mesh_functions::mesh2d_position_local_to_world(
        world_from_local,
        position
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

    out.instance_index = instance_index;

    return out;
}

@fragment
fn fragment(
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    var output_color = sprite_functions::sample_final_color(mesh.uv, mesh.instance_index);

#ifdef TONEMAP_IN_SHADER
    output_color = tonemapping::tone_mapping(output_color, view.color_grading);
#endif

#ifdef SRGB_OUTPUT
    output_color = vec4(linear_to_srgb(output_color.rgb), output_color.a);
#endif

#ifdef OKLAB_OUTPUT
    output_color = vec4(linear_rgb_to_oklab(output_color.rgb), output_color.a);
#endif

    return output_color;
}
