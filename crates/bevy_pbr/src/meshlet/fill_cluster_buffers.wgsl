#import bevy_pbr::meshlet_bindings::{
    scene_instance_count,
    meshlet_global_cluster_count,
    meshlet_instance_meshlet_counts,
    meshlet_instance_meshlet_slice_starts,
    meshlet_cluster_instance_ids,
    meshlet_cluster_meshlet_ids,
}

/// Writes out instance_id and meshlet_id to the global buffers for each cluster in the scene.

var<workgroup> cluster_slice_start_workgroup: u32;

@compute
@workgroup_size(1024, 1, 1) // 1024 threads per workgroup, 1 instance per workgroup
fn fill_cluster_buffers(
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
    @builtin(local_invocation_index) local_invocation_index: u32,
) {
    // Calculate the instance ID for this workgroup
    var instance_id = workgroup_id.x + (workgroup_id.y * num_workgroups.x);
    if instance_id >= scene_instance_count { return; }

    let instance_meshlet_count = meshlet_instance_meshlet_counts[instance_id];
    let instance_meshlet_slice_start = meshlet_instance_meshlet_slice_starts[instance_id];

    // Reserve cluster slots for the instance and broadcast to the workgroup
    if local_invocation_index == 0u {
        cluster_slice_start_workgroup = atomicAdd(&meshlet_global_cluster_count, instance_meshlet_count);
    }
    let cluster_slice_start = workgroupUniformLoad(&cluster_slice_start_workgroup);

    // Loop enough times to write out all the meshlets for the instance given that each thread writes 1 meshlet in each iteration
    for (var clusters_written = 0u; clusters_written < instance_meshlet_count; clusters_written += 1024u) {
        // Calculate meshlet ID within this instance's MeshletMesh to process for this thread
        let meshlet_id_local = clusters_written + local_invocation_index;
        if meshlet_id_local >= instance_meshlet_count { return; }

        // Find the overall cluster ID in the global cluster buffer
        let cluster_id = cluster_slice_start + meshlet_id_local;

        // Find the overall meshlet ID in the global meshlet buffer
        let meshlet_id = instance_meshlet_slice_start + meshlet_id_local;

        // Write results to buffers
        meshlet_cluster_instance_ids[cluster_id] = instance_id;
        meshlet_cluster_meshlet_ids[cluster_id] = meshlet_id;
    }
}
