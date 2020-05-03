mod support;

use glam::f32::*;
use support::deg;

const IDENTITY: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

const MATRIX: [[f32; 4]; 4] = [
    [1.0, 2.0, 3.0, 4.0],
    [5.0, 6.0, 7.0, 8.0],
    [9.0, 10.0, 11.0, 12.0],
    [13.0, 14.0, 15.0, 16.0],
];

const ZERO: [[f32; 4]; 4] = [[0.0; 4]; 4];

#[test]
fn test_mat4_align() {
    use std::mem;
    assert_eq!(64, mem::size_of::<Mat4>());
    if cfg!(feature = "scalar-math") {
        assert_eq!(4, mem::align_of::<Mat4>());
    } else {
        assert_eq!(16, mem::align_of::<Mat4>());
    }
}

#[test]
fn test_mat4_identity() {
    let identity = Mat4::identity();
    assert_eq!(IDENTITY, identity.to_cols_array_2d());
    assert_eq!(Mat4::from_cols_array_2d(&IDENTITY), identity);
    assert_eq!(identity, identity * identity);
    assert_eq!(identity, Mat4::default());
}

#[test]
fn test_mat4_zero() {
    assert_eq!(Mat4::from_cols_array_2d(&ZERO), Mat4::zero());
}

#[test]
fn test_mat4_accessors() {
    let mut m = Mat4::zero();
    m.set_x_axis(Vec4::new(1.0, 2.0, 3.0, 4.0));
    m.set_y_axis(Vec4::new(5.0, 6.0, 7.0, 8.0));
    m.set_z_axis(Vec4::new(9.0, 10.0, 11.0, 12.0));
    m.set_w_axis(Vec4::new(13.0, 14.0, 15.0, 16.0));
    assert_eq!(Mat4::from_cols_array_2d(&MATRIX), m);
    assert_eq!(Vec4::new(1.0, 2.0, 3.0, 4.0), m.x_axis());
    assert_eq!(Vec4::new(5.0, 6.0, 7.0, 8.0), m.y_axis());
    assert_eq!(Vec4::new(9.0, 10.0, 11.0, 12.0), m.z_axis());
    assert_eq!(Vec4::new(13.0, 14.0, 15.0, 16.0), m.w_axis());
}

#[test]
fn test_mat4_from_axes() {
    let a = Mat4::from_cols_array_2d(&[
        [1.0, 2.0, 3.0, 4.0],
        [5.0, 6.0, 7.0, 8.0],
        [9.0, 10.0, 11.0, 12.0],
        [13.0, 14.0, 15.0, 16.0],
    ]);
    assert_eq!(MATRIX, a.to_cols_array_2d());
    let b = Mat4::from_cols(
        vec4(1.0, 2.0, 3.0, 4.0),
        vec4(5.0, 6.0, 7.0, 8.0),
        vec4(9.0, 10.0, 11.0, 12.0),
        vec4(13.0, 14.0, 15.0, 16.0),
    );
    assert_eq!(a, b);
    let c = mat4(
        vec4(1.0, 2.0, 3.0, 4.0),
        vec4(5.0, 6.0, 7.0, 8.0),
        vec4(9.0, 10.0, 11.0, 12.0),
        vec4(13.0, 14.0, 15.0, 16.0),
    );
    assert_eq!(a, c);
    let d = b.to_cols_array();
    let f = Mat4::from_cols_array(&d);
    assert_eq!(b, f);
}

#[test]
fn test_mat4_translation() {
    let translate = Mat4::from_translation(vec3(1.0, 2.0, 3.0));
    assert_eq!(
        Mat4::from_cols(
            vec4(1.0, 0.0, 0.0, 0.0),
            vec4(0.0, 1.0, 0.0, 0.0),
            vec4(0.0, 0.0, 1.0, 0.0),
            vec4(1.0, 2.0, 3.0, 1.0)
        ),
        translate
    );
}

#[test]
fn test_from_rotation() {
    let rot_x1 = Mat4::from_rotation_x(deg(180.0));
    let rot_x2 = Mat4::from_axis_angle(Vec3::unit_x(), deg(180.0));
    assert_approx_eq!(rot_x1, rot_x2);
    let rot_y1 = Mat4::from_rotation_y(deg(180.0));
    let rot_y2 = Mat4::from_axis_angle(Vec3::unit_y(), deg(180.0));
    assert_approx_eq!(rot_y1, rot_y2);
    let rot_z1 = Mat4::from_rotation_z(deg(180.0));
    let rot_z2 = Mat4::from_axis_angle(Vec3::unit_z(), deg(180.0));
    assert_approx_eq!(rot_z1, rot_z2);
}

