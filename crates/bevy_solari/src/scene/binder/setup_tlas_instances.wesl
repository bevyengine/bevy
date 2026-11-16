@group(0) @binding(0) var<storage, read> transforms: array<array<vec4<f32>, 3>>;
@group(0) @binding(1) var<storage, read> blas_refs: array<vec2<u32>>;
@group(0) @binding(2) var<storage, read_write> instances: array<TlasInstance>;

struct TlasInstance {
    transform: array<vec4<f32>, 3>,
    custom_data_and_mask: u32,
    sbt_offset_and_flags: u32,
    blas_ref: vec2<u32>,
}

@compute @workgroup_size(64, 1, 1)
fn setup_tlas_instances(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let slot = global_id.x;
    if slot >= arrayLength(&instances) || slot >= arrayLength(&blas_refs) || slot >= arrayLength(&transforms) {
        return;
    }

    instances[slot] = TlasInstance(
        transforms[slot],
        (slot & 0xFFFFFFu) | 0xFF000000u,
        0u,
        blas_refs[slot],
    );
}
