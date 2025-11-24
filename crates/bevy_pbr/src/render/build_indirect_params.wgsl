// Builds GPU indirect draw parameters from metadata.
//
// This only runs when indirect drawing is enabled. It takes the output of
// `mesh_preprocess.wgsl` and creates indirect parameters for the GPU.
//
// This shader runs separately for indexed and non-indexed meshes. Unlike
// `mesh_preprocess.wgsl`, which runs one instance per mesh *instance*, one
// instance of this shader corresponds to a single *batch* which could contain
// arbitrarily many instances of a single mesh.

#import bevy_pbr::mesh_preprocess_types::{
    IndirectBatchSet,
    IndirectParametersIndexed,
    IndirectParametersNonIndexed,
    IndirectParametersCpuMetadata,
    IndirectParametersGpuMetadata,
    MeshInput
}

// The data for each mesh that the CPU supplied to the GPU.
@group(0) @binding(0) var<storage> current_input: array<MeshInput>;

// Data that we use to generate the indirect parameters.
//
// The `mesh_preprocess.wgsl` shader emits these.
@group(0) @binding(1) var<storage> indirect_parameters_cpu_metadata:
    array<IndirectParametersCpuMetadata>;

@group(0) @binding(2) var<storage> indirect_parameters_gpu_metadata:
    array<IndirectParametersGpuMetadata>;

// Information about each batch set.
//
// A *batch set* is a set of meshes that might be multi-drawn together.
@group(0) @binding(3) var<storage, read_write> indirect_batch_sets: array<IndirectBatchSet>;

#ifdef INDEXED
// The buffer of indirect draw parameters that we generate, and that the GPU
// reads to issue the draws.
//
// This buffer is for indexed meshes.
@group(0) @binding(4) var<storage, read_write> indirect_parameters:
    array<IndirectParametersIndexed>;
#else   // INDEXED
// The buffer of indirect draw parameters that we generate, and that the GPU
// reads to issue the draws.
//
// This buffer is for non-indexed meshes.
@group(0) @binding(4) var<storage, read_write> indirect_parameters:
    array<IndirectParametersNonIndexed>;
#endif  // INDEXED

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    // Figure out our instance index (i.e. batch index). If this thread doesn't
    // correspond to any index, bail.
    let instance_index = global_invocation_id.x;
    if (instance_index >= arrayLength(&indirect_parameters_cpu_metadata)) {
        return;
    }

    // Unpack the metadata for this batch.
    let base_output_index = indirect_parameters_cpu_metadata[instance_index].base_output_index;
    let batch_set_index = indirect_parameters_cpu_metadata[instance_index].batch_set_index;
    let mesh_index = indirect_parameters_gpu_metadata[instance_index].mesh_index;

    // If we aren't using `multi_draw_indirect_count`, we have a 1:1 fixed
    // assignment of batches to slots in the indirect parameters buffer, so we
    // can just use the instance index as the index of our indirect parameters.
    let early_instance_count =
        indirect_parameters_gpu_metadata[instance_index].early_instance_count;
    let late_instance_count = indirect_parameters_gpu_metadata[instance_index].late_instance_count;

    // If in the early phase, we draw only the early meshes. If in the late
    // phase, we draw only the late meshes. If in the main phase, draw all the
    // meshes.
#ifdef EARLY_PHASE
    let instance_count = early_instance_count;
#else   // EARLY_PHASE
#ifdef LATE_PHASE
    let instance_count = late_instance_count;
#else   // LATE_PHASE
    let instance_count = early_instance_count + late_instance_count;
#endif  // LATE_PHASE
#endif  // EARLY_PHASE

    var indirect_parameters_index = instance_index;

    // If the current hardware and driver support `multi_draw_indirect_count`,
    // dynamically reserve an index for the indirect parameters we're to
    // generate.
#ifdef MULTI_DRAW_INDIRECT_COUNT_SUPPORTED
    // If this batch belongs to a batch set, then allocate space for the
    // indirect commands in that batch set.
    if (batch_set_index != 0xffffffffu) {
        // Bail out now if there are no instances. Note that we can only bail if
        // we're in a batch set. That's because only batch sets are drawn using
        // `multi_draw_indirect_count`. If we aren't using
        // `multi_draw_indirect_count`, then we need to continue in order to
        // zero out the instance count; otherwise, it'll have garbage data in
        // it.
        if (instance_count == 0u) {
            return;
        }

        let indirect_parameters_base =
            indirect_batch_sets[batch_set_index].indirect_parameters_base;
        let indirect_parameters_offset =
            atomicAdd(&indirect_batch_sets[batch_set_index].indirect_parameters_count, 1u);

        indirect_parameters_index = indirect_parameters_base + indirect_parameters_offset;
    }
#endif  // MULTI_DRAW_INDIRECT_COUNT_SUPPORTED

    // Build up the indirect parameters. The structures for indexed and
    // non-indexed meshes are slightly different.

    indirect_parameters[indirect_parameters_index].instance_count = instance_count;

#ifdef LATE_PHASE
    // The late mesh instances are stored after the early mesh instances, so we
    // offset the output index by the number of early mesh instances.
    indirect_parameters[indirect_parameters_index].first_instance =
        base_output_index + early_instance_count;
#else   // LATE_PHASE
    indirect_parameters[indirect_parameters_index].first_instance = base_output_index;
#endif  // LATE_PHASE

    indirect_parameters[indirect_parameters_index].base_vertex =
        current_input[mesh_index].first_vertex_index;

#ifdef INDEXED
    indirect_parameters[indirect_parameters_index].index_count =
        current_input[mesh_index].index_count;
    indirect_parameters[indirect_parameters_index].first_index =
        current_input[mesh_index].first_index_index;
#else   // INDEXED
    indirect_parameters[indirect_parameters_index].vertex_count =
        current_input[mesh_index].index_count;
#endif  // INDEXED
}
