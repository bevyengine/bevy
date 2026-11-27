import bevy_render::occlusion_culling::mesh_preprocess_types::{MeshInput, MeshCullingData};
import package::gpu_instance_batch::gpu_instance::GpuMeshInstance;
import package::render::mesh_types::MESH_FLAGS_SIGN_DETERMINANT_MODEL_3X3_BIT;

struct Metadata {
    mesh_input: MeshInput,
    bounds: MeshCullingData,
    input_base: u32,
    capacity: u32,
    pad: vec2<u32>,
}
@group(0) @binding(0) var<uniform> metadata: Metadata;
@group(0) @binding(1) var<storage> instances: array<GpuMeshInstance>;
@group(0) @binding(2) var<storage, read_write> inputs: array<MeshInput>;
@group(0) @binding(3) var<storage, read_write> bounds: array<MeshCullingData>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= metadata.capacity { return; }
    let output = metadata.input_base + id.x;
    let instance = instances[id.x];
    var input = metadata.mesh_input;
    input.world_from_local = instance.world_from_local;
    // The affine transpose stores the linear matrix in the first three lanes.
    let linear = mat3x3<f32>(instance.world_from_local[0].xyz,
        instance.world_from_local[1].xyz, instance.world_from_local[2].xyz);
    input.flags = (input.flags & ~MESH_FLAGS_SIGN_DETERMINANT_MODEL_3X3_BIT)
        | select(0u, MESH_FLAGS_SIGN_DETERMINANT_MODEL_3X3_BIT, determinant(linear) >= 0.0);
    input.tag = instance.tag;
    inputs[output] = input;
    var culling = metadata.bounds;
    culling.is_active = instance.is_active;
    bounds[output] = culling;
}
