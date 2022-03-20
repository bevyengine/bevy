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
        a.x + b.x,
        a.y + b.y,
        a.z + b.z,
        a.w + b.w,
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