#[test]
fn test_mat4_mul() {
    let mat_a = Mat4::from_axis_angle(Vec3::unit_z(), deg(90.0));
    let result3 = mat_a.transform_vector3(Vec3::unit_y());
    assert_approx_eq!(vec3(-1.0, 0.0, 0.0), result3);
    assert_approx_eq!(result3, (mat_a * Vec3::unit_y().extend(0.0)).truncate());
    let result4 = mat_a * Vec4::unit_y();
    assert_approx_eq!(vec4(-1.0, 0.0, 0.0, 0.0), result4);
    assert_approx_eq!(result4, mat_a * Vec4::unit_y());

    let mat_b = Mat4::from_scale_rotation_translation(
        Vec3::new(0.5, 1.5, 2.0),
        Quat::from_rotation_x(deg(90.0)),
        Vec3::new(1.0, 2.0, 3.0),
    );
    let result3 = mat_b.transform_vector3(Vec3::unit_y());
    assert_approx_eq!(vec3(0.0, 0.0, 1.5), result3, 1.0e-6);
    assert_approx_eq!(result3, (mat_b * Vec3::unit_y().extend(0.0)).truncate());

    let result3 = mat_b.transform_point3(Vec3::unit_y());
    assert_approx_eq!(vec3(1.0, 2.0, 4.5), result3, 1.0e-6);
    assert_approx_eq!(result3, (mat_b * Vec3::unit_y().extend(1.0)).truncate());
}

#[test]
fn test_from_ypr() {
    let zero = deg(0.0);
    let yaw = deg(30.0);
    let pitch = deg(60.0);
    let roll = deg(90.0);
    let y0 = Mat4::from_rotation_y(yaw);
    let y1 = Mat4::from_rotation_ypr(yaw, zero, zero);
    assert_approx_eq!(y0, y1);

    let x0 = Mat4::from_rotation_x(pitch);
    let x1 = Mat4::from_rotation_ypr(zero, pitch, zero);
    assert_approx_eq!(x0, x1);

    let z0 = Mat4::from_rotation_z(roll);
    let z1 = Mat4::from_rotation_ypr(zero, zero, roll);
    assert_approx_eq!(z0, z1);

    let yx0 = y0 * x0;
    let yx1 = Mat4::from_rotation_ypr(yaw, pitch, zero);
    assert_approx_eq!(yx0, yx1);

    let yxz0 = y0 * x0 * z0;
    let yxz1 = Mat4::from_rotation_ypr(yaw, pitch, roll);
    assert_approx_eq!(yxz0, yxz1, 1e-6);
}

#[test]
fn test_from_scale() {
    let m = Mat4::from_scale(Vec3::new(2.0, 4.0, 8.0));
    assert_approx_eq!(
        m.transform_point3(Vec3::new(1.0, 1.0, 1.0)),
        Vec3::new(2.0, 4.0, 8.0)
    );
    assert_approx_eq!(Vec4::unit_x() * 2.0, m.x_axis());
    assert_approx_eq!(Vec4::unit_y() * 4.0, m.y_axis());
    assert_approx_eq!(Vec4::unit_z() * 8.0, m.z_axis());
    assert_approx_eq!(Vec4::unit_w(), m.w_axis());
}

#[test]
fn test_mat4_transpose() {
    let m = mat4(
        vec4(1.0, 2.0, 3.0, 4.0),
        vec4(5.0, 6.0, 7.0, 8.0),
        vec4(9.0, 10.0, 11.0, 12.0),
        vec4(13.0, 14.0, 15.0, 16.0),
    );
    let mt = m.transpose();
    assert_eq!(mt.x_axis(), vec4(1.0, 5.0, 9.0, 13.0));
    assert_eq!(mt.y_axis(), vec4(2.0, 6.0, 10.0, 14.0));
    assert_eq!(mt.z_axis(), vec4(3.0, 7.0, 11.0, 15.0));
    assert_eq!(mt.w_axis(), vec4(4.0, 8.0, 12.0, 16.0));
}

#[test]
fn test_mat4_det() {
    assert_eq!(0.0, Mat4::zero().determinant());
    assert_eq!(1.0, Mat4::identity().determinant());
    assert_eq!(1.0, Mat4::from_rotation_x(deg(90.0)).determinant());
    assert_eq!(1.0, Mat4::from_rotation_y(deg(180.0)).determinant());
    assert_eq!(1.0, Mat4::from_rotation_z(deg(270.0)).determinant());
    assert_eq!(
        2.0 * 2.0 * 2.0,
        Mat4::from_scale(vec3(2.0, 2.0, 2.0)).determinant()
    );
}

