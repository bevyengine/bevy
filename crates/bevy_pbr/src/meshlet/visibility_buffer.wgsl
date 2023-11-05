#import bevy_pbr::{
    meshlet_bindings::{meshlet_thread_meshlet_ids, meshlets, meshlet_vertex_ids, meshlet_vertex_data, meshlet_thread_instance_ids, meshlet_instance_uniforms, view, get_meshlet_index, unpack_vertex},
    mesh_functions,
}
#import bevy_render::maths::affine_to_square

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) output: u32,
}

@vertex
fn vertex(@builtin(vertex_index) cull_output: u32) -> VertexOutput {
    let thread_id = cull_output >> 8u;
    let meshlet_id = meshlet_thread_meshlet_ids[thread_id];
    let meshlet = meshlets[meshlet_id];
    let index_id = extractBits(cull_output, 0u, 8u);
    let index = get_meshlet_index(meshlet.start_index_id + index_id);
    let vertex_id = meshlet_vertex_ids[meshlet.start_vertex_id + index];
    let vertex = unpack_vertex(meshlet_vertex_data[vertex_id]);
    let instance_id = meshlet_thread_instance_ids[thread_id];
    let instance_uniform = meshlet_instance_uniforms[instance_id];

    let model = affine_to_square(instance_uniform.model);
    let world_position = mesh_functions::mesh_position_local_to_world(model, vec4(vertex.position, 1.0));
    var clip_position = view.view_proj * vec4(world_position.xyz, 1.0);
#ifdef DEPTH_CLAMP_ORTHO
    clip_position.z = min(clip_position.z, 1.0);
#endif

    let output = (thread_id << 8u) | (index_id / 3u);
    return VertexOutput(clip_position, output);
}

@fragment
fn fragment(vertex_output: VertexOutput) -> @location(0) vec4<u32> {
    return vec4(vertex_output.output, 0u, 0u, 0u);
}
