// This shader is used for the gpu_readback example
// The actual work it does is not important for the example

// This is the data that lives in the gpu only buffer
@group(0) @binding(0) var<storage, read_write> data: array<u32>;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // We use the global_id to index the array to make sure we don't
    // access data used in another workgroup
    data[global_id.x] += 1u;
}
