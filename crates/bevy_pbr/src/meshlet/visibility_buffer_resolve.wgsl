#define_import_path bevy_pbr::meshlet_visibility_buffer_resolve

#import bevy_pbr::{
    meshlet_bindings::{
        meshlet_visibility_buffer,
        meshlet_cluster_meshlet_ids,
        meshlets,
        meshlet_vertex_ids,
        meshlet_vertex_data,
        meshlet_cluster_instance_ids,
        meshlet_instance_uniforms,
        get_meshlet_index,
        unpack_meshlet_vertex,
    },
    mesh_view_bindings::view,
    mesh_functions::{mesh_position_local_to_world, sign_determinant_model_3x3m},
    mesh_types::{Mesh, MESH_FLAGS_SIGN_DETERMINANT_MODEL_3X3_BIT},
    view_transformations::{position_world_to_clip, frag_coord_to_ndc},
}
#import bevy_render::maths::{affine3_to_square, mat2x4_f32_to_mat3x3_unpack}

#ifdef PREPASS_FRAGMENT
#ifdef MOTION_VECTOR_PREPASS
#import bevy_pbr::{
    prepass_bindings::previous_view_uniforms,
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
    cluster_id: u32,
#ifdef PREPASS_FRAGMENT
#ifdef MOTION_VECTOR_PREPASS
    motion_vector: vec2<f32>,
#endif
#endif
}

/// Load the visibility buffer texture and resolve it into a VertexOutput.
fn resolve_vertex_output(frag_coord: vec4<f32>) -> VertexOutput {
    let packed_ids = textureLoad(meshlet_visibility_buffer, vec2<i32>(frag_coord.xy), 0).r;
    let cluster_id = packed_ids >> 6u;
    let meshlet_id = meshlet_cluster_meshlet_ids[cluster_id];
    let meshlet = meshlets[meshlet_id];

    let triangle_id = extractBits(packed_ids, 0u, 6u);
    let index_ids = meshlet.start_index_id + vec3(triangle_id * 3u) + vec3(0u, 1u, 2u);
    let indices = meshlet.start_vertex_id + vec3(get_meshlet_index(index_ids.x), get_meshlet_index(index_ids.y), get_meshlet_index(index_ids.z));
    let vertex_ids = vec3(meshlet_vertex_ids[indices.x], meshlet_vertex_ids[indices.y], meshlet_vertex_ids[indices.z]);
    let vertex_1 = unpack_meshlet_vertex(meshlet_vertex_data[vertex_ids.x]);
    let vertex_2 = unpack_meshlet_vertex(meshlet_vertex_data[vertex_ids.y]);
    let vertex_3 = unpack_meshlet_vertex(meshlet_vertex_data[vertex_ids.z]);

    let instance_id = meshlet_cluster_instance_ids[cluster_id];
    var instance_uniform = meshlet_instance_uniforms[instance_id];

    let world_from_local = affine3_to_square(instance_uniform.world_from_local);
    let world_position_1 = mesh_position_local_to_world(world_from_local, vec4(vertex_1.position, 1.0));
    let world_position_2 = mesh_position_local_to_world(world_from_local, vec4(vertex_2.position, 1.0));
    let world_position_3 = mesh_position_local_to_world(world_from_local, vec4(vertex_3.position, 1.0));

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
    let world_normal = mat3x3(
        normal_local_to_world(vertex_1.normal, &instance_uniform),
        normal_local_to_world(vertex_2.normal, &instance_uniform),
        normal_local_to_world(vertex_3.normal, &instance_uniform),
    ) * partial_derivatives.barycentrics;
    let uv = mat3x2(vertex_1.uv, vertex_2.uv, vertex_3.uv) * partial_derivatives.barycentrics;
    let ddx_uv = mat3x2(vertex_1.uv, vertex_2.uv, vertex_3.uv) * partial_derivatives.ddx;
    let ddy_uv = mat3x2(vertex_1.uv, vertex_2.uv, vertex_3.uv) * partial_derivatives.ddy;
    let world_tangent = mat3x4(
        tangent_local_to_world(vertex_1.tangent, world_from_local, instance_uniform.flags),
        tangent_local_to_world(vertex_2.tangent, world_from_local, instance_uniform.flags),
        tangent_local_to_world(vertex_3.tangent, world_from_local, instance_uniform.flags),
    ) * partial_derivatives.barycentrics;

#ifdef PREPASS_FRAGMENT
#ifdef MOTION_VECTOR_PREPASS
    let previous_world_from_local = affine3_to_square(instance_uniform.previous_world_from_local);
    let previous_world_position_1 = mesh_position_local_to_world(previous_world_from_local, vec4(vertex_1.position, 1.0));
    let previous_world_position_2 = mesh_position_local_to_world(previous_world_from_local, vec4(vertex_2.position, 1.0));
    let previous_world_position_3 = mesh_position_local_to_world(previous_world_from_local, vec4(vertex_3.position, 1.0));
    let previous_world_position = mat3x4(previous_world_position_1, previous_world_position_2, previous_world_position_3) * partial_derivatives.barycentrics;
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
        cluster_id,
#ifdef PREPASS_FRAGMENT
#ifdef MOTION_VECTOR_PREPASS
        motion_vector,
#endif
#endif
    );
}

fn normal_local_to_world(vertex_normal: vec3<f32>, instance_uniform: ptr<function, Mesh>) -> vec3<f32> {
    if any(vertex_normal != vec3<f32>(0.0)) {
        return normalize(
            mat2x4_f32_to_mat3x3_unpack(
                (*instance_uniform).local_from_world_transpose_a,
                (*instance_uniform).local_from_world_transpose_b,
            ) * vertex_normal
        );
    } else {
        return vertex_normal;
    }
}

fn tangent_local_to_world(vertex_tangent: vec4<f32>, world_from_local: mat4x4<f32>, mesh_flags: u32) -> vec4<f32> {
    if any(vertex_tangent != vec4<f32>(0.0)) {
        return vec4<f32>(
            normalize(
                mat3x3<f32>(
                    world_from_local[0].xyz,
                    world_from_local[1].xyz,
                    world_from_local[2].xyz,
                ) * vertex_tangent.xyz
            ),
            vertex_tangent.w * sign_determinant_model_3x3m(mesh_flags)
        );
    } else {
        return vertex_tangent;
    }
}
#endif
