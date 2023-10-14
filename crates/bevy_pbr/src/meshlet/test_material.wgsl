#import bevy_pbr::meshlet_bindings meshlet_thread_meshlet_ids, meshlets, meshlet_vertex_ids, meshlet_vertex_data, meshlet_thread_instance_ids, meshlet_instance_uniforms

fn rand_f(state: ptr<function, u32>) -> f32 {
    *state = *state * 747796405u + 2891336453u;
    let word = ((*state >> ((*state >> 28u) + 4u)) ^ *state) * 277803737u;
    return f32((word >> 22u) ^ word) * bitcast<f32>(0x2f800004u);
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) world_tangent: vec4<f32>,
    @location(4) @interpolate(flat) meshlet_id: u32,
}

@vertex
fn vertex(@builtin(vertex_index) packed_meshlet_index: u32) -> VertexOutput {
    let thead_id = packed_meshlet_index >> 8u;
    let meshlet_id = meshlet_thread_meshlet_ids[thead_id];
    let meshlet = meshlets[meshlet_id];
    let index = extractBits(packed_meshlet_index, 0u, 8u);
    let vertex_id = meshlet_vertex_ids[meshlet.start_vertex_id + index];
    let vertex = meshlet_vertex_data[vertex_id];
    let instance_id = meshlet_thread_instance_ids[thead_id];
    let instance_uniform = meshlet_instance_uniforms[instance_id];

    var out: VertexOutput;
    // TODO
    // var model = mesh_functions::get_model_matrix(vertex_no_morph.instance_index);
    // out.world_normal = mesh_functions::mesh_normal_local_to_world(
    //     vertex.normal,
    //     get_instance_index(vertex_no_morph.instance_index)
    // );
    // out.world_position = mesh_functions::mesh_position_local_to_world(model, vec4<f32>(vertex.position, 1.0));
    // out.position = mesh_functions::mesh_position_world_to_clip(out.world_position);
    out.uv = vertex.uv;
    // out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
    //     model,
    //     vertex.tangent,
    //     get_instance_index(vertex_no_morph.instance_index)
    // );
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var rng = in.meshlet_id;
    let light_position = vec3(10.0);
    let light_direction = normalize(light_position - in.world_position);
    let cos_theta = max(dot(in.world_normal, light_direction), vec3(0.0));
    let base_color = vec3(rand_f(&rng), rand_f(&rng), rand_f(&rng));
    let light = base_color * cos_theta;
    return vec4(light, 1.0);
}
