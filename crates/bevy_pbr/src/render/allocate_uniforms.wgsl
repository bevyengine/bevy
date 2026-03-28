// A compute shader that allocates `MeshUniform`s.
//
// This shader runs before mesh preprocessing in order to determine the
// positions of `MeshUniform`s. Unlike `MeshInputUniform`s, which are scattered
// throughout the buffer, `MeshUniform`s are indexed by instance ID, and so we
// must place instances of the same mesh together in the buffer. One dispatch
// call corresponds to one batch set (i.e. one multidraw operation), and one
// thread corresponds to one bin (a.k.a. draw, a.k.a. batch).
//
// Essentially, the goal of this shader is to perform a prefix sum, using the
// "scan-then-fan" approach. It has three phases:
//
// 1. *Local scan*: Perform a [Hillis-Steele scan] on each chunk of draws, where
// the size of each chunk (i.e. the number of draws) is equal to the workgroup
// size (256). Write the total size for this chunk to the fan buffer.
//
// 2. *Global scan*: Do a Hillis-Steele scan on the fan buffer. Now we know the
// running total for each chunk.
//
// 3. *Fan*: Copy the running total for each chunk to every element of that
// chunk.
//
// Note that, for batch sets (i.e. multidraw indirect calls) that have fewer
// than 256 batches in them, we only need step (1). This is the common case.
//
// [Hillis-Steele scan]: https://en.wikipedia.org/wiki/Prefix_sum#Algorithm_1:_Shorter_span,_more_parallel

#import bevy_pbr::mesh_preprocess_types::{BinMetadata, IndirectParametersMetadata}

// Information needed to allocate `MeshUniform`s.
struct UniformAllocationMetadata {
    // The index of this batch set in the `IndirectBatchSet` array.
    //
    // We write this into the `indirect_parameters_metadata`.
    batch_set_index: u32,

    // The number of bins (a.k.a. draws, a.k.a. batches) in this batch set.
    bin_count: u32,

    // The index of the first set of indirect parameters for this batch set.
    //
    // This is also the index of the first `IndirectParametersMetadata`, as
    // that's a parallel array with the indirect parameters.
    first_indirect_parameters_index: u32,

    // The index of the first `MeshUniform` slot for this batch set.
    first_output_mesh_uniform_index: u32,

    // Padding.
    pad: array<vec4<f32>, 15u>,
};

// The number of threads in a workgroup.
const WORKGROUP_SIZE: u32 = 256u;

// Information needed to allocate `MeshUniform`s.
@group(0) @binding(0) var<uniform> allocate_uniforms_metadata: UniformAllocationMetadata;

// Information for each bin, including the indirect parameters offset and the
// instance count.
@group(0) @binding(1) var<storage> bin_metadata: array<BinMetadata>;

// The array of indirect parameters metadata that we fill out, one for each
// batch.
@group(0) @binding(2) var<storage, read_write> indirect_parameters_metadata:
    array<IndirectParametersMetadata>;

// A temporary buffer that stores the mesh uniform index of the last instance
// plus one for each workgroup (i.e. for each 256-bin chunk).
//
// This is accumulated in the second stage and written out in the third.
@group(0) @binding(3) var<storage, read_write> fan_buffer: array<u32>;

// Scratch memory that stores the prefix sum for every element in our chunk.
var<workgroup> output_offsets: array<u32, 256>;

