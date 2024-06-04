#import bevy_pbr::meshlet_bindings::{
    cluster_count,
    meshlet_instance_meshlet_counts_prefix_sum,
    meshlet_instance_meshlet_slice_starts,
    meshlet_cluster_instance_ids,
    meshlet_cluster_meshlet_ids,
}

/// Writes out instance_id and meshlet_id to the global buffers for each cluster in the scene.

@compute
@workgroup_size(128, 1, 1) // 128 threads per workgroup, 1 cluster per thread
fn fill_cluster_buffers(
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
    @builtin(local_invocation_id) local_invocation_id: vec3<u32>
) {
    // Calculate the cluster ID for this thread
    let cluster_id = local_invocation_id.x + 128u * dot(workgroup_id, vec3(num_workgroups.x * num_workgroups.x, num_workgroups.x, 1u));
    if cluster_id >= cluster_count { return; }

    // Binary search to find the instance this cluster belongs to
    var left = 0u;
    var right = arrayLength(&meshlet_instance_meshlet_counts_prefix_sum) - 1u;
    while left <= right {
        let mid = (left + right) / 2u;
        if meshlet_instance_meshlet_counts_prefix_sum[mid] <= cluster_id {
            left = mid + 1u;
        } else {
            right = mid - 1u;
        }
    }
    let instance_id = right;

    // Find the meshlet ID for this cluster within the instance's MeshletMesh
    let meshlet_id_local = cluster_id - meshlet_instance_meshlet_counts_prefix_sum[instance_id];

    // Find the overall meshlet ID in the global meshlet buffer
    let meshlet_id = meshlet_id_local + meshlet_instance_meshlet_slice_starts[instance_id];

    // Write results to buffers
    meshlet_cluster_instance_ids[cluster_id] = instance_id;
    meshlet_cluster_meshlet_ids[cluster_id] = meshlet_id;
}
