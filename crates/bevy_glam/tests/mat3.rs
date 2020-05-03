mod support;

use glam::f32::*;
use support::deg;

const IDENTITY: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

const MATRIX: [[f32; 3]; 3] = [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]];

const ZERO: [[f32; 3]; 3] = [[0.0; 3]; 3];

#[test]
fn test_mat3_align() {
    use std::mem;
    if cfg!(any(feature = "packed-vec3", feature = "scalar-math")) {
        assert_eq!(36, mem::size_of::<Mat3>());
        assert_eq!(4, mem::align_of::<Mat3>());
    } else {
        assert_eq!(48, mem::size_of::<Mat3>());
        assert_eq!(16, mem::align_of::<Mat3>());
    }
}

#[test]
fn test_mat3_identity() {
    let identity = Mat3::identity();
    assert_eq!(IDENTITY, identity.to_cols_array_2d());
    assert_eq!(Mat3::from_cols_array_2d(&IDENTITY), identity);
    assert_eq!(identity, identity * identity);
    assert_eq!(identity, Mat3::default());
}

#[test]
fn test_mat3_zero() {
    assert_eq!(Mat3::from_cols_array_2d(&ZERO), Mat3::zero());
}

#[test]
fn test_mat3_accessors() {
    let mut m = Mat3::zero();
    m.set_x_axis(Vec3::new(1.0, 2.0, 3.0));
    m.set_y_axis(Vec3::new(4.0, 5.0, 6.0));
    m.set_z_axis(Vec3::new(7.0, 8.0, 9.0));
    assert_eq!(Mat3::from_cols_array_2d(&MATRIX), m);
    assert_eq!(Vec3::new(1.0, 2.0, 3.0), m.x_axis());
    assert_eq!(Vec3::new(4.0, 5.0, 6.0), m.y_axis());
    assert_eq!(Vec3::new(7.0, 8.0, 9.0), m.z_axis());
}

#[test]
fn test_mat3_from_axes() {
    let a = Mat3::from_cols_array_2d(&[[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]]);
    assert_eq!(MATRIX, a.to_cols_array_2d());
    let b = Mat3::from_cols(
        vec3(1.0, 2.0, 3.0),
        vec3(4.0, 5.0, 6.0),
        vec3(7.0, 8.0, 9.0),
    );
    assert_eq!(a, b);
    let c = mat3(
        vec3(1.0, 2.0, 3.0),
        vec3(4.0, 5.0, 6.0),
        vec3(7.0, 8.0, 9.0),
    );
    assert_eq!(a, c);
    let d = b.to_cols_array();
    let f = Mat3::from_cols_array(&d);
    assert_eq!(b, f);
}

#[test]
fn test_from_rotation() {
    let rot_x1 = Mat3::from_rotation_x(deg(180.0));
    let rot_x2 = Mat3::from_axis_angle(Vec3::unit_x(), deg(180.0));
    assert_approx_eq!(rot_x1, rot_x2);
    let rot_y1 = Mat3::from_rotation_y(deg(180.0));
    let rot_y2 = Mat3::from_axis_angle(Vec3::unit_y(), deg(180.0));
    assert_approx_eq!(rot_y1, rot_y2);
    let rot_z1 = Mat3::from_rotation_z(deg(180.0));
    let rot_z2 = Mat3::from_axis_angle(Vec3::unit_z(), deg(180.0));
    assert_approx_eq!(rot_z1, rot_z2);
}

#[test]
fn test_mat3_mul() {
    let mat_a = Mat3::from_axis_angle(Vec3::unit_z(), deg(90.0));
    let result3 = mat_a * Vec3::unit_y();
    assert_approx_eq!(vec3(-1.0, 0.0, 0.0), result3);
}

#[test]
fn test_mat3_transform2d() {
    let mat_b = Mat3::from_scale_angle_translation(
        Vec2::new(0.5, 1.5),
        f32::to_radians(90.0),
        Vec2::new(1.0, 2.0),
    );
    let result2 = mat_b.transform_vector2(Vec2::unit_y());
    assert_approx_eq!(vec2(-1.5, 0.0), result2, 1.0e-6);
    assert_approx_eq!(result2, (mat_b * Vec2::unit_y().extend(0.0)).truncate());

    let result2 = mat_b.transform_point2(Vec2::unit_y());
    assert_approx_eq!(vec2(-0.5, 2.0), result2, 1.0e-6);
    assert_approx_eq!(result2, (mat_b * Vec2::unit_y().extend(1.0)).truncate());
}

#[test]
fn test_from_ypr() {
    let zero = deg(0.0);
    let yaw = deg(30.0);
    let pitch = deg(60.0);
    let roll = deg(90.0);
    let y0 = Mat3::from_rotation_y(yaw);
    let y1 = Mat3::from_rotation_ypr(yaw, zero, zero);
    assert_approx_eq!(y0, y1);

    let x0 = Mat3::from_rotation_x(pitch);
    let x1 = Mat3::from_rotation_ypr(zero, pitch, zero);
    assert_approx_eq!(x0, x1);

    let z0 = Mat3::from_rotation_z(roll);
    let z1 = Mat3::from_rotation_ypr(zero, zero, roll);
    assert_approx_eq!(z0, z1);

    let yx0 = y0 * x0;
    let yx1 = Mat3::from_rotation_ypr(yaw, pitch, zero);
    assert_approx_eq!(yx0, yx1);

    let yxz0 = y0 * x0 * z0;
    let yxz1 = Mat3::from_rotation_ypr(yaw, pitch, roll);
    assert_approx_eq!(yxz0, yxz1, 1e-6);
}

