#import bevy_pbr::meshlet_bindings meshlet_thread_meshlet_ids, meshlets, meshlet_vertex_ids, meshlet_vertex_data, meshlet_thread_instance_ids, meshlet_instance_uniforms
#import bevy_pbr::mesh_functions
#import bevy_pbr::mesh_types MESH_FLAGS_SIGN_DETERMINANT_MODEL_3X3_BIT
#import bevy_render::maths affine_to_square, mat2x4_f32_to_mat3x3_unpack

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
    let thread_id = packed_meshlet_index >> 8u;
    let meshlet_id = meshlet_thread_meshlet_ids[thread_id];
    let meshlet = meshlets[meshlet_id];
    let index = extractBits(packed_meshlet_index, 0u, 8u);
    let vertex_id = meshlet_vertex_ids[meshlet.start_vertex_id + index];
    let vertex = meshlet_vertex_data[vertex_id];
    let instance_id = meshlet_thread_instance_ids[thread_id];
    let instance_uniform = meshlet_instance_uniforms[instance_id];

    var out: VertexOutput;
    let model = affine_to_square(instance_uniform.model);
    out.world_normal = normalize(
        mat2x4_f32_to_mat3x3_unpack(
            instance_uniform.inverse_transpose_model_a,
            instance_uniform.inverse_transpose_model_b,
        ) * vertex.normal
    );
    out.world_position = mesh_functions::mesh_position_local_to_world(model, vec4<f32>(vertex.position, 1.0));
    out.position = mesh_functions::mesh_position_world_to_clip(out.world_position);
    out.uv = vertex.uv;
    out.world_tangent = vec4<f32>(
        normalize(
            mat3x3<f32>(
                model[0].xyz,
                model[1].xyz,
                model[2].xyz
            ) * vertex.tangent.xyz
        ),
        vertex.tangent.w * (f32(bool(instance_uniform.flags & MESH_FLAGS_SIGN_DETERMINANT_MODEL_3X3_BIT)) * 2.0 - 1.0)
    );
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var rng = in.meshlet_id;
    let base_color = vec3(rand_f(&rng), rand_f(&rng), rand_f(&rng));
    let cos_theta = max(dot(in.world_normal, vec3(0.0, 0.0, 1.0)), vec3(0.0));
    return vec4(base_color * cos_theta, 1.0);
}
