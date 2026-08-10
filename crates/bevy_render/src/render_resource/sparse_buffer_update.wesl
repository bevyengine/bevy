// A compute shader that performs scattering sparse updates to
// `AtomicSparseBufferVec` types.
//
// This shader isn't used for every update. Only if the number of updates is
// small is this shader used. Otherwise, the standard `write_buffer` `wgpu`
// command is used to update the buffer in bulk.
//
// We issue one thread per *word*, not per element. That allows us to achieve
// maximum parallelism, without any loops.

// Metadata that describes the update.
struct SparseBufferUpdateMetadata {
    // The size of a single element in words.
    element_size: u32,
    // The total number of elements to be updated.
    updated_element_count: u32,
};

// The buffer we're copying to.
@group(0) @binding(0) var<storage, read_write> dest_buffer: array<u32>;
// The buffer we're copying from.
@group(0) @binding(1) var<storage> src_buffer: array<u32>;
// For each element in `src_buffer`, the element in `dest_buffer` that we should
// copy it to.
@group(0) @binding(2) var<storage> indices: array<u32>;
// Metadata that describes the operation.
@group(0) @binding(3) var<uniform> metadata: SparseBufferUpdateMetadata;

@workgroup_size(256, 1, 1)
@compute
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // Calculate which word we are. Remember that this shader executes with one
    // thread per word.
    let invocation_index = global_id.x;
    let total_word_count = metadata.updated_element_count * metadata.element_size;
    if (invocation_index >= total_word_count) {
        return;
    }

    // Calculate which element we are.
    let element_index = invocation_index / metadata.element_size;
    // Calculate which word *within* that element we're looking at.
    let word_index = invocation_index % metadata.element_size;

    // Look up our destination element.
    let dest_element_index = indices[element_index];
    // Calculate where we should write our word.
    let dest_index = dest_element_index * metadata.element_size + word_index;
    if (dest_index >= arrayLength(&dest_buffer)) {
        return;
    }

    // Copy the word over.
    let src_index = element_index * metadata.element_size + word_index;
    dest_buffer[dest_index] = src_buffer[src_index];
}
