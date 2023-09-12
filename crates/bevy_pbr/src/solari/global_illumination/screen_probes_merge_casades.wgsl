@compute @workgroup_size(8, 8, 1)
fn merge_screen_probe_cascades(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // layer 3 -> 2
    // layer 2 -> 1
    // layer 1 -> 0
}
