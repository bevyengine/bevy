#import bevy_pbr::meshlet_bindings::{
    meshlet_thread_meshlet_ids,
    meshlet_bounding_spheres,
    meshlet_thread_instance_ids,
    meshlet_instance_uniforms,
    meshlet_occlusion,
    view,
    should_cull_instance,
    get_meshlet_previous_occlusion,
}
#ifdef MESHLET_SECOND_CULLING_PASS
#import bevy_pbr::meshlet_bindings::depth_pyramid
#endif
#import bevy_render::maths::affine3_to_square

/// Culls individual clusters (1 per thread) in two passes (two pass occlusion culling), and outputs a bitmask of which clusters survived.
/// 1. The first pass is only frustum culling, on only the clusters that were visible last frame.
/// 2. The second pass performs both frustum and occlusion culling (using the depth buffer generated from the first pass), on all clusters.

@compute
@workgroup_size(128, 1, 1) // 128 threads per workgroup, 1 instanced meshlet per thread
fn cull_meshlets(@builtin(global_invocation_id) cluster_id: vec3<u32>) {
    // Fetch the instance data and check for instance culling
    if cluster_id.x >= arrayLength(&meshlet_thread_meshlet_ids) { return; }
    let instance_id = meshlet_thread_instance_ids[cluster_id.x];
    if should_cull_instance(instance_id) {
        return;
    }

    // Fetch other meshlet data
    let meshlet_id = meshlet_thread_meshlet_ids[cluster_id.x];
    let instance_uniform = meshlet_instance_uniforms[instance_id];
    let model = affine3_to_square(instance_uniform.model);
    let model_scale = max(length(model[0]), max(length(model[1]), length(model[2])));
    let bounding_spheres = meshlet_bounding_spheres[meshlet_id];

    // Calculate view-space LOD bounding sphere for the meshlet
    let lod_bounding_sphere_center = model * vec4(bounding_spheres.self_lod.center, 1.0);
    let lod_bounding_sphere_radius = model_scale * bounding_spheres.self_lod.radius;
    let lod_bounding_sphere_center_view_space = (view.inverse_view * vec4(lod_bounding_sphere_center.xyz, 1.0)).xyz;

    // Calculate view-space LOD bounding sphere for the meshlet's parent
    let parent_lod_bounding_sphere_center = model * vec4(bounding_spheres.parent_lod.center, 1.0);
    let parent_lod_bounding_sphere_radius = model_scale * bounding_spheres.parent_lod.radius;
    let parent_lod_bounding_sphere_center_view_space = (view.inverse_view * vec4(parent_lod_bounding_sphere_center.xyz, 1.0)).xyz;

    // Check LOD cut (meshlet error imperceptible, and parent error not imperceptible)
    let lod_is_ok = lod_error_is_imperceptible(lod_bounding_sphere_center_view_space, lod_bounding_sphere_radius);
    let parent_lod_is_ok = lod_error_is_imperceptible(parent_lod_bounding_sphere_center_view_space, parent_lod_bounding_sphere_radius);
    if !lod_is_ok || parent_lod_is_ok { return; }

    // In the first pass, operate only on the clusters visible last frame. In the second pass, operate on all clusters.
#ifdef MESHLET_SECOND_CULLING_PASS
    var meshlet_visible = true;
#else
    var meshlet_visible = get_meshlet_previous_occlusion(cluster_id.x);
    if !meshlet_visible { return; }
#endif

    // Calculate world-space culling bounding sphere for the cluster
    let culling_bounding_sphere_center = model * vec4(bounding_spheres.self_culling.center, 1.0);
    let culling_bounding_sphere_radius = model_scale * bounding_spheres.self_culling.radius;

    // Frustum culling
    // TODO: Faster method from https://vkguide.dev/docs/gpudriven/compute_culling/#frustum-culling-function
    for (var i = 0u; i < 6u; i++) {
        if !meshlet_visible { break; }
        meshlet_visible &= dot(view.frustum[i], culling_bounding_sphere_center) > -culling_bounding_sphere_radius;
    }

#ifdef MESHLET_SECOND_CULLING_PASS
    // In the second culling pass, cull against the depth pyramid generated from the first pass
    if meshlet_visible {
        let culling_bounding_sphere_center_view_space = (view.inverse_view * vec4(culling_bounding_sphere_center.xyz, 1.0)).xyz;
        let aabb = project_view_space_sphere_to_screen_space_aabb(culling_bounding_sphere_center_view_space, culling_bounding_sphere_radius);

        // Halve the AABB size because the first depth mip resampling pass cut the full screen resolution into a power of two conservatively
        let depth_pyramid_size_mip_0 = vec2<f32>(textureDimensions(depth_pyramid, 0)) * 0.5;
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
        if view.projection[3][3] == 1.0 {
            // Orthographic
            let sphere_depth = view.projection[3][2] + (culling_bounding_sphere_center_view_space.z + culling_bounding_sphere_radius) * view.projection[2][2];
            meshlet_visible &= sphere_depth >= occluder_depth;
        } else {
            // Perspective
            let sphere_depth = -view.projection[3][2] / (culling_bounding_sphere_center_view_space.z + culling_bounding_sphere_radius);
            meshlet_visible &= sphere_depth >= occluder_depth;
        }
    }
#endif

    // Write the bitmask of whether or not the cluster was culled
    let occlusion_bit = u32(meshlet_visible) << (cluster_id.x % 32u);
    atomicOr(&meshlet_occlusion[cluster_id.x / 32u], occlusion_bit);
}

// https://stackoverflow.com/questions/21648630/radius-of-projected-sphere-in-screen-space/21649403#21649403
fn lod_error_is_imperceptible(cp: vec3<f32>, r: f32) -> bool {
    let d2 = dot(cp, cp);
    let r2 = r * r;
    let sphere_diameter_uv = view.projection[0][0] * r / sqrt(d2 - r2);
    let view_size = f32(max(view.viewport.z, view.viewport.w));
    let sphere_diameter_pixels = sphere_diameter_uv * view_size;
    return sphere_diameter_pixels < 1.0;
}

// https://zeux.io/2023/01/12/approximate-projected-bounds
fn project_view_space_sphere_to_screen_space_aabb(cp: vec3<f32>, r: f32) -> vec4<f32> {
    let inv_width = view.projection[0][0] * 0.5;
    let inv_height = view.projection[1][1] * 0.5;
    if view.projection[3][3] == 1.0 {
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