#[test]
fn test_mat4_inverse() {
    // assert_eq!(None, Mat4::zero().inverse());
    let inv = Mat4::identity().inverse();
    // assert_ne!(None, inv);
    assert_approx_eq!(Mat4::identity(), inv);

    let rotz = Mat4::from_rotation_z(deg(90.0));
    let rotz_inv = rotz.inverse();
    // assert_ne!(None, rotz_inv);
    // let rotz_inv = rotz_inv.unwrap();
    assert_approx_eq!(Mat4::identity(), rotz * rotz_inv);
    assert_approx_eq!(Mat4::identity(), rotz_inv * rotz);

    let trans = Mat4::from_translation(vec3(1.0, 2.0, 3.0));
    let trans_inv = trans.inverse();
    // assert_ne!(None, trans_inv);
    // let trans_inv = trans_inv.unwrap();
    assert_approx_eq!(Mat4::identity(), trans * trans_inv);
    assert_approx_eq!(Mat4::identity(), trans_inv * trans);

    let scale = Mat4::from_scale(vec3(4.0, 5.0, 6.0));
    let scale_inv = scale.inverse();
    // assert_ne!(None, scale_inv);
    // let scale_inv = scale_inv.unwrap();
    assert_approx_eq!(Mat4::identity(), scale * scale_inv);
    assert_approx_eq!(Mat4::identity(), scale_inv * scale);

    let m = scale * rotz * trans;
    let m_inv = m.inverse();
    // assert_ne!(None, m_inv);
    // let m_inv = m_inv.unwrap();
    assert_approx_eq!(Mat4::identity(), m * m_inv, 1.0e-5);
    assert_approx_eq!(Mat4::identity(), m_inv * m, 1.0e-5);
    assert_approx_eq!(m_inv, trans_inv * rotz_inv * scale_inv, 1.0e-6);
}

#[test]
fn test_mat4_decompose() {
    // identity
    let (out_scale, out_rotation, out_translation) =
        Mat4::identity().to_scale_rotation_translation();
    assert_approx_eq!(Vec3::one(), out_scale);
    assert!(out_rotation.is_near_identity());
    assert_approx_eq!(Vec3::zero(), out_translation);

    // no scale
    let in_scale = Vec3::one();
    let in_translation = Vec3::new(-2.0, 4.0, -0.125);
    let in_rotation = Quat::from_rotation_ypr(
        f32::to_radians(-45.0),
        f32::to_radians(180.0),
        f32::to_radians(270.0),
    );
    let in_mat = Mat4::from_scale_rotation_translation(in_scale, in_rotation, in_translation);
    let (out_scale, out_rotation, out_translation) = in_mat.to_scale_rotation_translation();
    assert_approx_eq!(in_scale, out_scale, 1e-6);
    // out_rotation is different but produces the same matrix
    // assert_approx_eq!(in_rotation, out_rotation);
    assert_approx_eq!(in_translation, out_translation);
    assert_approx_eq!(
        in_mat,
        Mat4::from_scale_rotation_translation(out_scale, out_rotation, out_translation),
        1e-6
    );

    // positive scale
    let in_scale = Vec3::new(1.0, 2.0, 4.0);
    let in_mat = Mat4::from_scale_rotation_translation(in_scale, in_rotation, in_translation);
    let (out_scale, out_rotation, out_translation) = in_mat.to_scale_rotation_translation();
    assert_approx_eq!(in_scale, out_scale, 1e-6);
    // out_rotation is different but produces the same matrix
    // assert_approx_eq!(in_rotation, out_rotation);
    assert_approx_eq!(in_translation, out_translation);
    assert_approx_eq!(
        in_mat,
        Mat4::from_scale_rotation_translation(out_scale, out_rotation, out_translation),
        1e-6
    );

    // negative scale
    let in_scale = Vec3::new(-4.0, 1.0, 2.0);
    let in_mat = Mat4::from_scale_rotation_translation(in_scale, in_rotation, in_translation);
    let (out_scale, out_rotation, out_translation) = in_mat.to_scale_rotation_translation();
    assert_approx_eq!(in_scale, out_scale, 1e-6);
    // out_rotation is different but produces the same matrix
    // assert_approx_eq!(in_rotation, out_rotation);
    assert_approx_eq!(in_translation, out_translation);
    assert_approx_eq!(
        in_mat,
        Mat4::from_scale_rotation_translation(out_scale, out_rotation, out_translation),
        1e-5
    );

    // negative scale
    let in_scale = Vec3::new(4.0, -1.0, -2.0);
    let in_mat = Mat4::from_scale_rotation_translation(in_scale, in_rotation, in_translation);
    let (out_scale, out_rotation, out_translation) = in_mat.to_scale_rotation_translation();
    // out_scale and out_rotation are different but they produce the same matrix
    // assert_approx_eq!(in_scale, out_scale, 1e-6);
    // assert_approx_eq!(in_rotation, out_rotation);
    assert_approx_eq!(in_translation, out_translation);
    assert_approx_eq!(
        in_mat,
        Mat4::from_scale_rotation_translation(out_scale, out_rotation, out_translation),
        1e-6
    );
}

