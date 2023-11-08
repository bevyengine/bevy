#import bevy_pbr::{
    meshlet_bindings::{meshlet_visibility_buffer, meshlet_thread_meshlet_ids, meshlets, meshlet_vertex_ids, meshlet_vertex_data, meshlet_thread_instance_ids, meshlet_instance_uniforms, unpack_meshlet_vertex, compute_derivatives},
    mesh_functions::mesh_position_local_to_world,
    mesh_types::MESH_FLAGS_SIGN_DETERMINANT_MODEL_3X3_BIT,
    view_transformations::{uv_to_ndc, position_world_to_clip},
}
#import bevy_pbr::mesh_view_bindings::view
#import bevy_render::maths::{affine_to_square, mat2x4_f32_to_mat3x3_unpack}

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
    return vec4(uv_to_ndc(uv), material_depth, 1.0);
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

    let instance_id = meshlet_thread_instance_ids[thread_id];
    let instance_uniform = meshlet_instance_uniforms[instance_id];
    let model = affine_to_square(instance_uniform.model);

    let world_position_1 = mesh_position_local_to_world(model, vec4(vertex_1.position, 1.0));
    let world_position_2 = mesh_position_local_to_world(model, vec4(vertex_2.position, 1.0));
    let world_position_3 = mesh_position_local_to_world(model, vec4(vertex_3.position, 1.0));
    let clip_position_1 = position_world_to_clip(world_position_1.xyz);
    let clip_position_2 = position_world_to_clip(world_position_2.xyz);
    let clip_position_3 = position_world_to_clip(world_position_3.xyz);

    let partial_derivatives = compute_derivatives(
        array(clip_position_1, clip_position_2, clip_position_3),
        clip_position.xy,
        view.viewport.zw,
    );

    // TODO: Compute vertex output

    var rng = meshlet_id;
    let color = vec3(rand_f(&rng), rand_f(&rng), rand_f(&rng));
    return vec4(color, 1.0);
}
