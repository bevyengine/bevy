#import bevy_pbr::meshlet_bindings::{
    InstancedOffset,
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
}

@compute
@workgroup_size(128, 1, 1) // 8 threads per node, 16 nodes per workgroup
fn cull_bvh(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    // Calculate the queue ID for this thread
    let dispatch_id = global_invocation_id.x;
    var node = dispatch_id >> 3u;
    let subnode = dispatch_id & 7u;
    if node >= meshlet_bvh_cull_count_read { return; }

    node = select(node, constants.rightmost_slot - node, constants.read_from_front == 0u);
    let instanced_offset = meshlet_bvh_cull_queue[node];
    let instance_id = instanced_offset.instance_id;
    let bvh_node = &meshlet_bvh_nodes[instanced_offset.offset];

    var aabb_error_offset = (*bvh_node).aabbs[subnode];
    let aabb = get_aabb(&aabb_error_offset);
    let parent_error = get_aabb_error(&aabb_error_offset);
    let lod_sphere = (*bvh_node).lod_bounds[subnode];

    let parent_is_imperceptible = lod_error_is_imperceptible(lod_sphere, parent_error, instance_id);
    // Error and frustum cull, in both passes
    if parent_is_imperceptible || !aabb_in_frustum(aabb, instance_id) { return; }

    let child_offset = get_aabb_child_offset(&aabb_error_offset);
    let index = subnode >> 2u;
    let bit_offset = subnode & 3u;
    let packed_child_count = (*bvh_node).child_counts[index];
    let child_count = extractBits(packed_child_count, bit_offset * 8u, 8u);
    var value = InstancedOffset(instance_id, child_offset);

    // If we pass, try occlusion culling
    // If this node was occluded, push it's children to the second pass to check against this frame's HZB
    if should_occlusion_cull_aabb(aabb, instance_id) {
#ifdef MESHLET_FIRST_CULLING_PASS
        if child_count == 255u {
            let id = atomicAdd(&meshlet_second_pass_bvh_count, 1u);
            meshlet_second_pass_bvh_queue[id] = value;
            if ((id & 15u) == 0u) {
                atomicAdd(&meshlet_second_pass_bvh_dispatch.x, 1u);
            }
        } else {
            let base = atomicAdd(&meshlet_meshlet_cull_count_late, child_count);
            let start = constants.rightmost_slot - base;
            for (var i = start; i < start - child_count; i--) {
                meshlet_meshlet_cull_queue[i] = value;
                value.offset += 1u;
            }
            let req = (base + child_count + 127u) >> 7u;
            atomicMax(&meshlet_meshlet_cull_dispatch_late.x, req);
        }
#endif
        return;
    }

    // If we pass, push the children to the next BVH cull
    if child_count == 255u {
        let id = atomicAdd(&meshlet_bvh_cull_count_write, 1u);
        let index = select(constants.rightmost_slot - id, id, constants.read_from_front == 0u);
        meshlet_bvh_cull_queue[index] = value;
        if ((id & 15u) == 0u) {
            atomicAdd(&meshlet_bvh_cull_dispatch.x, 1u);
        }
    } else {
#ifdef MESHLET_FIRST_CULLING_PASS
        let base = atomicAdd(&meshlet_meshlet_cull_count_early, child_count);
        let end = base + child_count;
        for (var i = base; i < end; i++) {
            meshlet_meshlet_cull_queue[i] = value;
            value.offset += 1u;
        }
        let req = (end + 127u) >> 7u;
        atomicMax(&meshlet_meshlet_cull_dispatch_early.x, req);
#else
        let base = atomicAdd(&meshlet_meshlet_cull_count_late, child_count);
        let start = constants.rightmost_slot - base;
        for (var i = start; i < start - child_count; i--) {
            meshlet_meshlet_cull_queue[i] = value;
            value.offset += 1u;
        }
        let req = (base + child_count + 127u) >> 7u;
        atomicMax(&meshlet_meshlet_cull_dispatch_late.x, req);
#endif
    }
}