#[test]
fn test_mat4_look_at() {
    let eye = Vec3::new(0.0, 0.0, -5.0);
    let center = Vec3::new(0.0, 0.0, 0.0);
    let up = Vec3::new(1.0, 0.0, 0.0);
    let lh = Mat4::look_at_lh(eye, center, up);
    let rh = Mat4::look_at_rh(eye, center, up);
    let point = Vec3::new(1.0, 0.0, 0.0);
    assert_approx_eq!(lh.transform_point3(point), Vec3::new(0.0, 1.0, 5.0));
    assert_approx_eq!(rh.transform_point3(point), Vec3::new(0.0, 1.0, -5.0));
}

#[test]
fn test_mat4_perspective_gl_rh() {
    let projection = Mat4::perspective_rh_gl(f32::to_radians(90.0), 2.0, 5.0, 15.0);

    let original = Vec3::new(5.0, 5.0, -15.0);
    let projected = projection * original.extend(1.0);
    assert_approx_eq!(Vec4::new(2.5, 5.0, 15.0, 15.0), projected);

    let original = Vec3::new(5.0, 5.0, -5.0);
    let projected = projection * original.extend(1.0);
    assert_approx_eq!(Vec4::new(2.5, 5.0, -5.0, 5.0), projected);
}

#[test]
fn test_mat4_perspective_lh() {
    let projection = Mat4::perspective_lh(f32::to_radians(90.0), 2.0, 5.0, 15.0);

    let original = Vec3::new(5.0, 5.0, 15.0);
    let projected = projection * original.extend(1.0);
    assert_approx_eq!(Vec4::new(2.5, 5.0, 15.0, 15.0), projected);

    let original = Vec3::new(5.0, 5.0, 5.0);
    let projected = projection * original.extend(1.0);
    assert_approx_eq!(Vec4::new(2.5, 5.0, 0.0, 5.0), projected);
}

#[test]
fn test_mat4_perspective_infinite_lh() {
    let projection = Mat4::perspective_infinite_lh(f32::to_radians(90.0), 2.0, 5.0);

    let original = Vec3::new(5.0, 5.0, 15.0);
    let projected = projection * original.extend(1.0);
    assert_approx_eq!(Vec4::new(2.5, 5.0, 10.0, 15.0), projected);

    let original = Vec3::new(5.0, 5.0, 5.0);
    let projected = projection * original.extend(1.0);
    assert_approx_eq!(Vec4::new(2.5, 5.0, 0.0, 5.0), projected);
}

#[test]
fn test_mat4_perspective_infinite_reverse_lh() {
    let projection = Mat4::perspective_infinite_reverse_lh(f32::to_radians(90.0), 2.0, 5.0);

    let original = Vec3::new(5.0, 5.0, 15.0);
    let projected = projection * original.extend(1.0);
    assert_approx_eq!(Vec4::new(2.5, 5.0, 5.0, 15.0), projected);

    let original = Vec3::new(5.0, 5.0, 5.0);
    let projected = projection * original.extend(1.0);
    assert_approx_eq!(Vec4::new(2.5, 5.0, 5.0, 5.0), projected);
}

#[test]
fn test_mat4_orthographic_gl_rh() {
    let projection = Mat4::orthographic_rh_gl(-10.0, 10.0, -5.0, 5.0, 0.0, -10.0);
    let original = Vec4::new(5.0, 5.0, -5.0, 1.0);
    let projected = projection.mul_vec4(original);
    assert_approx_eq!(projected, Vec4::new(0.5, 1.0, -2.0, 1.0));
}

