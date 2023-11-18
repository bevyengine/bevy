#import bevy_pbr::meshlet_bindings::{
    meshlet_previous_thread_ids,
    meshlet_previous_occlusion,
    meshlet_occlusion,
    meshlet_thread_meshlet_ids,
    meshlets,
    draw_command_buffer,
    draw_index_buffer,
    meshlet_thread_instance_ids,
    meshlet_instance_uniforms,
    meshlet_bounding_spheres,
    view,
}
#ifdef MESHLET_SECOND_CULLING_PASS
#import bevy_pbr::meshlet_bindings::{depth_pyramid, depth_pyramid_sampler}
#endif
#import bevy_render::maths::affine_to_square

@compute
@workgroup_size(128, 1, 1)
fn cull_meshlets(@builtin(global_invocation_id) thread_id: vec3<u32>) {
    if thread_id.x >= arrayLength(&meshlet_thread_meshlet_ids) { return; }
    let meshlet_id = meshlet_thread_meshlet_ids[thread_id.x];
    let bounding_sphere = meshlet_bounding_spheres[meshlet_id];

    let instance_id = meshlet_thread_instance_ids[thread_id.x];
    let instance_uniform = meshlet_instance_uniforms[instance_id];
    let model = affine_to_square(instance_uniform.model);
    let model_scale = max(length(model[0]), max(length(model[1]), length(model[2])));
    let bounding_sphere_center = model * vec4(bounding_sphere.center, 1.0);
    let bounding_sphere_radius = model_scale * -bounding_sphere.radius;

#ifdef MESHLET_SECOND_CULLING_PASS
    var meshlet_visible = true;
#else
    let previous_thread_id = meshlet_previous_thread_ids[thread_id.x];
    var meshlet_visible = bool(meshlet_previous_occlusion[previous_thread_id]);
#endif

    // TODO: Faster method from https://vkguide.dev/docs/gpudriven/compute_culling/#frustum-culling-function
    for (var i = 0u; i < 6u; i++) {
        meshlet_visible &= dot(view.frustum[i], bounding_sphere_center) > bounding_sphere_radius;
        if !meshlet_visible { break; }
    }

#ifdef MESHLET_SECOND_CULLING_PASS
    var aabb: vec4<f32>;
    if project_sphere(bounding_sphere_center.xyz, bounding_sphere_radius, &aabb) {
        let depth_pyramid_size = vec2<f32>(textureDimensions(depth_pyramid));
        let width = (aabb.z - aabb.x) * depth_pyramid_size.x;
        let height = (aabb.w - aabb.y) * depth_pyramid_size.y;
        let level = floor(log2(max(width, height)));

        let depth = textureSampleLevel(depth_pyramid, depth_pyramid_sampler, (aabb.xy + aabb.zw) * 0.5, level).x;
        let sphere_depth = view.projection[3][2] / (bounding_sphere_center.z - bounding_sphere_radius);

        meshlet_visible &= sphere_depth >= depth;
    }
#endif

    if meshlet_visible {
        let meshlet = meshlets[meshlet_id];
        let draw_index_buffer_start = atomicAdd(&draw_command_buffer.vertex_count, meshlet.index_count);
        let packed_thread_id = thread_id.x << 8u;
        for (var index_id = 0u; index_id < meshlet.index_count; index_id++) {
            draw_index_buffer[draw_index_buffer_start + index_id] = packed_thread_id | index_id;
        }
    }

#ifdef MESHLET_SECOND_CULLING_PASS
    meshlet_occlusion[thread_id.x] = u32(meshlet_visible);
#endif
}

// 2D Polyhedral Bounds of a Clipped, Perspective-Projected 3D Sphere
// https://jcgt.org/published/0002/02/05/paper.pdf
fn project_sphere(c: vec3<f32>, r: f32, aabb_out: ptr<function, vec4<f32>>) -> bool {
    if c.z < r + view.projection[3][2] {
        return false;
    }

    let cr = c * r;
    let czr2 = c.x * c.z -r * r;

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
