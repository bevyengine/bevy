// Custom material vertex shader (HLSL)
//
// This shader mirrors Bevy's standard mesh vertex pipeline.
// It reads per-instance mesh data from a storage buffer and transforms
// vertex positions into clip space using the camera's view-projection matrix.
//
// Binding layout:
//   set 0, binding 0 -> View uniform (CameraViewProj + other view data)
//   set 2, binding 0 -> Mesh storage buffer (array of Mesh structs)

struct VertexInput {
    float3 position : POSITION;
    float3 normal   : NORMAL;
    float2 uv       : TEXCOORD0;
};

struct VertexOutput {
    float4 clip_position : SV_POSITION;
    float2 uv            : TEXCOORD0;
};

// View uniform buffer (set 0, binding 0).
// Matches Bevy's View struct. Only clip_from_world is used here;
// it is the very first field so it lives at byte offset 0.
cbuffer View : register(b0, space0) {
    float4x4 clip_from_world;
    // (remaining View fields omitted)
};

// Mesh storage buffer (set 2, binding 0).
// Must match Bevy's Mesh struct layout & alignment.
struct Mesh {
    // world_from_local: mat3x4<f32> — 3 columns, each a float4
    float4 world_from_local_col0;
    float4 world_from_local_col1;
    float4 world_from_local_col2;
    // previous_world_from_local: mat3x4<f32> (not used, but must be present to pad correctly)
    float4 previous_world_from_local_col0;
    float4 previous_world_from_local_col1;
    float4 previous_world_from_local_col2;
};

StructuredBuffer<Mesh> mesh_uniforms : register(t0, space2);

// HLSL equivalent of WGSL affine3_to_square.
// WGSL columns become HLSL rows (column-major transpose -> row-major layout).
float4x4 affine3_to_square(float4 col0, float4 col1, float4 col2) {
    return float4x4(
        col0,
        col1,
        col2,
        float4(0.0, 0.0, 0.0, 1.0)
    );
}

VertexOutput main(VertexInput input, uint instance_id : SV_InstanceID) {
    Mesh m = mesh_uniforms[instance_id];
    float4x4 model = affine3_to_square(
        m.world_from_local_col0,
        m.world_from_local_col1,
        m.world_from_local_col2
    );

    float4 world_position = mul(model, float4(input.position, 1.0));

    VertexOutput output;
    output.clip_position = mul(clip_from_world, world_position);
    output.uv = input.uv;
    return output;
}
