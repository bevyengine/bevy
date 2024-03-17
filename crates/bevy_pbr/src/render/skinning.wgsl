#define_import_path bevy_pbr::skinning

#import bevy_pbr::mesh_bindings::mesh

#ifdef SKINNED

#ifdef SKINNED_MESH_STORAGE_BUFFER
@group(1) @binding(1) var<storage> joint_matrices: array<mat4x4<f32>>;
#else
@group(1) @binding(1) var<uniform> joint_matrices: array<mat4x4<f32>,256u>;
#endif


fn skin_model(
    instance_index: u32,
    indexes: vec4<u32>,
    weights: vec4<f32>,
) -> mat4x4<f32> {
#ifdef SKINNED_MESH_STORAGE_BUFFER
    let skin_index=mesh[instance_index].skin_index;
    return weights.x * joint_matrices[skin_index + indexes.x]
        + weights.y * joint_matrices[skin_index + indexes.y]
        + weights.z * joint_matrices[skin_index + indexes.z]
        + weights.w * joint_matrices[skin_index + indexes.w];
#else
    return weights.x * joint_matrices[indexes.x]
        + weights.y * joint_matrices[indexes.y]
        + weights.z * joint_matrices[indexes.z]
        + weights.w * joint_matrices[indexes.w];
#endif
}

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

fn skin_normals(
    model: mat4x4<f32>,
    normal: vec3<f32>,
) -> vec3<f32> {
    return normalize(
        inverse_transpose_3x3m(
            mat3x3<f32>(
                model[0].xyz,
                model[1].xyz,
                model[2].xyz
            )
        ) * normal
    );
}

#endif
