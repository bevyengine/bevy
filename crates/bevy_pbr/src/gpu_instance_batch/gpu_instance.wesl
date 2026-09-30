// One instance slot published by a GPU producer. Zeroed slots are inactive.
struct GpuMeshInstance {
    world_from_local: mat3x4<f32>,
    is_active: u32,
    tag: u32,
    pad: vec2<u32>,
}
