#import bevy_pbr::{
    meshlet_bindings::{meshlet_visibility_buffer, meshlet_thread_meshlet_ids, meshlets, meshlet_vertex_ids, meshlet_vertex_data, meshlet_thread_instance_ids, meshlet_instance_uniforms, unpack_meshlet_vertex},
    mesh_functions,
    mesh_types::MESH_FLAGS_SIGN_DETERMINANT_MODEL_3X3_BIT,
    view_transformations::position_world_to_clip,
}
#import bevy_render::maths::{affine_to_square, mat2x4_f32_to_mat3x3_unpack}

// #ifdef PREPASS_PIPELINE
// #import bevy_pbr::prepass_io::VertexOutput
// #else
// #import bevy_pbr::forward_io::VertexOutput
// #endif

// @vertex
// fn meshlet_vertex(@builtin(vertex_index) packed_meshlet_index: u32) -> VertexOutput {
//     let thread_id = packed_meshlet_index >> 8u;
//     let meshlet_id = meshlet_thread_meshlet_ids[thread_id];
//     let meshlet = meshlets[meshlet_id];
//     let index = extractBits(packed_meshlet_index, 0u, 8u);
//     let vertex_id = meshlet_vertex_ids[meshlet.start_vertex_id + index];
//     let vertex = unpack_meshlet_vertex(meshlet_vertex_data[vertex_id]);
//     let instance_id = meshlet_thread_instance_ids[thread_id];
//     let instance_uniform = meshlet_instance_uniforms[instance_id];

//     var out: VertexOutput;
//     let model = affine_to_square(instance_uniform.model);
//     out.world_normal = normalize(
//         mat2x4_f32_to_mat3x3_unpack(
//             instance_uniform.inverse_transpose_model_a,
//             instance_uniform.inverse_transpose_model_b,
//         ) * vertex.normal
//     );
//     out.world_position = mesh_functions::mesh_position_local_to_world(model, vec4<f32>(vertex.position, 1.0));
//     out.position = position_world_to_clip(out.world_position.xyz);
//     out.uv = vertex.uv;
//     out.world_tangent = vec4<f32>(
//         normalize(
//             mat3x3<f32>(
//                 model[0].xyz,
//                 model[1].xyz,
//                 model[2].xyz
//             ) * vertex.tangent.xyz
//         ),
//         vertex.tangent.w * (f32(bool(instance_uniform.flags & MESH_FLAGS_SIGN_DETERMINANT_MODEL_3X3_BIT)) * 2.0 - 1.0)
//     );

// #ifdef MOTION_VECTOR_PREPASS
//     out.previous_world_position = mesh_functions::mesh_position_local_to_world(
//         affine_to_square(instance_uniform.previous_model),
//         vec4<f32>(vertex.position, 1.0)
//     );
// #endif

// #ifdef DEPTH_CLAMP_ORTHO
//     out.clip_position_unclamped = out.position;
//     out.position.z = min(out.position.z, 1.0);
// #endif

// #ifdef VERTEX_OUTPUT_MESH_FLAGS
//     out.mesh_flags = instance_uniform.flags;
// #endif

//     return out;
// }

fn rand_f(state: ptr<function, u32>) -> f32 {
    *state = *state * 747796405u + 2891336453u;
    let word = ((*state >> ((*state >> 28u) + 4u)) ^ *state) * 277803737u;
    return f32((word >> 22u) ^ word) * bitcast<f32>(0x2f800004u);
}

@vertex
fn vertex(@builtin(vertex_index) vertex_input: u32) -> @builtin(position) vec4<f32> {
    let vertex_index = vertex_input % 3u;
    let material_id = vertex_input / 3u;
    let material_depth = f32(material_id) / 65535.0;
    let uv = vec2<f32>(vec2(vertex_index >> 1u, vertex_index & 1u)) * 2.0;
    return vec4(uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0), material_depth, 1.0);
}

@fragment
fn fragment(@builtin(position) clip_position: vec4<f32>) -> @location(0) vec4<f32> {
    let vbuffer = textureLoad(meshlet_visibility_buffer, vec2<i32>(clip_position.xy), 0).r;
    let thread_id = vbuffer >> 8u;
    let meshlet_id = meshlet_thread_meshlet_ids[thread_id];
    let meshlet = meshlets[meshlet_id];
    let triangle_id = extractBits(vbuffer, 0u, 8u);

    let indices = meshlet.start_vertex_id + vec3(triangle_id * 3u) + vec3(0u, 1u, 2u);
    let vertex_ids = vec3(meshlet_vertex_ids[indices.x], meshlet_vertex_ids[indices.y], meshlet_vertex_ids[indices.z]);
    let vertex_1 = unpack_meshlet_vertex(meshlet_vertex_data[vertex_ids.x]);
    let vertex_2 = unpack_meshlet_vertex(meshlet_vertex_data[vertex_ids.y]);
    let vertex_3 = unpack_meshlet_vertex(meshlet_vertex_data[vertex_ids.z]);

    // TODO: Barycentric interpolation

    let instance_id = meshlet_thread_instance_ids[thread_id];
    let instance_uniform = meshlet_instance_uniforms[instance_id];

    // TODO: Compute vertex output

    var rng = meshlet_id;
    let color = vec3(rand_f(&rng), rand_f(&rng), rand_f(&rng));
    return vec4(color, 1.0);
}
