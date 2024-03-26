#define_import_path bevy_pbr::meshlet_visibility_buffer_resolve

#import bevy_pbr::{
    meshlet_bindings::{
        meshlet_visibility_buffer,
        meshlet_thread_meshlet_ids,
        meshlets,
        meshlet_vertex_ids,
        meshlet_vertex_data,
        meshlet_thread_instance_ids,
        meshlet_instance_uniforms,
        get_meshlet_index,
        unpack_meshlet_vertex,
    },
    mesh_view_bindings::view,
    mesh_functions::mesh_position_local_to_world,
    mesh_types::MESH_FLAGS_SIGN_DETERMINANT_MODEL_3X3_BIT,
    view_transformations::{position_world_to_clip, frag_coord_to_ndc},
}
#import bevy_render::maths::{affine3_to_square, mat2x4_f32_to_mat3x3_unpack}

#ifdef PREPASS_FRAGMENT
#ifdef MOTION_VECTOR_PREPASS
#import bevy_pbr::{
    prepass_bindings::previous_view_proj,
    pbr_prepass_functions::calculate_motion_vector,
}
#endif
#endif

/// Functions to be used by materials for reading from a meshlet visibility buffer texture.

#ifdef MESHLET_MESH_MATERIAL_PASS
struct PartialDerivatives {
    barycentrics: vec3<f32>,
    ddx: vec3<f32>,
    ddy: vec3<f32>,
}

// https://github.com/ConfettiFX/The-Forge/blob/2d453f376ef278f66f97cbaf36c0d12e4361e275/Examples_3/Visibility_Buffer/src/Shaders/FSL/visibilityBuffer_shade.frag.fsl#L83-L139
fn compute_partial_derivatives(vertex_clip_positions: array<vec4<f32>, 3>, ndc_uv: vec2<f32>, screen_size: vec2<f32>) -> PartialDerivatives {
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
    position: vec4<f32>,
    world_position: vec4<f32>,
    world_normal: vec3<f32>,
    uv: vec2<f32>,
    ddx_uv: vec2<f32>,
    ddy_uv: vec2<f32>,
    world_tangent: vec4<f32>,
    mesh_flags: u32,
    meshlet_id: u32,
#ifdef PREPASS_FRAGMENT
#ifdef MOTION_VECTOR_PREPASS
    motion_vector: vec2<f32>,
#endif
#endif
}

