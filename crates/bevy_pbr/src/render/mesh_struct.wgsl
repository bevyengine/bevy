#define_import_path bevy_pbr::mesh_struct

struct Mesh {
    model: mat4x4<f32>;
    inverse_transpose_model: mat4x4<f32>;
    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
    flags: u32;
};

struct SkinnedMesh {
    joints: array<mat4x4<f32>, 256>;
};

fn skin_model(
    indexes: vec2<u32>,
    weights: vec4<f32>,
    skin: SkinnedMesh,
) -> vec3<f32> {
    weights.x * skin.joints[indexes.x >> 16] +
    weights.y * skin.joints[indexes.x & 0xFFFF] +
    weights.z * skin.joints[indexes.y >> 16] +
    weights.w * skin.joints[indexes.y & 0xFFFF] 
}

let MESH_FLAGS_SHADOW_RECEIVER_BIT: u32 = 1u;
