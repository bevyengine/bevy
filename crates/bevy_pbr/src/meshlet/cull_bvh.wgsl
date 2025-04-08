#import bevy_pbr::meshlet_bindings::{
    get_aabb,
    get_aabb_error,
    get_aabb_child_offset,
    constants,
    meshlet_bvh_nodes,
    meshlet_bvh_cull_count_read,
    meshlet_bvh_cull_count_write,
    meshlet_bvh_cull_dispatch,
    meshlet_bvh_cull_queue,
    meshlet_meshlet_cull_count_early,
    meshlet_meshlet_cull_count_late,
    meshlet_meshlet_cull_dispatch_early,
    meshlet_meshlet_cull_dispatch_late,
    meshlet_meshlet_cull_queue,
    meshlet_second_pass_bvh_count,
    meshlet_second_pass_bvh_dispatch,
    meshlet_second_pass_bvh_queue,
}
#import bevy_pbr::meshlet_cull_shared::{
    lod_error_is_imperceptible,
    aabb_in_frustum,
    should_occlusion_cull_aabb,
    push_bvh,
    push_meshlets,
}

@compute
@workgroup_size(128, 1, 1) // 128 threads per workgroup, 1 instance per thread
fn cull_bvh(
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
    @builtin(local_invocation_index) local_invocation_index: u32,
) {
    // Calculate the queue ID for this thread
    let dispatch_id = local_invocation_index + 128u * dot(workgroup_id, vec3(num_workgroups.x * num_workgroups.x, num_workgroups.x, 1u));
    var node = dispatch_id >> 3u;
    let subnode = dispatch_id & 7u;
    if node >= meshlet_bvh_cull_count_read { return; }
    
    node = select(node, constants.rightmost_slot - node, constants.read_from_front == 0u);
    let instanced_offset = meshlet_bvh_cull_queue[node];
    let instance_id = instanced_offset.instance_id;
    let aabb_error_offset = meshlet_bvh_nodes[node].aabbs[subnode];
    let aabb = get_aabb(&aabb_error_offset);
    let parent_error = get_aabb_error(&aabb_error_offset);
    let lod_sphere = meshlet_bvh_nodes[node].lod_spheres[subnode];

    let parent_is_imperceptible = lod_error_is_imperceptible(lod_sphere, parent_error, instance_id);
    // Error and frustum cull, in both passes
    if parent_is_imperceptible || !aabb_in_frustum(aabb, instance_id) { return; }

    let child_offset = get_aabb_child_offset(&aabb_error_offset);    
    let index = subnode >> 2u;
    let bit_offset = subnode & 3u;
    let packed_child_count = meshlet_bvh_nodes[node].child_counts[index];
    let child_count = extractBits(packed_child_count, bit_offset * 8u, 8u);
    // If we pass, try occlusion culling
    // If this node was occluded, push it's children to the second pass to check against this frame's HZB
    if occlusion_cull_aabb(aabb, instance_id) {
#ifdef MESHLET_FIRST_CULLING_PASS
        if child_count == 255u {
            push_bvh(
                &meshlet_second_pass_bvh_count,
                &meshlet_second_pass_bvh_dispatch,
                &meshlet_second_pass_bvh_queue,
                InstancedOffset(instance_id, child_offset),
                0u, constants.rightmost_slot
            );
        } else {
            push_meshlets(
                &meshlet_meshlet_cull_count_late,
                &meshlet_meshlet_cull_dispatch_late,
                &meshlet_meshlet_cull_queue,
                InstancedOffset(instance_id, child_offset),
                child_count,
                1u, constants.rightmost_slot,
            );
        }
#endif
        return;
    }

    // If we pass, push the children to the next BVH cull
    if child_count == 255u {
        push_bvh(
            &meshlet_bvh_cull_count_write,
            &meshlet_bvh_cull_dispatch,
            &meshlet_bvh_cull_queue,
            InstancedOffset(instance_id, child_offset),
            constants.read_from_front, constants.rightmost_slot,
        );
    } else {
        push_meshlets(
            &meshlet_meshlet_cull_count_early,
            &meshlet_meshlet_cull_dispatch_early,
            &meshlet_meshlet_cull_queue,
            InstancedOffset(instance_id, child_offset),
            child_count,
            0u, constants.rightmost_slot,
        );
    }
}
