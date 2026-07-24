import bevy_pbr::render::mesh_types::SkinnedMesh;
import bevy_pbr::render::mesh_bindings::mesh;

@if(SKINNED && SKINS_USE_UNIFORM_BUFFERS)
@group(2) @binding(1) var<uniform> joint_matrices: SkinnedMesh;
@if(SKINNED && !SKINS_USE_UNIFORM_BUFFERS)
@group(2) @binding(1) var<storage> joint_matrices: array<mat4x4<f32>>;

// An array of matrices specifying the joint positions from the previous frame.
//
// This is used for motion vector computation.
//
// If this is the first frame, or we're otherwise prevented from using data from
// the previous frame, this is simply the same as `joint_matrices` above.
@if(SKINNED && SKINS_USE_UNIFORM_BUFFERS)
@group(2) @binding(6) var<uniform> prev_joint_matrices: SkinnedMesh;
@if(SKINNED && !SKINS_USE_UNIFORM_BUFFERS)
@group(2) @binding(6) var<storage> prev_joint_matrices: array<mat4x4<f32>>;

@if(SKINNED)
fn skin_model(
    indexes: vec4<u32>,
    weights: vec4<f32>,
    instance_index: u32,
) -> mat4x4<f32> {
@if(SKINS_USE_UNIFORM_BUFFERS)
    return weights.x * joint_matrices.data[indexes.x]
        + weights.y * joint_matrices.data[indexes.y]
        + weights.z * joint_matrices.data[indexes.z]
        + weights.w * joint_matrices.data[indexes.w];
@if(!SKINS_USE_UNIFORM_BUFFERS)
    var skin_index = mesh[instance_index].current_skin_index;
@if(!SKINS_USE_UNIFORM_BUFFERS)
    return weights.x * joint_matrices[skin_index + indexes.x]
        + weights.y * joint_matrices[skin_index + indexes.y]
        + weights.z * joint_matrices[skin_index + indexes.z]
        + weights.w * joint_matrices[skin_index + indexes.w];
}

// Returns the skinned position of a vertex with the given weights from the
// previous frame.
//
// This is used for motion vector computation.
@if(SKINNED)
fn skin_prev_model(
    indexes: vec4<u32>,
    weights: vec4<f32>,
    instance_index: u32,
) -> mat4x4<f32> {
@if(SKINS_USE_UNIFORM_BUFFERS)
    return weights.x * prev_joint_matrices.data[indexes.x]
        + weights.y * prev_joint_matrices.data[indexes.y]
        + weights.z * prev_joint_matrices.data[indexes.z]
        + weights.w * prev_joint_matrices.data[indexes.w];
@if(!SKINS_USE_UNIFORM_BUFFERS)
    let skin_index = mesh[instance_index].current_skin_index;
@if(!SKINS_USE_UNIFORM_BUFFERS)
    return weights.x * prev_joint_matrices[skin_index + indexes.x]
        + weights.y * prev_joint_matrices[skin_index + indexes.y]
        + weights.z * prev_joint_matrices[skin_index + indexes.z]
        + weights.w * prev_joint_matrices[skin_index + indexes.w];
}

@if(SKINNED)
fn inverse_transpose_3x3m(in: mat3x3<f32>) -> mat3x3<f32> {
    let x = cross(in[1], in[2]);
    let y = cross(in[2], in[0]);
    let z = cross(in[0], in[1]);
    let det = dot(in[2], z);
    return mat3x3<f32>(
        x / det,
        y / det,
        z / det
    );
}

@if(SKINNED)
fn skin_normals(
    world_from_local: mat4x4<f32>,
    normal: vec3<f32>,
) -> vec3<f32> {
    return normalize(
        inverse_transpose_3x3m(
            mat3x3<f32>(
                world_from_local[0].xyz,
                world_from_local[1].xyz,
                world_from_local[2].xyz
            )
        ) * normal
    );
}
