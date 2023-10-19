#import bevy_pbr::meshlet_bindings meshlet_thread_meshlet_ids, meshlets, get_meshlet_index, draw_command_buffer, draw_index_buffer, meshlet_thread_instance_ids, meshlet_instance_uniforms, meshlet_bounding_spheres, view
#import bevy_render::maths affine_to_square

@compute
@workgroup_size(128, 1, 1)
fn cull_meshlets(@builtin(global_invocation_id) thread_id: vec3<u32>) {
    if thread_id.x >= arrayLength(&meshlet_thread_meshlet_ids) { return; }

    let meshlet_id = meshlet_thread_meshlet_ids[thread_id.x];
    let instance_id = meshlet_thread_instance_ids[thread_id.x];
    let instance_uniform = meshlet_instance_uniforms[instance_id];
    let model = affine_to_square(instance_uniform.model);

    var meshlet_visible = true;

    // TODO: Faster method from https://vkguide.dev/docs/gpudriven/compute_culling/#frustum-culling-function
    // TODO: Does using the mesh model for translation work here?
    // TODO: Maybe need to multiply radius by the max model scale?
    let bounding_sphere = meshlet_bounding_spheres[meshlet_id];
    let bounding_sphere_center = model * vec4(bounding_sphere.center, 1.0);
    for (var i = 0u; i < 6u; i++) {
        meshlet_visible &= dot(view.frustum[i], bounding_sphere_center) > -bounding_sphere.radius;
        if !meshlet_visible { break; }
    }

    if meshlet_visible {
        let meshlet = meshlets[meshlet_id];
        let meshlet_vertex_count = meshlet.triangle_count * 3u;
        let draw_index_buffer_start = atomicAdd(&draw_command_buffer.vertex_count, meshlet_vertex_count);

        let packed_thread_id = thread_id.x << 8u;
        for (var offset = 0u; offset < meshlet_vertex_count; offset++) {
            let index = get_meshlet_index(meshlet.start_index_id + offset);
            draw_index_buffer[draw_index_buffer_start + offset] = packed_thread_id | index;
        }
    }
}