#[test]
fn test_from_scale() {
    let m = Mat3::from_scale(Vec3::new(2.0, 4.0, 8.0));
    assert_approx_eq!(m * Vec3::new(1.0, 1.0, 1.0), Vec3::new(2.0, 4.0, 8.0));
    assert_approx_eq!(Vec3::unit_x() * 2.0, m.x_axis());
    assert_approx_eq!(Vec3::unit_y() * 4.0, m.y_axis());
    assert_approx_eq!(Vec3::unit_z() * 8.0, m.z_axis());
}

#[test]
fn test_mat3_transpose() {
    let m = mat3(
        vec3(1.0, 2.0, 3.0),
        vec3(4.0, 5.0, 6.0),
        vec3(7.0, 8.0, 9.0),
    );
    let mt = m.transpose();
    assert_eq!(mt.x_axis(), vec3(1.0, 4.0, 7.0));
    assert_eq!(mt.y_axis(), vec3(2.0, 5.0, 8.0));
    assert_eq!(mt.z_axis(), vec3(3.0, 6.0, 9.0));
}

#[test]
fn test_mat3_det() {
    assert_eq!(0.0, Mat3::zero().determinant());
    assert_eq!(1.0, Mat3::identity().determinant());
    assert_eq!(1.0, Mat3::from_rotation_x(deg(90.0)).determinant());
    assert_eq!(1.0, Mat3::from_rotation_y(deg(180.0)).determinant());
    assert_eq!(1.0, Mat3::from_rotation_z(deg(270.0)).determinant());
    assert_eq!(
        2.0 * 2.0 * 2.0,
        Mat3::from_scale(vec3(2.0, 2.0, 2.0)).determinant()
    );
}

#[test]
fn test_mat3_inverse() {
    // assert_eq!(None, Mat3::zero().inverse());
    let inv = Mat3::identity().inverse();
    // assert_ne!(None, inv);
    assert_approx_eq!(Mat3::identity(), inv);

    let rotz = Mat3::from_rotation_z(deg(90.0));
    let rotz_inv = rotz.inverse();
    // assert_ne!(None, rotz_inv);
    // let rotz_inv = rotz_inv.unwrap();
    assert_approx_eq!(Mat3::identity(), rotz * rotz_inv);
    assert_approx_eq!(Mat3::identity(), rotz_inv * rotz);

    let scale = Mat3::from_scale(vec3(4.0, 5.0, 6.0));
    let scale_inv = scale.inverse();
    // assert_ne!(None, scale_inv);
    // let scale_inv = scale_inv.unwrap();
    assert_approx_eq!(Mat3::identity(), scale * scale_inv);
    assert_approx_eq!(Mat3::identity(), scale_inv * scale);

    let m = scale * rotz;
    let m_inv = m.inverse();
    // assert_ne!(None, m_inv);
    // let m_inv = m_inv.unwrap();
    assert_approx_eq!(Mat3::identity(), m * m_inv);
    assert_approx_eq!(Mat3::identity(), m_inv * m);
    assert_approx_eq!(m_inv, rotz_inv * scale_inv);
}

#[test]
fn test_mat3_ops() {
    let m0 = Mat3::from_cols_array_2d(&MATRIX);
    let m0x2 = Mat3::from_cols_array_2d(&[[2.0, 4.0, 6.0], [8.0, 10.0, 12.0], [14.0, 16.0, 18.0]]);
    assert_eq!(m0x2, m0 * 2.0);
    assert_eq!(m0x2, 2.0 * m0);
    assert_eq!(m0x2, m0 + m0);
    assert_eq!(Mat3::zero(), m0 - m0);
    assert_approx_eq!(m0, m0 * Mat3::identity());
    assert_approx_eq!(m0, Mat3::identity() * m0);
}

#[test]
fn test_mat3_fmt() {
    let a = Mat3::from_cols_array_2d(&MATRIX);
    assert_eq!(format!("{}", a), "[[1, 2, 3], [4, 5, 6], [7, 8, 9]]");
}

#[cfg(feature = "serde")]
#[test]
fn test_mat3_serde() {
    let a = Mat3::from_cols(
        vec3(1.0, 2.0, 3.0),
        vec3(4.0, 5.0, 6.0),
        vec3(7.0, 8.0, 9.0),
    );
    let serialized = serde_json::to_string(&a).unwrap();
    assert_eq!(serialized, "[1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0]");
    let deserialized = serde_json::from_str(&serialized).unwrap();
    assert_eq!(a, deserialized);
    let deserialized = serde_json::from_str::<Mat3>("[]");
    assert!(deserialized.is_err());
    let deserialized = serde_json::from_str::<Mat3>("[1.0]");
    assert!(deserialized.is_err());
    let deserialized = serde_json::from_str::<Mat3>("[1.0,2.0]");
    assert!(deserialized.is_err());
    let deserialized = serde_json::from_str::<Mat3>("[1.0,2.0,3.0]");
    assert!(deserialized.is_err());
    let deserialized = serde_json::from_str::<Mat3>("[1.0,2.0,3.0,4.0,5.0]");
    assert!(deserialized.is_err());
    let deserialized = serde_json::from_str::<Mat3>("[[1.0,2.0,3.0],[4.0,5.0,6.0],[7.0,8.0,9.0]]");
    assert!(deserialized.is_err());
}

#[cfg(feature = "rand")]
#[test]
fn test_mat3_rand() {
    use rand::{Rng, SeedableRng};
    use rand_xoshiro::Xoshiro256Plus;
    let mut rng1 = Xoshiro256Plus::seed_from_u64(0);
    let a = Mat3::from_cols_array(&rng1.gen::<[f32; 9]>());
    let mut rng2 = Xoshiro256Plus::seed_from_u64(0);
    let b = rng2.gen::<Mat3>();
    assert_eq!(a, b);
}