// The first step of the prefix sum. This computes the prefix sum for each
// 256-element chunk.
//
// Note that this will be the *only* step in the operation if the total number
// of bins in this batch set is 256 or fewer. Thus we must fill in the indirect
// parameters metadata for each batch here, as we can't guarantee that the
// following two steps will be run at all.
@compute @workgroup_size(256, 1, 1)
fn allocate_local_scan(
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) group_id: vec3<u32>,
    @builtin(global_invocation_id) global_id: vec3<u32>
) {
    let bin_count = allocate_uniforms_metadata.bin_count;

    let block_start = group_id.x * WORKGROUP_SIZE;
    let block_end = min(block_start + WORKGROUP_SIZE, bin_count);

    // If this is the first workgroup, take the first output index from the
    // metadata into account. But if this is the second chunk or beyond, don't
    // do that, as the second and third phases will add it in and we don't want
    // to double-count it.
    if (group_id.x == 0u) {
        output_offsets[local_id.x] = allocate_uniforms_metadata.first_output_mesh_uniform_index;
    } else {
        output_offsets[local_id.x] = 0u;
    }
    workgroupBarrier();

    // We're doing an inclusive sum, so put the instance count in the *next* bin.
    if (global_id.x < block_end && local_id.x < WORKGROUP_SIZE - 1u) {
        output_offsets[local_id.x + 1] = bin_metadata[global_id.x].instance_count;
    }
    workgroupBarrier();

    // Prefix sum within our workgroup.
    hillis_steele_scan(local_id.x);

    // Now write the indirect parameters metadata for this batch. We fill in the
    // `base_output_index` with the value of the prefix sum (which might be
    // incomplete if this isn't the first chunk). We also populate a few
    // bookkeeping fields for later rendering passes to use.
    if (global_id.x < block_end) {
        let indirect_parameters_offset =
            allocate_uniforms_metadata.first_indirect_parameters_index +
            bin_metadata[global_id.x].indirect_parameters_offset;
        indirect_parameters_metadata[indirect_parameters_offset].base_output_index =
            output_offsets[local_id.x];
        indirect_parameters_metadata[indirect_parameters_offset].batch_set_index =
            allocate_uniforms_metadata.batch_set_index;
        // These parameters get filled in later. Initialize them to zero for now.
        // This is required in the case of the early/late instance counts
        // because the mesh preprocessing shader will atomically increment them.
        indirect_parameters_metadata[indirect_parameters_offset].mesh_index = 0u;
        indirect_parameters_metadata[indirect_parameters_offset].early_instance_count = 0u;
        indirect_parameters_metadata[indirect_parameters_offset].late_instance_count = 0u;
    }

    // If this is the last element in the workgroup, put the total number of
    // instances (plus the first output mesh uniform index if we're the first
    // workgroup) in the fan buffer in preparation for the next phase.
    if (local_id.x == WORKGROUP_SIZE - 1u) {
        fan_buffer[group_id.x] = output_offsets[WORKGROUP_SIZE - 1u] +
            bin_metadata[global_id.x].instance_count;
    }
}

// The second step of the prefix sum.
//
// This step takes the intermediate fan values computed in the previous step
// (i.e. the sum going out of each chunk) and performs one or more Hillis-Steele
// scans in order to compute the fan value going into each chunk.
//
// This step is omitted if there are 256 or fewer total draws.
@compute @workgroup_size(256, 1, 1)
fn allocate_global_scan(@builtin(local_invocation_id) local_id: vec3<u32>) {
    var sum = 0u;
    let chunk_count = div_ceil(allocate_uniforms_metadata.bin_count, WORKGROUP_SIZE);

    // Do a sequential loop over each block of 256 chunks. Because each
    // iteration of this loop covers 64K meshes, the fact that it's sequential
    // isn't going to be a problem in practice.
    for (var block_start = 0u; block_start < chunk_count; block_start += WORKGROUP_SIZE) {
        // Set up the Hillis-Steele scan.
        let block_end = min(block_start + WORKGROUP_SIZE, chunk_count);
        let global_id = block_start + local_id.x;
        if (global_id < block_end) {
            output_offsets[local_id.x] = sum + fan_buffer[global_id];
        }
        workgroupBarrier();

        // Perform the scan.
        hillis_steele_scan(local_id.x);

        // Write the value back.
        if (global_id < block_end) {
            fan_buffer[global_id] = output_offsets[local_id.x];
        }

        // Save the sum coming out of this block for the next one.
        sum = output_offsets[WORKGROUP_SIZE - 1u];
    }
}

// The third step of the prefix sum.
//
// We take the summed fan value computed in the previous step and add it in to
// each value of each chunk beyond the first. We dispatch one fewer workgroup
// here than in step (1), because there's nothing to do for the first chunk.
//
// This step is omitted if there are 256 or fewer total draws.
@compute @workgroup_size(256, 1, 1)
fn allocate_fan(
    @builtin(workgroup_id) group_id: vec3<u32>,
    @builtin(global_invocation_id) global_id: vec3<u32>
) {
    let id = global_id.x + WORKGROUP_SIZE;
    let bin_count = allocate_uniforms_metadata.bin_count;
    if (id >= bin_count) {
        return;
    }

    let fan_value = fan_buffer[group_id.x];
    let indirect_parameters_offset =
        allocate_uniforms_metadata.first_indirect_parameters_index +
        bin_metadata[id].indirect_parameters_offset;
    indirect_parameters_metadata[indirect_parameters_offset].base_output_index += fan_value;
}

// Calculates a running exclusive sum.
// https://en.wikipedia.org/wiki/Prefix_sum#Algorithm_1:_Shorter_span,_more_parallel
fn hillis_steele_scan(local_id: u32) {
    for (var offset = 1u; offset < WORKGROUP_SIZE; offset *= 2u) {
        var term = 0u;
        if (local_id >= offset) {
            term = output_offsets[local_id - offset];
        }
        workgroupBarrier();
        output_offsets[local_id] += term;
        workgroupBarrier();
    }
}

// Divides unsigned integer a by b, rounding up.
fn div_ceil(a: u32, b: u32) -> u32 {
    return (a + b - 1u) / b;
}
