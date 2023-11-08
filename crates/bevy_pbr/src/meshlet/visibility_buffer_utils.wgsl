#define_import_path bevy_pbr::meshlet_visibility_buffer_utils

#import bevy_pbr::{
    meshlet_bindings::{meshlet_visibility_buffer, meshlet_thread_meshlet_ids, meshlets, meshlet_vertex_ids, meshlet_vertex_data, meshlet_thread_instance_ids, meshlet_instance_uniforms, unpack_meshlet_vertex},
    mesh_functions::mesh_position_local_to_world,
    mesh_types::MESH_FLAGS_SIGN_DETERMINANT_MODEL_3X3_BIT,
    view_transformations::{position_world_to_clip, frag_coord_to_ndc},
}
#import bevy_pbr::mesh_view_bindings::view
#import bevy_render::maths::{affine_to_square, mat2x4_f32_to_mat3x3_unpack}

struct PartialDerivatives {
    barycentrics: vec3<f32>,
    ddx: vec3<f32>,
    ddy: vec3<f32>,
}

// https://github.com/ConfettiFX/The-Forge/blob/2d453f376ef278f66f97cbaf36c0d12e4361e275/Examples_3/Visibility_Buffer/src/Shaders/FSL/visibilityBuffer_shade.frag.fsl#L83-L139
fn compute_derivatives(vertex_clip_positions: array<vec4<f32>, 3>, ndc_uv: vec2<f32>, screen_size: vec2<f32>) -> PartialDerivatives {
    var result: PartialDerivatives;

    let inv_w = 1.0 / vec3(vertex_clip_positions[0].w, vertex_clip_positions[1].w, vertex_clip_positions[2].w);
    let ndc_0 = vertex_clip_positions[0].xy * inv_w[0];
    let ndc_1 = vertex_clip_positions[1].xy * inv_w[1];
    let ndc_2 = vertex_clip_positions[2].xy * inv_w[2];

    let inv_det = 1.0 / determinant(mat2x2(ndc_2 - ndc_1, ndc_0 - ndc_1));
    result.ddx = vec3(ndc_1.y - ndc_2.y, ndc_2.y - ndc_0.y, ndc_0.y - ndc_1.y) * inv_det * inv_w;
    result.ddy = vec3(ndc_2.x - ndc_1.x, ndc_0.x - ndc_2.x, ndc_1.x - ndc_0.x) * inv_det * inv_w;

    var ddx_sum = dot(result.ddx, vec3(1.0));
    var ddy_sum = dot(result.ddy, vec3(1.0));

    let delta_v = ndc_uv - ndc_0;
    let interp_inv_w = inv_w.x + delta_v.x * ddx_sum + delta_v.y * ddy_sum;
    let interp_w = 1.0 / interp_inv_w;

    result.barycentrics = vec3(
        interp_w * (delta_v.x * result.ddx.x + delta_v.y * result.ddy.x + inv_w.x),
        interp_w * (delta_v.x * result.ddx.y + delta_v.y * result.ddy.y),
        interp_w * (delta_v.x * result.ddx.z + delta_v.y * result.ddy.z),
    );

    result.ddx *= 2.0 / screen_size.x;
    result.ddy *= 2.0 / screen_size.y;
    ddx_sum *= 2.0 / screen_size.x;
    ddy_sum *= 2.0 / screen_size.y;

    let interp_ddx_w = 1.0 / (interp_inv_w + ddx_sum);
    let interp_ddy_w = 1.0 / (interp_inv_w + ddy_sum);

    result.ddx = interp_ddx_w * (result.barycentrics * interp_inv_w + result.ddx) - result.barycentrics;
    result.ddy = interp_ddy_w * (result.barycentrics * interp_inv_w + result.ddy) - result.barycentrics;
    return result;
}

struct VertexOutput {
    meshlet_id: u32,
}

fn load_vertex_output(frag_coord: vec4<f32>) -> VertexOutput {
    let vbuffer = textureLoad(meshlet_visibility_buffer, vec2<i32>(frag_coord.xy), 0).r;
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
        frag_coord_to_ndc(frag_coord).xy,
        view.viewport.zw,
    );

    // TODO: Compute vertex output
    return VertexOutput(meshlet_id);
}
