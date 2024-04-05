@group(0) @binding(0) var<storage, read_write> data: u32;

@compute @workgroup_size(1, 1, 1)
fn main() {
    data += 1u;
}
