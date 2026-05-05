#import bevy_pbr::cluster::ClusterMetadata
#import bevy_pbr::mesh_view_types::{ClusterOffsetsAndCounts, Lights}

// The shader that allocates the clustered object ID buffer.
//
// The clustered object ID buffer consists of many tightly-packed
// variable-length arrays of clustered object IDs. The offset to the start of
// each list is stored in `ClusterOffsetsAndCounts`. To allocate the lists in
// the buffer, the number of objects of each type in each cluster must be known.
//
// Since the lists are tightly packed, determining the offsets in the global
// buffer is a [prefix sum] problem. To deal with the fact that workgroup sizes
// are limited to 256 in `wgpu`, and we will usually have more clusters than
// that, we use a two-pass approach:
//
// 1. First, the *local* allocation pass runs on workgroups of 256 clusters
// each. A [Hillis-Steele scan] is performed to allocate the position in the
// buffer for each 256 clusters, relative to the first cluster in the workgroup.
//
// 2. Next, the *global* allocation pass runs a sequential loop over each 256
// clusters to calculate the final offsets relative to the previous 256 cluster
// chunk.
//
// At the end of this process, the clusters will have been assigned to their
// final positions in the list.
//
// [prefix sum]: https://en.wikipedia.org/wiki/Prefix_sum
//
// [Hillis-Steele scan]: https://en.wikipedia.org/wiki/Prefix_sum#Algorithm_1:_Shorter_span,_more_parallel

@group(0) @binding(0) var<storage, read_write> offsets_and_counts: ClusterOffsetsAndCounts;
@group(0) @binding(1) var<uniform> lights: Lights;
@group(0) @binding(2) var<storage, read_write> clustering_metadata: ClusterMetadata;
@group(0) @binding(3) var<storage, read_write> scratchpad_offsets_and_counts:
    ClusterOffsetsAndCounts;

// The offset of the first clustered light within the buffer for each thread.
var<workgroup> block_offsets: array<u32, 256>;

// The local allocation pass that, for each chunk of 256 clusters, calculates
// the offset for each cluster in the chunk.
@compute @workgroup_size(256, 1, 1)
fn allocate_local_main(
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) group_id: vec3<u32>,
    @builtin(global_invocation_id) global_id: vec3<u32>
) {
    let cluster_count =
        lights.cluster_dimensions.x *
        lights.cluster_dimensions.y *
        lights.cluster_dimensions.z;

    let block_start = group_id.x * 256u;
    let block_end = min(block_start + 256u, cluster_count);

    // Initialize the block offsets.
    block_offsets[local_id.x] = 0u;
    workgroupBarrier();

    // Compute our "block offset" relative to the cluster before us.
    // After a prefix scan, the offsets will be correct.
    if (global_id.x < block_end && local_id.x < 255u) {
        block_offsets[local_id.x + 1] = cluster_object_count(global_id.x);
    }
    workgroupBarrier();

    // Do the Hillis-Steele scan.
    for (var offset = 1u; offset < 256u; offset *= 2u) {
        var term = 0u;
        if (local_id.x >= offset) {
            term = block_offsets[local_id.x - offset];
        }
        workgroupBarrier();
        block_offsets[local_id.x] += term;
        workgroupBarrier();
    }

    if (global_id.x < block_end) {
        // Now write in the local offset.
        offsets_and_counts.data[global_id.x][0].x = block_offsets[local_id.x];

        // Zero out the scratchpad counts in preparation for the populate phase
        // of the rasterizer.
        scratchpad_offsets_and_counts.data[global_id.x][0u] = vec4(0u);
        scratchpad_offsets_and_counts.data[global_id.x][1u] = vec4(0u);
    }
}

// The global allocation pass that propagates the offsets from each chunk of 256
// clusters to later chunks sequentially.
@compute @workgroup_size(256, 1, 1)
fn allocate_global_main(@builtin(local_invocation_id) local_id: vec3<u32>) {
    let cluster_count =
        lights.cluster_dimensions.x *
        lights.cluster_dimensions.y *
        lights.cluster_dimensions.z;

    // March along the chunks sequentially, accumulating as we go.
    var current_offset = 0u;
    for (var i = 0u; i < cluster_count; i += 256u) {
        offsets_and_counts.data[i + local_id.x][0].x += current_offset;
        storageBarrier();

        if (i + 255u < cluster_count) {
            current_offset = offsets_and_counts.data[i + 255u][0].x +
                cluster_object_count(i + 255u);
        }
    }
    storageBarrier();

    // Write in the final size. This will be read back to the CPU so that the
    // buffer can be resized if necessary.
    if (local_id.x == 0u) {
        clustering_metadata.index_list_size = offsets_and_counts.data[cluster_count - 1][0].x +
            cluster_object_count(cluster_count - 1);
    }
}

// Returns the total number of objects in the given cluster.
fn cluster_object_count(cluster_index: u32) -> u32 {
    return
        offsets_and_counts.data[cluster_index][0].y +
        offsets_and_counts.data[cluster_index][0].z +
        offsets_and_counts.data[cluster_index][0].w +
        offsets_and_counts.data[cluster_index][1].x +
        offsets_and_counts.data[cluster_index][1].y;
}
