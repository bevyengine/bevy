#define_import_path bevy_pbr::meshlet_visibility_buffer_resolve

#import bevy_pbr::{
    meshlet_bindings::{
        Meshlet,
        meshlet_visibility_buffer,
        meshlet_raster_clusters,
        meshlets,
        meshlet_instance_uniforms,
        get_meshlet_vertex_id,
        get_meshlet_vertex_position,
        get_meshlet_vertex_normal,
        get_meshlet_vertex_uv,
    },
    mesh_view_bindings::view,
    mesh_functions::mesh_position_local_to_world,
    mesh_types::Mesh,
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

// https://github.com/ConfettiFX/The-Forge/blob/9d43e69141a9cd0ce2ce2d2db5122234d3a2d5b5/Common_3/Renderer/VisibilityBuffer2/Shaders/FSL/vb_shading_utilities.h.fsl#L90-L150
fn compute_partial_derivatives(vertex_world_positions: array<vec4<f32>, 3>, ndc_uv: vec2<f32>, half_screen_size: vec2<f32>) -> PartialDerivatives {
    var result: PartialDerivatives;

    let vertex_clip_position_0 = position_world_to_clip(vertex_world_positions[0].xyz);
    let vertex_clip_position_1 = position_world_to_clip(vertex_world_positions[1].xyz);
    let vertex_clip_position_2 = position_world_to_clip(vertex_world_positions[2].xyz);

    let inv_w = 1.0 / vec3(vertex_clip_position_0.w, vertex_clip_position_1.w, vertex_clip_position_2.w);
    let ndc_0 = vertex_clip_position_0.xy * inv_w[0];
    let ndc_1 = vertex_clip_position_1.xy * inv_w[1];
    let ndc_2 = vertex_clip_position_2.xy * inv_w[2];

    let inv_det = 1.0 / determinant(mat2x2(ndc_2 - ndc_1, ndc_0 - ndc_1));
    result.ddx = vec3(ndc_1.y - ndc_2.y, ndc_2.y - ndc_0.y, ndc_0.y - ndc_1.y) * inv_det * inv_w;
    result.ddy = vec3(ndc_2.x - ndc_1.x, ndc_0.x - ndc_2.x, ndc_1.x - ndc_0.x) * inv_det * inv_w;

    var ddx_sum = dot(result.ddx, vec3(1.0));
    var ddy_sum = dot(result.ddy, vec3(1.0));

    let delta_v = ndc_uv - ndc_0;
    let interp_inv_w = inv_w.x + delta_v.x * ddx_sum + delta_v.y * ddy_sum;
    let interp_w = 1.0 / interp_inv_w;

    result.barycentrics = vec3(
        interp_w * (inv_w[0] + delta_v.x * result.ddx.x + delta_v.y * result.ddy.x),
        interp_w * (delta_v.x * result.ddx.y + delta_v.y * result.ddy.y),
        interp_w * (delta_v.x * result.ddx.z + delta_v.y * result.ddy.z),
    );

    result.ddx *= half_screen_size.x;
    result.ddy *= half_screen_size.y;
    ddx_sum *= half_screen_size.x;
    ddy_sum *= half_screen_size.y;

    result.ddy *= -1.0;
    ddy_sum *= -1.0;

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
    material_bind_group_slot: u32,
#ifdef PREPASS_FRAGMENT
#ifdef MOTION_VECTOR_PREPASS
    motion_vector: vec2<f32>,
#endif
#endif
}

