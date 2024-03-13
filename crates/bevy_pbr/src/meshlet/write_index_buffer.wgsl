#import bevy_pbr::meshlet_bindings::{
    meshlet_thread_meshlet_ids,
    meshlets,
    draw_command_buffer,
    draw_index_buffer,
    get_meshlet_occlusion,
    get_meshlet_previous_occlusion,
}

var<workgroup> draw_index_buffer_start_workgroup: u32;

/// This pass writes out a buffer of cluster + triangle IDs for the draw_indirect() call to rasterize each visible meshlet.

@compute
@workgroup_size(64, 1, 1) // 64 threads per workgroup, 1 workgroup per cluster, 1 thread per triangle
fn write_index_buffer(@builtin(workgroup_id) workgroup_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>, @builtin(local_invocation_index) triangle_id: u32) {
    // Calculate the cluster ID for this workgroup
    let cluster_id = dot(workgroup_id, vec3(num_workgroups.x * num_workgroups.x, num_workgroups.x, 1u));
    if cluster_id >= arrayLength(&meshlet_thread_meshlet_ids) { return; }

    // If the meshlet was culled, then we don't need to draw it
    if !get_meshlet_occlusion(cluster_id) { return; }

    // If the meshlet was drawn in the first pass, and this is the second pass, then we don't need to draw it
#ifdef MESHLET_SECOND_WRITE_INDEX_BUFFER_PASS
    if get_meshlet_previous_occlusion(cluster_id) { return; }
#endif

    let meshlet_id = meshlet_thread_meshlet_ids[cluster_id];
    let meshlet = meshlets[meshlet_id];

    // Reserve space in the buffer for this meshlet's triangles, and broadcast the start of that slice to all threads
    if triangle_id == 0u {
        draw_index_buffer_start_workgroup = atomicAdd(&draw_command_buffer.vertex_count, meshlet.triangle_count * 3u);
        draw_index_buffer_start_workgroup /= 3u;
    }
    workgroupBarrier();

    // Each thread writes one triangle of the meshlet to the buffer slice reserved for the meshlet
    if triangle_id < meshlet.triangle_count {
        draw_index_buffer[draw_index_buffer_start_workgroup + base_index_id] = (cluster_id << 8u) | triangle_id;
    }
}
