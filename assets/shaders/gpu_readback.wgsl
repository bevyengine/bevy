@group(0) @binding(0) var<storage, read_write> data: array<u32, 1>;

@compute @workgroup_size(1, 1, 1)
fn main() {
    data[0] += 1u;
}