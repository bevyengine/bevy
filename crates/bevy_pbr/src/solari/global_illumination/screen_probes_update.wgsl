@compute @workgroup_size(8, 8, 1)
fn update_screen_probes(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // TODO: Reproject + trace
}
