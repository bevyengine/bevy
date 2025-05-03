#import bevy_pbr::meshlet_bindings::{
    InstancedOffset,
    get_aabb,
    get_aabb_error,
    constants,
    view,
    meshlet_cull_data,
    meshlet_software_raster_indirect_args,
    meshlet_hardware_raster_indirect_args,
    meshlet_previous_raster_counts,
    meshlet_raster_clusters,
    meshlet_meshlet_cull_count_read,
    meshlet_meshlet_cull_count_write,
    meshlet_meshlet_cull_dispatch,
    meshlet_meshlet_cull_queue,
}
#import bevy_pbr::meshlet_cull_shared::{
    lod_error_is_imperceptible,
    aabb_in_frustum,
    should_occlusion_cull_aabb,
}

@compute
@workgroup_size(128, 1, 1) // 1 cluster per thread
fn cull_clusters(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    if global_invocation_id.x >= meshlet_meshlet_cull_count_read { return; }

#ifdef MESHLET_FIRST_CULLING_PASS
    let meshlet_id = global_invocation_id.x;
#else
    let meshlet_id = constants.rightmost_slot - global_invocation_id.x;
#endif
    let instanced_offset = meshlet_meshlet_cull_queue[meshlet_id];
    let instance_id = instanced_offset.instance_id;
    let cull_data = &meshlet_cull_data[instanced_offset.offset];
    var aabb_error_offset = (*cull_data).aabb;
    let aabb = get_aabb(&aabb_error_offset);
    let error = get_aabb_error(&aabb_error_offset);
    let lod_sphere = (*cull_data).lod_group_sphere;

    let is_imperceptible = lod_error_is_imperceptible(lod_sphere, error, instance_id);
    // Error and frustum cull, in both passes
    if !is_imperceptible || !aabb_in_frustum(aabb, instance_id) { return; }

    // If we pass, try occlusion culling
    // If this node was occluded, push it's children to the second pass to check against this frame's HZB
    if should_occlusion_cull_aabb(aabb, instance_id) {
#ifdef MESHLET_FIRST_CULLING_PASS
        let id = atomicAdd(&meshlet_meshlet_cull_count_write, 1u);
        let value = InstancedOffset(instance_id, instanced_offset.offset);
        meshlet_meshlet_cull_queue[constants.rightmost_slot - id] = value;
        if ((id & 127u) == 0) {
            atomicAdd(&meshlet_meshlet_cull_dispatch.x, 1u);
        }
#endif
        return;
    }

    // If we pass, rasterize the meshlet
    // Check how big the cluster is in screen space
    // let culling_bounding_sphere_center_view_space = (view.view_from_world * vec4(culling_bounding_sphere_center.xyz, 1.0)).xyz;
    // aabb = project_view_space_sphere_to_screen_space_aabb(culling_bounding_sphere_center_view_space, culling_bounding_sphere_radius);
    // let aabb_width_pixels = (aabb.z - aabb.x) * view.viewport.z;
    // let aabb_height_pixels = (aabb.w - aabb.y) * view.viewport.w;
    // let cluster_is_small = all(vec2(aabb_width_pixels, aabb_height_pixels) < vec2(64.0));
    //
    // // Let the hardware rasterizer handle near-plane clipping
    // let not_intersects_near_plane = dot(view.frustum[4u], culling_bounding_sphere_center) > culling_bounding_sphere_radius;

    var buffer_slot: u32;
    // if cluster_is_small && not_intersects_near_plane {
    if false {
        // Append this cluster to the list for software rasterization
        buffer_slot = atomicAdd(&meshlet_software_raster_indirect_args.x, 1u);
        buffer_slot += meshlet_previous_raster_counts[0];
    } else {
        // Append this cluster to the list for hardware rasterization
        buffer_slot = atomicAdd(&meshlet_hardware_raster_indirect_args.instance_count, 1u);
        buffer_slot += meshlet_previous_raster_counts[1];
        buffer_slot = constants.rightmost_slot - buffer_slot;
    }
    meshlet_raster_clusters[buffer_slot] = InstancedOffset(instance_id, instanced_offset.offset);
}
