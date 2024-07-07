#import bevy_pbr::meshlet_bindings::{
    meshlet_cluster_meshlet_ids,
    meshlet_bounding_spheres,
    meshlet_cluster_instance_ids,
    meshlet_instance_uniforms,
    meshlet_second_pass_candidates,
    depth_pyramid,
    view,
    previous_view,
    should_cull_instance,
    cluster_is_second_pass_candidate,
    meshlets,
    draw_indirect_args,
    draw_triangle_buffer,
}
#import bevy_render::maths::affine3_to_square

/// Culls individual clusters (1 per thread) in two passes (two pass occlusion culling), and outputs a bitmask of which clusters survived.
/// 1. The first pass tests instance visibility, frustum culling, LOD selection, and finally occlusion culling using last frame's depth pyramid.
/// 2. The second pass performs occlusion culling (using the depth buffer generated from the first pass) on all clusters that passed
///    the instance, frustum, and LOD tests in the first pass, but were not visible last frame according to the occlusion culling.

@compute
@workgroup_size(128, 1, 1) // 128 threads per workgroup, 1 cluster per thread
fn cull_clusters(
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
    @builtin(local_invocation_id) local_invocation_id: vec3<u32>,
) {
    // Calculate the cluster ID for this thread
    let cluster_id = local_invocation_id.x + 128u * dot(workgroup_id, vec3(num_workgroups.x * num_workgroups.x, num_workgroups.x, 1u));
    if cluster_id >= arrayLength(&meshlet_cluster_meshlet_ids) { return; }

#ifdef MESHLET_SECOND_CULLING_PASS
    if !cluster_is_second_pass_candidate(cluster_id) { return; }
#endif

    // Check for instance culling
    let instance_id = meshlet_cluster_instance_ids[cluster_id];
#ifdef MESHLET_FIRST_CULLING_PASS
    if should_cull_instance(instance_id) { return; }
#endif

    // Calculate world-space culling bounding sphere for the cluster
    let instance_uniform = meshlet_instance_uniforms[instance_id];
    let meshlet_id = meshlet_cluster_meshlet_ids[cluster_id];
    let world_from_local = affine3_to_square(instance_uniform.world_from_local);
    let world_scale = max(length(world_from_local[0]), max(length(world_from_local[1]), length(world_from_local[2])));
    let bounding_spheres = meshlet_bounding_spheres[meshlet_id];
    var culling_bounding_sphere_center = world_from_local * vec4(bounding_spheres.self_culling.center, 1.0);
    var culling_bounding_sphere_radius = world_scale * bounding_spheres.self_culling.radius;

#ifdef MESHLET_FIRST_CULLING_PASS
    // Frustum culling
    // TODO: Faster method from https://vkguide.dev/docs/gpudriven/compute_culling/#frustum-culling-function
    for (var i = 0u; i < 6u; i++) {
        if dot(view.frustum[i], culling_bounding_sphere_center) + culling_bounding_sphere_radius <= 0.0 {
            return;
        }
    }

    // Calculate view-space LOD bounding sphere for the meshlet
    let lod_bounding_sphere_center = world_from_local * vec4(bounding_spheres.self_lod.center, 1.0);
    let lod_bounding_sphere_radius = world_scale * bounding_spheres.self_lod.radius;
    let lod_bounding_sphere_center_view_space = (view.view_from_world * vec4(lod_bounding_sphere_center.xyz, 1.0)).xyz;

    // Calculate view-space LOD bounding sphere for the meshlet's parent
    let parent_lod_bounding_sphere_center = world_from_local * vec4(bounding_spheres.parent_lod.center, 1.0);
    let parent_lod_bounding_sphere_radius = world_scale * bounding_spheres.parent_lod.radius;
    let parent_lod_bounding_sphere_center_view_space = (view.view_from_world * vec4(parent_lod_bounding_sphere_center.xyz, 1.0)).xyz;

    // Check LOD cut (meshlet error imperceptible, and parent error not imperceptible)
    let lod_is_ok = lod_error_is_imperceptible(lod_bounding_sphere_center_view_space, lod_bounding_sphere_radius);
    let parent_lod_is_ok = lod_error_is_imperceptible(parent_lod_bounding_sphere_center_view_space, parent_lod_bounding_sphere_radius);
    if !lod_is_ok || parent_lod_is_ok { return; }
#endif

    // Project the culling bounding sphere to view-space for occlusion culling
#ifdef MESHLET_FIRST_CULLING_PASS
    let previous_world_from_local = affine3_to_square(instance_uniform.previous_world_from_local);
    let previous_world_from_local_scale = max(length(previous_world_from_local[0]), max(length(previous_world_from_local[1]), length(previous_world_from_local[2])));
    culling_bounding_sphere_center = previous_world_from_local * vec4(bounding_spheres.self_culling.center, 1.0);
    culling_bounding_sphere_radius = previous_world_from_local_scale * bounding_spheres.self_culling.radius;
#endif
    let culling_bounding_sphere_center_view_space = (view.view_from_world * vec4(culling_bounding_sphere_center.xyz, 1.0)).xyz;

    let aabb = project_view_space_sphere_to_screen_space_aabb(culling_bounding_sphere_center_view_space, culling_bounding_sphere_radius);
    let depth_pyramid_size_mip_0 = vec2<f32>(textureDimensions(depth_pyramid, 0));
    let width = (aabb.z - aabb.x) * depth_pyramid_size_mip_0.x;
    let height = (aabb.w - aabb.y) * depth_pyramid_size_mip_0.y;
    let depth_level = max(0, i32(ceil(log2(max(width, height))))); // TODO: Naga doesn't like this being a u32
    let depth_pyramid_size = vec2<f32>(textureDimensions(depth_pyramid, depth_level));
    let aabb_top_left = vec2<u32>(aabb.xy * depth_pyramid_size);

    let depth_quad_a = textureLoad(depth_pyramid, aabb_top_left, depth_level).x;
    let depth_quad_b = textureLoad(depth_pyramid, aabb_top_left + vec2(1u, 0u), depth_level).x;
    let depth_quad_c = textureLoad(depth_pyramid, aabb_top_left + vec2(0u, 1u), depth_level).x;
    let depth_quad_d = textureLoad(depth_pyramid, aabb_top_left + vec2(1u, 1u), depth_level).x;
    let occluder_depth = min(min(depth_quad_a, depth_quad_b), min(depth_quad_c, depth_quad_d));

    // Check whether or not the cluster would be occluded if drawn
    var cluster_visible: bool;
    if view.clip_from_view[3][3] == 1.0 {
        // Orthographic
        let sphere_depth = view.clip_from_view[3][2] + (culling_bounding_sphere_center_view_space.z + culling_bounding_sphere_radius) * view.clip_from_view[2][2];
        cluster_visible = sphere_depth >= occluder_depth;
    } else {
        // Perspective
        let sphere_depth = -view.clip_from_view[3][2] / (culling_bounding_sphere_center_view_space.z + culling_bounding_sphere_radius);
        cluster_visible = sphere_depth >= occluder_depth;
    }

    // Write if the cluster should be occlusion tested in the second pass
#ifdef MESHLET_FIRST_CULLING_PASS
    if !cluster_visible {
        let bit = 1u << cluster_id % 32u;
        atomicOr(&meshlet_second_pass_candidates[cluster_id / 32u], bit);
    }
#endif

    // Append a list of this cluster's triangles to draw if not culled
    if cluster_visible {
        let meshlet_triangle_count = meshlets[meshlet_id].triangle_count;
        let buffer_start = atomicAdd(&draw_indirect_args.vertex_count, meshlet_triangle_count * 3u) / 3u;
        let cluster_id_packed = cluster_id << 6u;
        for (var triangle_id = 0u; triangle_id < meshlet_triangle_count; triangle_id++) {
            draw_triangle_buffer[buffer_start + triangle_id] = cluster_id_packed | triangle_id;
        }
    }
}

