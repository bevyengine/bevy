#define_import_path bevy_render::maths

fn affine2_to_square(affine: mat3x2<f32>) -> mat3x3<f32> {
    return mat3x3<f32>(
        vec3<f32>(affine[0].xy, 0.0),
        vec3<f32>(affine[1].xy, 0.0),
        vec3<f32>(affine[2].xy, 1.0),
    );
}

fn affine3_to_square(affine: mat3x4<f32>) -> mat4x4<f32> {
    return transpose(mat4x4<f32>(
        affine[0],
        affine[1],
        affine[2],
        vec4<f32>(0.0, 0.0, 0.0, 1.0),
    ));
}

fn mat2x4_f32_to_mat3x3_unpack(
    a: mat2x4<f32>,
    b: f32,
) -> mat3x3<f32> {
    return mat3x3<f32>(
        a[0].xyz,
        vec3<f32>(a[0].w, a[1].xy),
        vec3<f32>(a[1].zw, b),
    );
}

// Extracts the square portion of an affine matrix: i.e. discards the
// translation.
fn affine3_to_mat3x3(affine: mat4x3<f32>) -> mat3x3<f32> {
    return mat3x3<f32>(affine[0].xyz, affine[1].xyz, affine[2].xyz);
}

// Returns the inverse of a 3x3 matrix.
fn inverse_mat3x3(matrix: mat3x3<f32>) -> mat3x3<f32> {
    let tmp0 = cross(matrix[1], matrix[2]);
    let tmp1 = cross(matrix[2], matrix[0]);
    let tmp2 = cross(matrix[0], matrix[1]);
    let inv_det = 1.0 / dot(matrix[2], tmp2);
    return transpose(mat3x3<f32>(tmp0 * inv_det, tmp1 * inv_det, tmp2 * inv_det));
}

// Returns the inverse of an affine matrix.
//
// https://en.wikipedia.org/wiki/Affine_transformation#Groups
fn inverse_affine3(affine: mat4x3<f32>) -> mat4x3<f32> {
    let matrix3 = affine3_to_mat3x3(affine);
    let inv_matrix3 = inverse_mat3x3(matrix3);
    return mat4x3<f32>(inv_matrix3[0], inv_matrix3[1], inv_matrix3[2], -(inv_matrix3 * affine[3]));
}

// Creates an orthonormal basis given a Z vector and an up vector (which becomes
// Y after orthonormalization).
//
// The results are equivalent to the Gram-Schmidt process [1].
//
// [1]: https://math.stackexchange.com/a/1849294
fn orthonormalize(z_unnormalized: vec3<f32>, up: vec3<f32>) -> mat3x3<f32> {
    let z_basis = normalize(z_unnormalized);
    let x_basis = normalize(cross(z_basis, up));
    let y_basis = cross(z_basis, x_basis);
    return mat3x3(x_basis, y_basis, z_basis);
}
