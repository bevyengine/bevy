// If using this WGSL snippet as an #import, a dedicated
// "joint_matricies" uniform of type SkinnedMesh must be added in the
// main shader.

#define_import_path bevy_pbr::skinning

/// HACK: This works around naga not supporting matrix addition in SPIR-V
// translations. See https://github.com/gfx-rs/naga/issues/1527
fn add_matrix(
    a: mat4x4<f32>,
    b: mat4x4<f32>,
) -> mat4x4<f32> {
    return mat4x4<f32>(
        a[0] + b[0],
        a[1] + b[1],
        a[2] + b[2],
        a[3] + b[3],
    );
}

fn skin_model(
    indexes: vec4<u32>,
    weights: vec4<f32>,
) -> mat4x4<f32> {
    var matrix = weights.x * joint_matrices.data[indexes.x];
    matrix = add_matrix(matrix, weights.y * joint_matrices.data[indexes.y]);
    matrix = add_matrix(matrix, weights.z * joint_matrices.data[indexes.z]);
    return add_matrix(matrix, weights.w * joint_matrices.data[indexes.w]);
}

fn inverse_transpose_3x3(in: mat3x3<f32>) -> mat3x3<f32> {
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
    return inverse_transpose_3x3(mat3x3<f32>(
        model[0].xyz,
        model[1].xyz,
        model[2].xyz
    )) * normal;
}
