#define_import_path bevy_render::maths

const PI: f32 = 3.141592653589793;      // π
const PI_2: f32 = 6.283185307179586;    // 2π
const HALF_PI: f32 = 1.57079632679;     // π/2
const FRAC_PI_3: f32 = 1.0471975512;    // π/3
const E: f32 = 2.718281828459045;       // exp(1)

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

// Extracts the upper 3x3 portion of a 4x4 matrix.
fn mat4x4_to_mat3x3(m: mat4x4<f32>) -> mat3x3<f32> {
    return mat3x3<f32>(m[0].xyz, m[1].xyz, m[2].xyz);
}

// Creates an orthonormal basis given a normalized Z vector.
//
// The results are equivalent to the Gram-Schmidt process [1].
//
// [1]: https://math.stackexchange.com/a/1849294
fn orthonormalize(z_normalized: vec3<f32>) -> mat3x3<f32> {
    var up = vec3(0.0, 1.0, 0.0);
    if (abs(dot(up, z_normalized)) > 0.99) {
        up = vec3(1.0, 0.0, 0.0); // Avoid creating a degenerate basis.
    }
    let x_basis = normalize(cross(z_normalized, up));
    let y_basis = cross(z_normalized, x_basis);
    return mat3x3(x_basis, y_basis, z_normalized);
}

// Returns true if any part of a sphere is on the positive side of a plane.
//
// `sphere_center.w` should be 1.0.
//
// This is used for frustum culling.
fn sphere_intersects_plane_half_space(
    plane: vec4<f32>,
    sphere_center: vec4<f32>,
    sphere_radius: f32
) -> bool {
    return dot(plane, sphere_center) + sphere_radius > 0.0;
}

// pow() but safe for NaNs/negatives
fn powsafe(color: vec3<f32>, power: f32) -> vec3<f32> {
    return pow(abs(color), vec3(power)) * sign(color);
}

// https://en.wikipedia.org/wiki/Vector_projection#Vector_projection_2
fn project_onto(lhs: vec3<f32>, rhs: vec3<f32>) -> vec3<f32> {
    let other_len_sq_rcp = 1.0 / dot(rhs, rhs);
    return rhs * dot(lhs, rhs) * other_len_sq_rcp;
}

// Below are fast approximations of common irrational and trig functions. These
// are likely most useful when raymarching, for example, where complete numeric
// accuracy can be sacrificed for greater sample count.

// Slightly less accurate than fast_acos_4, but much simpler.
fn fast_acos(in_x: f32) -> f32 {
    let x = abs(in_x);
    var res = -0.156583 * x + HALF_PI;
    res *= sqrt(1.0 - x);
    return select(PI - res, res, in_x >= 0.0);
}

// 4th order polynomial approximation
// 4 VGRP, 16 ALU Full Rate
// 7 * 10^-5 radians precision
// Reference : Handbook of Mathematical Functions (chapter : Elementary Transcendental Functions), M. Abramowitz and I.A. Stegun, Ed.
fn fast_acos_4(x: f32) -> f32 {
    let x1 = abs(x);
    let x2 = x1 * x1;
    let x3 = x2 * x1;
    var s: f32;

    s = -0.2121144 * x1 + 1.5707288;
    s = 0.0742610 * x2 + s;
    s = -0.0187293 * x3 + s;
    s = sqrt(1.0 - x1) * s;

	// acos function mirroring
    return select(PI - s, s, x >= 0.0);
}

fn fast_atan2(y: f32, x: f32) -> f32 {
    var t0 = max(abs(x), abs(y));
    var t1 = min(abs(x), abs(y));
    var t3 = t1 / t0;
    var t4 = t3 * t3;

    t0 = 0.0872929;
    t0 = t0 * t4 - 0.301895;
    t0 = t0 * t4 + 1.0;
    t3 = t0 * t3;

    t3 = select(t3, (0.5 * PI) - t3, abs(y) > abs(x));
    t3 = select(t3, PI - t3, x < 0);
    t3 = select(-t3, t3, y > 0);

    return t3;
}
