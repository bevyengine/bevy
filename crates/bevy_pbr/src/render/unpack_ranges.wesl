// Expands GPU-authored instance-batch range entries into PreprocessWorkItems.

import bevy_render::occlusion_culling::mesh_preprocess_types::PreprocessWorkItem;

struct RangeUnpackingMetadata {
    range_count: u32,
    total_instance_count: u32,
    pad: vec2<u32>,
}

struct RangeWorkItem {
    base_input_index: u32,
    base_output_or_indirect_parameters_index: u32,
    count: u32,
    cumulative_offset: u32,
    // Base index in the GPU-initialized tail of the work item buffer.
    work_item_base: u32,
}

@group(0) @binding(0) var<uniform> metadata: RangeUnpackingMetadata;
@group(0) @binding(1) var<storage> ranges: array<RangeWorkItem>;
@group(0) @binding(2) var<storage, read_write> preprocess_work_items: array<PreprocessWorkItem>;

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let global_id = global_invocation_id.x;
    if (global_id >= metadata.total_instance_count) {
        return;
    }

    // Prefix offsets are sorted, so binary search for the range containing
    // this instance.
    var low = 0u;
    var high = metadata.range_count;
    while (low + 1u < high) {
        let mid = low + (high - low) / 2u;
        if (ranges[mid].cumulative_offset <= global_id) {
            low = mid;
        } else {
            high = mid;
        }
    }

    let range = ranges[low];
    let i_in_range = global_id - range.cumulative_offset;
    let work_item_idx = range.work_item_base + i_in_range;

    preprocess_work_items[work_item_idx].input_index =
        range.base_input_index + i_in_range;
    preprocess_work_items[work_item_idx].output_or_indirect_parameters_index =
        range.base_output_or_indirect_parameters_index;
}
