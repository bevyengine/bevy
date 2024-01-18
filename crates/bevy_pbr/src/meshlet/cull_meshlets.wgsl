#import bevy_pbr::meshlet_bindings::{
    meshlet_thread_meshlet_ids,
    meshlet_bounding_spheres,
    meshlet_thread_instance_ids,
    meshlet_instance_uniforms,
    meshlet_occlusion,
    view,
    get_meshlet_previous_occlusion,
}
#ifdef MESHLET_SECOND_CULLING_PASS
#import bevy_pbr::meshlet_bindings::depth_pyramid
#endif
#import bevy_render::maths::affine_to_square

/// Culls individual meshlets (1 per thread) in two passes (two pass occlusion culling), and outputs a bitmask of which meshlets survived.
/// 1. The first pass is only frustum culling, on only the meshlets that were visible last frame.
/// 2. The second pass performs both frustum and occlusion culling (using the depth buffer generated from the first pass), on all meshlets.

@compute
@workgroup_size(128, 1, 1)
fn cull_meshlets(@builtin(global_invocation_id) thread_id: vec3<u32>) {
    // Fetch the instanced meshlet data
    if thread_id.x >= arrayLength(&meshlet_thread_meshlet_ids) { return; }
    let meshlet_id = meshlet_thread_meshlet_ids[thread_id.x];
    let bounding_sphere = meshlet_bounding_spheres[meshlet_id];
    let instance_id = meshlet_thread_instance_ids[thread_id.x];
    let instance_uniform = meshlet_instance_uniforms[instance_id];
    let model = affine_to_square(instance_uniform.model);
    let model_scale = max(length(model[0]), max(length(model[1]), length(model[2])));
    let bounding_sphere_center = model * vec4(bounding_sphere.center, 1.0);
    let bounding_sphere_radius = model_scale * bounding_sphere.radius;

    // In the first pass, operate only on the meshlets visible last frame. In the second pass, operate on all meshlets.
#ifdef MESHLET_SECOND_CULLING_PASS
    var meshlet_visible = true;
#else
    var meshlet_visible = get_meshlet_previous_occlusion(thread_id.x);
    if !meshlet_visible { return; }
#endif

    // Frustum culling
    // TODO: Faster method from https://vkguide.dev/docs/gpudriven/compute_culling/#frustum-culling-function
    for (var i = 0u; i < 6u; i++) {
        if !meshlet_visible { break; }
        meshlet_visible &= dot(view.frustum[i], bounding_sphere_center) > -bounding_sphere_radius;
    }

#ifdef MESHLET_SECOND_CULLING_PASS
    // In the second culling pass, cull against the depth pyramid generated from the first pass
    var aabb: vec4<f32>;
    let bounding_sphere_center_view_space = (view.inverse_view * vec4(bounding_sphere_center.xyz, 1.0)).xyz;
    if meshlet_visible && try_project_sphere(bounding_sphere_center_view_space, bounding_sphere_radius, &aabb) {
        let depth_pyramid_size = vec2<f32>(textureDimensions(depth_pyramid));
        let width = (aabb.z - aabb.x) * depth_pyramid_size.x;
        let height = (aabb.w - aabb.y) * depth_pyramid_size.y;
        let depth_level = i32(ceil(log2(max(width, height)))); // TODO: Naga dosen't like this being a u32
        let aabb_top_left = vec2<u32>(aabb.xy * depth_pyramid_size);

        let depth_quad_a = textureLoad(depth_pyramid, aabb_top_left, depth_level).x;
        let depth_quad_b = textureLoad(depth_pyramid, aabb_top_left + vec2(1u, 0u), depth_level).x;
        let depth_quad_c = textureLoad(depth_pyramid, aabb_top_left + vec2(0u, 1u), depth_level).x;
        let depth_quad_d = textureLoad(depth_pyramid, aabb_top_left + vec2(1u, 1u), depth_level).x;
        let occluder_depth = min(min(depth_quad_a, depth_quad_b), min(depth_quad_c, depth_quad_d));

        let sphere_depth = -view.projection[3][2] / (bounding_sphere_center_view_space.z + bounding_sphere_radius);
        meshlet_visible &= sphere_depth >= occluder_depth;
    }
#endif

    // Write the bitmask of whether or not the meshlet was culled
    let occlusion_bit = u32(meshlet_visible) << (thread_id.x % 32u);
    atomicOr(&meshlet_occlusion[thread_id.x / 32u], occlusion_bit);
}

// https://zeux.io/2023/01/12/approximate-projected-bounds
fn try_project_sphere(cp: vec3<f32>, r: f32, aabb_out: ptr<function, vec4<f32>>) -> bool {
    let c = vec3(cp.xy, -cp.z);

    if c.z < r + view.projection[3][2] {
        return false;
    }

    let cr = c * r;
    let czr2 = c.z * c.z - r * r;

    let vx = sqrt(c.x * c.x + czr2);
    let min_x = (vx * c.x - cr.z) / (vx * c.z + cr.x);
    let max_x = (vx * c.x + cr.z) / (vx * c.z - cr.x);

    let vy = sqrt(c.y * c.y + czr2);
    let min_y = (vy * c.y - cr.z) / (vy * c.z + cr.y);
    let max_y = (vy * c.y + cr.z) / (vy * c.z - cr.y);

    let p00 = view.projection[0][0];
    let p11 = view.projection[1][1];

    var aabb = vec4(min_x * p00, min_y * p11, max_x * p00, max_y * p11);
    aabb = aabb.xwzy * vec4(0.5, -0.5, 0.5, -0.5) + vec4(0.5);

    *aabb_out = aabb;
    return true;
}