// https://stackoverflow.com/questions/21648630/radius-of-projected-sphere-in-screen-space/21649403#21649403
fn lod_error_is_imperceptible(cp: vec3<f32>, r: f32) -> bool {
    let d2 = dot(cp, cp);
    let r2 = r * r;
    let sphere_diameter_uv = view.clip_from_view[0][0] * r / sqrt(d2 - r2);
    let view_size = f32(max(view.viewport.z, view.viewport.w));
    let sphere_diameter_pixels = sphere_diameter_uv * view_size;
    return sphere_diameter_pixels < 1.0;
}

// https://zeux.io/2023/01/12/approximate-projected-bounds
fn project_view_space_sphere_to_screen_space_aabb(cp: vec3<f32>, r: f32) -> vec4<f32> {
    let inv_width = view.clip_from_view[0][0] * 0.5;
    let inv_height = view.clip_from_view[1][1] * 0.5;
    if view.clip_from_view[3][3] == 1.0 {
        // Orthographic
        let min_x = cp.x - r;
        let max_x = cp.x + r;

        let min_y = cp.y - r;
        let max_y = cp.y + r;

        return vec4(min_x * inv_width, 1.0 - max_y * inv_height, max_x * inv_width, 1.0 - min_y * inv_height);
    } else {
        // Perspective
        let c = vec3(cp.xy, -cp.z);
        let cr = c * r;
        let czr2 = c.z * c.z - r * r;

        let vx = sqrt(c.x * c.x + czr2);
        let min_x = (vx * c.x - cr.z) / (vx * c.z + cr.x);
        let max_x = (vx * c.x + cr.z) / (vx * c.z - cr.x);

        let vy = sqrt(c.y * c.y + czr2);
        let min_y = (vy * c.y - cr.z) / (vy * c.z + cr.y);
        let max_y = (vy * c.y + cr.z) / (vy * c.z - cr.y);

        return vec4(min_x * inv_width, -max_y * inv_height, max_x * inv_width, -min_y * inv_height) + vec4(0.5);
    }
}