/// Load the visibility buffer texture and resolve it into a VertexOutput.
fn resolve_vertex_output(frag_coord: vec4<f32>) -> VertexOutput {
    let vbuffer = textureLoad(meshlet_visibility_buffer, vec2<i32>(frag_coord.xy), 0).r;
    let cluster_id = vbuffer >> 8u;
    let meshlet_id = meshlet_thread_meshlet_ids[cluster_id];
    let meshlet = meshlets[meshlet_id];
    let triangle_id = extractBits(vbuffer, 0u, 8u);
    let index_ids = meshlet.start_index_id + vec3(triangle_id * 3u) + vec3(0u, 1u, 2u);
    let indices = meshlet.start_vertex_id + vec3(get_meshlet_index(index_ids.x), get_meshlet_index(index_ids.y), get_meshlet_index(index_ids.z));
    let vertex_ids = vec3(meshlet_vertex_ids[indices.x], meshlet_vertex_ids[indices.y], meshlet_vertex_ids[indices.z]);
    let vertex_1 = unpack_meshlet_vertex(meshlet_vertex_data[vertex_ids.x]);
    let vertex_2 = unpack_meshlet_vertex(meshlet_vertex_data[vertex_ids.y]);
    let vertex_3 = unpack_meshlet_vertex(meshlet_vertex_data[vertex_ids.z]);

    let instance_id = meshlet_thread_instance_ids[cluster_id];
    let instance_uniform = meshlet_instance_uniforms[instance_id];
    let model = affine3_to_square(instance_uniform.model);

    let world_position_1 = mesh_position_local_to_world(model, vec4(vertex_1.position, 1.0));
    let world_position_2 = mesh_position_local_to_world(model, vec4(vertex_2.position, 1.0));
    let world_position_3 = mesh_position_local_to_world(model, vec4(vertex_3.position, 1.0));
    let clip_position_1 = position_world_to_clip(world_position_1.xyz);
    let clip_position_2 = position_world_to_clip(world_position_2.xyz);
    let clip_position_3 = position_world_to_clip(world_position_3.xyz);
    let frag_coord_ndc = frag_coord_to_ndc(frag_coord).xy;
    let partial_derivatives = compute_partial_derivatives(
        array(clip_position_1, clip_position_2, clip_position_3),
        frag_coord_ndc,
        view.viewport.zw,
    );

    let world_position = mat3x4(world_position_1, world_position_2, world_position_3) * partial_derivatives.barycentrics;
    let vertex_normal = mat3x3(vertex_1.normal, vertex_2.normal, vertex_3.normal) * partial_derivatives.barycentrics;
    let world_normal = normalize(
        mat2x4_f32_to_mat3x3_unpack(
            instance_uniform.inverse_transpose_model_a,
            instance_uniform.inverse_transpose_model_b,
        ) * vertex_normal
    );
    let uv = mat3x2(vertex_1.uv, vertex_2.uv, vertex_3.uv) * partial_derivatives.barycentrics;
    let ddx_uv = mat3x2(vertex_1.uv, vertex_2.uv, vertex_3.uv) * partial_derivatives.ddx;
    let ddy_uv = mat3x2(vertex_1.uv, vertex_2.uv, vertex_3.uv) * partial_derivatives.ddy;
    let vertex_tangent = mat3x4(vertex_1.tangent, vertex_2.tangent, vertex_3.tangent) * partial_derivatives.barycentrics;
    let world_tangent = vec4(
        normalize(
            mat3x3(
                model[0].xyz,
                model[1].xyz,
                model[2].xyz
            ) * vertex_tangent.xyz
        ),
        vertex_tangent.w * (f32(bool(instance_uniform.flags & MESH_FLAGS_SIGN_DETERMINANT_MODEL_3X3_BIT)) * 2.0 - 1.0)
    );

#ifdef PREPASS_FRAGMENT
#ifdef MOTION_VECTOR_PREPASS
    let previous_model = affine3_to_square(instance_uniform.previous_model);
    let previous_world_position_1 = mesh_position_local_to_world(previous_model, vec4(vertex_1.position, 1.0));
    let previous_world_position_2 = mesh_position_local_to_world(previous_model, vec4(vertex_2.position, 1.0));
    let previous_world_position_3 = mesh_position_local_to_world(previous_model, vec4(vertex_3.position, 1.0));
    let previous_clip_position_1 = previous_view_proj * vec4(previous_world_position_1.xyz, 1.0);
    let previous_clip_position_2 = previous_view_proj * vec4(previous_world_position_2.xyz, 1.0);
    let previous_clip_position_3 = previous_view_proj * vec4(previous_world_position_3.xyz, 1.0);
    let previous_partial_derivatives = compute_partial_derivatives(
        array(previous_clip_position_1, previous_clip_position_2, previous_clip_position_3),
        frag_coord_ndc,
        view.viewport.zw,
    );
    let previous_world_position = mat3x4(previous_world_position_1, previous_world_position_2, previous_world_position_3) * previous_partial_derivatives.barycentrics;
    let motion_vector = calculate_motion_vector(world_position, previous_world_position);
#endif
#endif

    return VertexOutput(
        frag_coord,
        world_position,
        world_normal,
        uv,
        ddx_uv,
        ddy_uv,
        world_tangent,
        instance_uniform.flags,
        meshlet_id,
#ifdef PREPASS_FRAGMENT
#ifdef MOTION_VECTOR_PREPASS
        motion_vector,
#endif
#endif
    );
}
#endif
