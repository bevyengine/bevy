// A compute shader that unpacks bins.
//
// This shader runs before mesh preprocessing in order to generate
// `PreprocessWorkItem`s from cached bins. A single dispatch of this shader
// corresponds to a single batch set, and one invocation of this shader
// corresponds to one binned entity. Each shader invocation builds the work item
// for its entity by copying over the mesh input uniform index and calculating
// the position of the command in the indirect parameters buffer that will draw
// that entity.

#import bevy_pbr::mesh_preprocess_types::PreprocessWorkItem

// Information needed to unpack bins belonging to a single batch set.
struct BinUnpackingMetadata {
    // The index of the first `PreprocessWorkItem` that this compute shader
    // dispatch is to write to.
    base_output_work_item_index: u32,
    // The index of the first GPU indirect parameters command for this batch
    // set.
    base_indirect_parameters_index: u32,
    // The number of binned mesh instances in the `binned_mesh_instances`
    // array.
    binned_mesh_instance_count: u32,
    // Padding.
    pad_a: u32,
    // Padding.
    pad_b: array<vec4<u32>, 15>,
};

// One mesh instance in a bin.
//
// This corresponds to the CPU-side `GpuRenderBinnedMeshInstance` structure.
//
// Note that this structure isn't sorted within the
// `binned_mesh_instances` buffer. Instances from the same bin aren't
// guaranteed to be adjacent to one another.
struct BinnedMeshInstance {
    // The index of the `MeshInputUniform` corresponding to this mesh instance
    // in the `MeshInputUniform` buffer.
    input_uniform_index: u32,
    // The index of the bin that this mesh instance belongs to.
    bin_index: u32,
};

// Metadata for the entire batch set.
@group(0) @binding(0) var<uniform> bin_unpacking_metadata: BinUnpackingMetadata;

// The input array of `BinnedMeshInstance`s.
//
// Note that this array isn't sorted.
@group(0) @binding(1) var<storage> binned_mesh_instances: array<BinnedMeshInstance>;

// The output list of `PreprocessWorkItem`s.
@group(0) @binding(2) var<storage, read_write> preprocess_work_items: array<PreprocessWorkItem>;

// A mapping from each `bin_index` to the index of the GPU indirect parameters
// for this bin, relative to the start of the indirect parameters for this batch
// set.
@group(0) @binding(3) var<storage> bin_index_to_indirect_parameters_offset: array<u32>;

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    // Figure out which instance we're looking at.
    let global_id = global_invocation_id.x;
    if (global_id >= bin_unpacking_metadata.binned_mesh_instance_count) {
        return;
    }

    // Unpack the `BinnedMeshInstance`.
    let input_uniform_index = binned_mesh_instances[global_id].input_uniform_index;
    let bin_index = binned_mesh_instances[global_id].bin_index;

    // Look up the indirect parameters index for this bin, relative to the first
    // indirect parameters offset for this batch set.
    let indirect_parameters_offset = bin_index_to_indirect_parameters_offset[bin_index];

    // Determine the location we should write the work item to.
    let output_index = bin_unpacking_metadata.base_output_work_item_index + global_id;

    // Write out the resulting work item.
    preprocess_work_items[output_index].input_index = input_uniform_index;
    preprocess_work_items[output_index].output_or_indirect_parameters_index =
        bin_unpacking_metadata.base_indirect_parameters_index + indirect_parameters_offset;
}