#[test]
fn test_mat4_orthographic_rh() {
    let projection = Mat4::orthographic_rh(-10.0, 10.0, -5.0, 5.0, -10.0, 10.0);
    let original = Vec4::new(5.0, 5.0, -5.0, 1.0);
    let projected = projection.mul_vec4(original);
    assert_approx_eq!(projected, Vec4::new(0.5, 1.0, 0.75, 1.0));

    let original = Vec4::new(5.0, 5.0, 5.0, 1.0);
    let projected = projection.mul_vec4(original);
    assert_approx_eq!(projected, Vec4::new(0.5, 1.0, 0.25, 1.0));
}

#[test]
fn test_mat4_orthographic_lh() {
    let projection = Mat4::orthographic_lh(-10.0, 10.0, -5.0, 5.0, -10.0, 10.0);
    let original = Vec4::new(5.0, 5.0, -5.0, 1.0);
    let projected = projection.mul_vec4(original);
    assert_approx_eq!(projected, Vec4::new(0.5, 1.0, 0.25, 1.0));

    let original = Vec4::new(5.0, 5.0, 5.0, 1.0);
    let projected = projection.mul_vec4(original);
    assert_approx_eq!(projected, Vec4::new(0.5, 1.0, 0.75, 1.0));
}

#[test]
fn test_mat4_ops() {
    let m0 = Mat4::from_cols_array_2d(&MATRIX);
    let m0x2 = Mat4::from_cols_array_2d(&[
        [2.0, 4.0, 6.0, 8.0],
        [10.0, 12.0, 14.0, 16.0],
        [18.0, 20.0, 22.0, 24.0],
        [26.0, 28.0, 30.0, 32.0],
    ]);
    assert_eq!(m0x2, m0 * 2.0);
    assert_eq!(m0x2, 2.0 * m0);
    assert_eq!(m0x2, m0 + m0);
    assert_eq!(Mat4::zero(), m0 - m0);
    assert_approx_eq!(m0, m0 * Mat4::identity());
    assert_approx_eq!(m0, Mat4::identity() * m0);
}

#[test]
fn test_mat4_fmt() {
    let a = Mat4::from_cols_array_2d(&MATRIX);
    assert_eq!(
        format!("{}", a),
        "[[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12], [13, 14, 15, 16]]"
    );
}

#[cfg(feature = "serde")]
#[test]
fn test_mat4_serde() {
    let a = Mat4::from_cols(
        vec4(1.0, 2.0, 3.0, 4.0),
        vec4(5.0, 6.0, 7.0, 8.0),
        vec4(9.0, 10.0, 11.0, 12.0),
        vec4(13.0, 14.0, 15.0, 16.0),
    );
    let serialized = serde_json::to_string(&a).unwrap();
    assert_eq!(
        serialized,
        "[1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0,10.0,11.0,12.0,13.0,14.0,15.0,16.0]"
    );
    let deserialized = serde_json::from_str(&serialized).unwrap();
    assert_eq!(a, deserialized);
    let deserialized = serde_json::from_str::<Mat4>("[]");
    assert!(deserialized.is_err());
    let deserialized = serde_json::from_str::<Mat4>("[1.0]");
    assert!(deserialized.is_err());
    let deserialized = serde_json::from_str::<Mat4>("[1.0,2.0]");
    assert!(deserialized.is_err());
    let deserialized = serde_json::from_str::<Mat4>("[1.0,2.0,3.0]");
    assert!(deserialized.is_err());
    let deserialized = serde_json::from_str::<Mat4>("[1.0,2.0,3.0,4.0,5.0]");
    assert!(deserialized.is_err());
    let deserialized = serde_json::from_str::<Mat4>("[[1.0,2.0,3.0],[4.0,5.0,6.0],[7.0,8.0,9.0]]");
    assert!(deserialized.is_err());
    let deserialized = serde_json::from_str::<Mat4>(
        "[[1.0,2.0,3.0,4.0],[5.0,6.0,7.0,8.0],[9.0,10.0,11.0,12.0][13.0,14.0,15.0,16.0]]",
    );
    assert!(deserialized.is_err());
}

#[cfg(feature = "rand")]
#[test]
fn test_mat4_rand() {
    use rand::{Rng, SeedableRng};
    use rand_xoshiro::Xoshiro256Plus;
    let mut rng1 = Xoshiro256Plus::seed_from_u64(0);
    let a = Mat4::from_cols_array(&rng1.gen::<[f32; 16]>());
    let mut rng2 = Xoshiro256Plus::seed_from_u64(0);
    let b = rng2.gen::<Mat4>();
    assert_eq!(a, b);
}
