#import bevy_pbr::meshlet_bindings::{
    InstancedOffset,
    constants,
    meshlet_view_instance_visibility,
    meshlet_instance_aabbs,
    meshlet_instance_bvh_root_nodes,
    meshlet_bvh_cull_count_write,
    meshlet_bvh_cull_dispatch,
    meshlet_bvh_cull_queue,
    meshlet_second_pass_instance_count,
    meshlet_second_pass_instance_dispatch,
    meshlet_second_pass_instance_candidates,
}
#import bevy_pbr::meshlet_cull_shared::{
    aabb_in_frustum,
    should_occlusion_cull_aabb,
}

fn instance_count() -> u32 {
#ifdef MESHLET_FIRST_CULLING_PASS
    return constants.scene_instance_count;
#else
    return meshlet_second_pass_instance_count;
#endif
}

fn map_instance_id(id: u32) -> u32 {
#ifdef MESHLET_FIRST_CULLING_PASS
    return id;
#else
    return meshlet_second_pass_instance_candidates[id];
#endif
}

fn should_cull_instance(instance_id: u32) -> bool {
    let bit_offset = instance_id >> 5u;
    let packed_visibility = meshlet_view_instance_visibility[instance_id & 31u];
    return bool(extractBits(packed_visibility, bit_offset, 1u));
}

@compute
@workgroup_size(128, 1, 1) // 1 instance per thread
fn cull_instances(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    // Calculate the instance ID for this thread
    let dispatch_id = global_invocation_id.x;
    if dispatch_id >= instance_count() { return; }

    let instance_id = map_instance_id(dispatch_id);
    let aabb = meshlet_instance_aabbs[instance_id];

    // Visibility and frustum cull, but only in the first pass
#ifdef MESHLET_FIRST_CULLING_PASS
    if should_cull_instance(instance_id) || !aabb_in_frustum(aabb, instance_id) { return; }
#endif

    // If we pass, try occlusion culling
    // If this instance was occluded, push it to the second pass to check against this frame's HZB
    if should_occlusion_cull_aabb(aabb, instance_id) {
#ifdef MESHLET_FIRST_CULLING_PASS
        let id = atomicAdd(&meshlet_second_pass_instance_count, 1u);
        meshlet_second_pass_instance_candidates[id] = instance_id;
        if ((id & 127u) == 0u) {
            atomicAdd(&meshlet_second_pass_instance_dispatch.x, 1u);
        }
#endif
        return;
    }

    // If we pass, push the instance's root node to BVH cull
    let root_node = meshlet_instance_bvh_root_nodes[instance_id];
    let id = atomicAdd(&meshlet_bvh_cull_count_write, 1u);
    meshlet_bvh_cull_queue[id] = InstancedOffset(instance_id, root_node);
    if ((id & 15u) == 0u) {
        atomicAdd(&meshlet_bvh_cull_dispatch.x, 1u);
    }
}