/// Load the visibility buffer texture and resolve it into a VertexOutput.
fn resolve_vertex_output(frag_coord: vec4<f32>) -> VertexOutput {
    let packed_ids = u32(textureLoad(meshlet_visibility_buffer, vec2<u32>(frag_coord.xy)).r);
    let cluster_id = packed_ids >> 7u;
    let instanced_offset = meshlet_raster_clusters[cluster_id];
    let meshlet_id = instanced_offset.offset;
    var meshlet = meshlets[meshlet_id];

    let triangle_id = extractBits(packed_ids, 0u, 7u);
    let index_ids = meshlet.start_index_id + (triangle_id * 3u) + vec3(0u, 1u, 2u);
    let vertex_ids = vec3(get_meshlet_vertex_id(index_ids[0]), get_meshlet_vertex_id(index_ids[1]), get_meshlet_vertex_id(index_ids[2]));
    let vertex_0 = load_vertex(&meshlet, vertex_ids[0]);
    let vertex_1 = load_vertex(&meshlet, vertex_ids[1]);
    let vertex_2 = load_vertex(&meshlet, vertex_ids[2]);

    let instance_id = instanced_offset.instance_id;
    var instance_uniform = meshlet_instance_uniforms[instance_id];

    let world_from_local = affine3_to_square(instance_uniform.world_from_local);
    let world_position_0 = mesh_position_local_to_world(world_from_local, vec4(vertex_0.position, 1.0));
    let world_position_1 = mesh_position_local_to_world(world_from_local, vec4(vertex_1.position, 1.0));
    let world_position_2 = mesh_position_local_to_world(world_from_local, vec4(vertex_2.position, 1.0));

    let frag_coord_ndc = frag_coord_to_ndc(frag_coord).xy;
    let partial_derivatives = compute_partial_derivatives(
        array(world_position_0, world_position_1, world_position_2),
        frag_coord_ndc,
        view.viewport.zw / 2.0,
    );

    let world_position = mat3x4(world_position_0, world_position_1, world_position_2) * partial_derivatives.barycentrics;
    let world_positions_camera_relative = mat3x3(
        world_position_0.xyz - view.world_position,
        world_position_1.xyz - view.world_position,
        world_position_2.xyz - view.world_position,
    );
    let ddx_world_position = world_positions_camera_relative * partial_derivatives.ddx;
    let ddy_world_position = world_positions_camera_relative * partial_derivatives.ddy;

    let world_normal = mat3x3(
        normal_local_to_world(vertex_0.normal, &instance_uniform),
        normal_local_to_world(vertex_1.normal, &instance_uniform),
        normal_local_to_world(vertex_2.normal, &instance_uniform),
    ) * partial_derivatives.barycentrics;

    let uv = mat3x2(vertex_0.uv, vertex_1.uv, vertex_2.uv) * partial_derivatives.barycentrics;
    let ddx_uv = mat3x2(vertex_0.uv, vertex_1.uv, vertex_2.uv) * partial_derivatives.ddx;
    let ddy_uv = mat3x2(vertex_0.uv, vertex_1.uv, vertex_2.uv) * partial_derivatives.ddy;

    let world_tangent = calculate_world_tangent(world_normal, ddx_world_position, ddy_world_position, ddx_uv, ddy_uv);

#ifdef PREPASS_FRAGMENT
#ifdef MOTION_VECTOR_PREPASS
    let previous_world_from_local = affine3_to_square(instance_uniform.previous_world_from_local);
    let previous_world_position_0 = mesh_position_local_to_world(previous_world_from_local, vec4(vertex_0.position, 1.0));
    let previous_world_position_1 = mesh_position_local_to_world(previous_world_from_local, vec4(vertex_1.position, 1.0));
    let previous_world_position_2 = mesh_position_local_to_world(previous_world_from_local, vec4(vertex_2.position, 1.0));
    let previous_world_position = mat3x4(previous_world_position_0, previous_world_position_1, previous_world_position_2) * partial_derivatives.barycentrics;
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
        instance_id ^ meshlet_id,
        instance_uniform.material_and_lightmap_bind_group_slot & 0xffffu,
#ifdef PREPASS_FRAGMENT
#ifdef MOTION_VECTOR_PREPASS
        motion_vector,
#endif
#endif
    );
}

struct MeshletVertex {
    position: vec3<f32>,
    normal: vec3<f32>,
    uv: vec2<f32>,
}

fn load_vertex(meshlet: ptr<function, Meshlet>, vertex_id: u32) -> MeshletVertex {
    return MeshletVertex(
        get_meshlet_vertex_position(meshlet, vertex_id),
        get_meshlet_vertex_normal(meshlet, vertex_id),
        get_meshlet_vertex_uv(meshlet, vertex_id),
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

// https://www.jeremyong.com/graphics/2023/12/16/surface-gradient-bump-mapping/#surface-gradient-from-a-tangent-space-normal-vector-without-an-explicit-tangent-basis
fn calculate_world_tangent(
    world_normal: vec3<f32>,
    ddx_world_position: vec3<f32>,
    ddy_world_position: vec3<f32>,
    ddx_uv: vec2<f32>,
    ddy_uv: vec2<f32>,
) -> vec4<f32> {
    // Project the position gradients onto the tangent plane
    let ddx_world_position_s = ddx_world_position - dot(ddx_world_position, world_normal) * world_normal;
    let ddy_world_position_s = ddy_world_position - dot(ddy_world_position, world_normal) * world_normal;

    // Compute the jacobian matrix to leverage the chain rule
    let jacobian_sign = sign(ddx_uv.x * ddy_uv.y - ddx_uv.y * ddy_uv.x);

    var world_tangent = jacobian_sign * (ddy_uv.y * ddx_world_position_s - ddx_uv.y * ddy_world_position_s);

    // The sign intrinsic returns 0 if the argument is 0
    if jacobian_sign != 0.0 {
        world_tangent = normalize(world_tangent);
    }

    // The second factor here ensures a consistent handedness between
    // the tangent frame and surface basis w.r.t. screenspace.
    let w = jacobian_sign * sign(dot(ddy_world_position, cross(world_normal, ddx_world_position)));

    return vec4(world_tangent, -w); // TODO: Unclear why we need to negate this to match mikktspace generated tangents
}
#endif
