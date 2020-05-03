mod support;

use glam::f32::{quat, Mat3, Mat4, Quat, Vec3, Vec4};
use support::{deg, rad};

#[test]
fn test_quat_align() {
    use std::mem;
    assert_eq!(16, mem::size_of::<Quat>());
    if cfg!(feature = "scalar-math") {
        assert_eq!(4, mem::align_of::<Quat>());
    } else {
        assert_eq!(16, mem::align_of::<Quat>());
    }
}

#[test]
fn test_quat_rotation() {
    let zero = deg(0.0);
    let yaw = deg(30.0);
    let pitch = deg(60.0);
    let roll = deg(90.0);
    let y0 = Quat::from_rotation_y(yaw);
    assert!(y0.is_normalized());
    let (axis, angle) = y0.to_axis_angle();
    assert_approx_eq!(axis, Vec3::unit_y(), 1.0e-6);
    assert_approx_eq!(angle, yaw);
    let y1 = Quat::from_rotation_ypr(yaw, zero, zero);
    assert_approx_eq!(y0, y1);
    let y2 = Quat::from_axis_angle(Vec3::unit_y(), yaw);
    assert_approx_eq!(y0, y2);
    let y3 = Quat::from_rotation_mat3(&Mat3::from_rotation_y(yaw));
    assert_approx_eq!(y0, y3);
    let y4 = Quat::from_rotation_mat3(&Mat3::from_quat(y0));
    assert_approx_eq!(y0, y4);

    let x0 = Quat::from_rotation_x(pitch);
    assert!(x0.is_normalized());
    let (axis, angle) = x0.to_axis_angle();
    assert_approx_eq!(axis, Vec3::unit_x());
    assert_approx_eq!(angle, pitch);
    let x1 = Quat::from_rotation_ypr(zero, pitch, zero);
    assert_approx_eq!(x0, x1);
    let x2 = Quat::from_axis_angle(Vec3::unit_x(), pitch);
    assert_approx_eq!(x0, x2);
    let x3 = Quat::from_rotation_mat4(&Mat4::from_rotation_x(deg(180.0)));
    assert!(x3.is_normalized());
    assert_approx_eq!(Quat::from_rotation_x(deg(180.0)), x3);

    let z0 = Quat::from_rotation_z(roll);
    assert!(z0.is_normalized());
    let (axis, angle) = z0.to_axis_angle();
    assert_approx_eq!(axis, Vec3::unit_z());
    assert_approx_eq!(angle, roll);
    let z1 = Quat::from_rotation_ypr(zero, zero, roll);
    assert_approx_eq!(z0, z1);
    let z2 = Quat::from_axis_angle(Vec3::unit_z(), roll);
    assert_approx_eq!(z0, z2);
    let z3 = Quat::from_rotation_mat4(&Mat4::from_rotation_z(roll));
    assert_approx_eq!(z0, z3);

    let yx0 = y0 * x0;
    assert!(yx0.is_normalized());
    let yx1 = Quat::from_rotation_ypr(yaw, pitch, zero);
    assert_approx_eq!(yx0, yx1);

    let yxz0 = y0 * x0 * z0;
    assert!(yxz0.is_normalized());
    let yxz1 = Quat::from_rotation_ypr(yaw, pitch, roll);
    assert_approx_eq!(yxz0, yxz1);

    // use the conjugate of z0 to remove the rotation from yxz0
    let yx2 = yxz0 * z0.conjugate();
    assert_approx_eq!(yx0, yx2);

    let yxz2 = Quat::from_rotation_mat4(&Mat4::from_quat(yxz0));
    assert_approx_eq!(yxz0, yxz2);

    // if near identity, just returns x axis and 0 rotation
    let (axis, angle) = Quat::identity().to_axis_angle();
    assert_eq!(axis, Vec3::unit_x());
    assert_eq!(angle, rad(0.0));
}

#[test]
fn test_quat_new() {
    let ytheta = deg(45.0);
    let q0 = Quat::from_rotation_y(ytheta);

    let t1 = (0.0, (ytheta * 0.5).sin(), 0.0, (ytheta * 0.5).cos());
    assert_eq!(q0, t1.into());
    let q1 = Quat::from(t1);
    assert_eq!(t1, q1.into());

    assert_eq!(q0, quat(t1.0, t1.1, t1.2, t1.3));

    let a1 = [0.0, (ytheta * 0.5).sin(), 0.0, (ytheta * 0.5).cos()];
    assert_eq!(q0, a1.into());
    let q1 = Quat::from(a1);
    let a2: [f32; 4] = q1.into();
    assert_eq!(a1, a2);
}

#[test]
fn test_quat_mul_vec() {
    let qrz = Quat::from_rotation_z(deg(90.0));
    assert_approx_eq!(Vec3::unit_y(), qrz * Vec3::unit_x());
    assert_approx_eq!(Vec3::unit_y(), -qrz * Vec3::unit_x());
    assert_approx_eq!(-Vec3::unit_x(), qrz * Vec3::unit_y());
    assert_approx_eq!(-Vec3::unit_x(), -qrz * Vec3::unit_y());

    // check vec3 * mat3 is the same
    let mrz = Mat3::from_quat(qrz);
    assert_approx_eq!(Vec3::unit_y(), mrz * Vec3::unit_x());
    // assert_approx_eq!(Vec3::unit_y(), -mrz * Vec3::unit_x());
    assert_approx_eq!(-Vec3::unit_x(), mrz * Vec3::unit_y());

    let qrx = Quat::from_rotation_x(deg(90.0));
    assert_approx_eq!(Vec3::unit_x(), qrx * Vec3::unit_x());
    assert_approx_eq!(Vec3::unit_x(), -qrx * Vec3::unit_x());
    assert_approx_eq!(Vec3::unit_z(), qrx * Vec3::unit_y());
    assert_approx_eq!(Vec3::unit_z(), -qrx * Vec3::unit_y());

    // check vec3 * mat3 is the same
    let mrx = Mat3::from_quat(qrx);
    assert_approx_eq!(Vec3::unit_x(), mrx * Vec3::unit_x());
    assert_approx_eq!(Vec3::unit_z(), mrx * Vec3::unit_y());

    let qrxz = qrz * qrx;
    assert_approx_eq!(Vec3::unit_y(), qrxz * Vec3::unit_x());
    assert_approx_eq!(Vec3::unit_z(), qrxz * Vec3::unit_y());

    let mrxz = mrz * mrx;
    assert_approx_eq!(Vec3::unit_y(), mrxz * Vec3::unit_x());
    assert_approx_eq!(Vec3::unit_z(), mrxz * Vec3::unit_y());

    let qrzx = qrx * qrz;
    assert_approx_eq!(Vec3::unit_z(), qrzx * Vec3::unit_x());
    assert_approx_eq!(-Vec3::unit_x(), qrzx * Vec3::unit_y());

    let mrzx = qrx * qrz;
    assert_approx_eq!(Vec3::unit_z(), mrzx * Vec3::unit_x());
    assert_approx_eq!(-Vec3::unit_x(), mrzx * Vec3::unit_y());
}

#[test]
fn test_quat_funcs() {
    let q0 = Quat::from_rotation_ypr(deg(45.0), deg(180.0), deg(90.0));
    assert!(q0.is_normalized());
    assert_approx_eq!(q0.length_squared(), 1.0);
    assert_approx_eq!(q0.length(), 1.0);
    assert_approx_eq!(q0.length_reciprocal(), 1.0);
    assert_approx_eq!(q0, q0.normalize());

    assert_approx_eq!(q0.dot(q0), 1.0);
    assert_approx_eq!(q0.dot(q0), 1.0);

    let q1 = Quat::from(Vec4::from(q0) * 2.0);
    assert!(!q1.is_normalized());
    assert_approx_eq!(q1.length_squared(), 4.0, 1.0e-6);
    assert_approx_eq!(q1.length(), 2.0);
    assert_approx_eq!(q1.length_reciprocal(), 0.5);
    assert_approx_eq!(q0, q1.normalize());
    assert_approx_eq!(q0.dot(q1), 2.0, 1.0e-6);
}

#[test]
fn test_quat_lerp() {
    let q0 = Quat::from_rotation_y(deg(0.0));
    let q1 = Quat::from_rotation_y(deg(90.0));
    assert_approx_eq!(q0, q0.lerp(q1, 0.0));
    assert_approx_eq!(q1, q0.lerp(q1, 1.0));
    assert_approx_eq!(Quat::from_rotation_y(deg(45.0)), q0.lerp(q1, 0.5));
}

#[test]
fn test_quat_slerp() {
    let q0 = Quat::from_rotation_y(deg(0.0));
    let q1 = Quat::from_rotation_y(deg(90.0));
    assert_approx_eq!(q0, q0.slerp(q1, 0.0), 1.0e-3);
    assert_approx_eq!(q1, q0.slerp(q1, 1.0), 1.0e-3);
    assert_approx_eq!(Quat::from_rotation_y(deg(45.0)), q0.slerp(q1, 0.5), 1.0e-3);
}

#[test]
fn test_quat_slerp_constant_speed() {
    let step = 0.01;
    let mut s = 0.0;
    while s <= 1.0 {
        let q0 = Quat::from_rotation_y(deg(0.0));
        let q1 = Quat::from_rotation_y(deg(90.0));
        assert_approx_eq!(
            Quat::from_rotation_y(deg(s * 90.0)),
            q0.slerp(q1, s),
            1.0e-3
        );
        s += step;
    }
}

#[test]
fn test_quat_fmt() {
    let a = Quat::identity();
    #[cfg(all(target_feature = "sse2", not(feature = "scalar-math")))]
    assert_eq!(format!("{:?}", a), "Quat(__m128(0.0, 0.0, 0.0, 1.0))");
    #[cfg(any(not(target_feature = "sse2"), feature = "scalar-math"))]
    assert_eq!(format!("{:?}", a), "Quat(0.0, 0.0, 0.0, 1.0)");
    // assert_eq!(
    //     format!("{:#?}", a),
    //     "Quat(\n    1.0,\n    2.0,\n    3.0,\n    4.0\n)"
    // );
    assert_eq!(format!("{}", a), "[0, 0, 0, 1]");
}

#[test]
fn test_quat_identity() {
    let identity = Quat::identity();
    assert!(identity.is_near_identity());
    assert!(identity.is_normalized());
    assert_eq!(identity, Quat::from_xyzw(0.0, 0.0, 0.0, 1.0));
    assert_eq!(identity, identity * identity);
    let q = Quat::from_rotation_ypr(deg(10.0), deg(-10.0), deg(45.0));
    assert_eq!(q, q * identity);
    assert_eq!(q, identity * q);
    assert_eq!(identity, Quat::default());
}

#[test]
fn test_quat_slice() {
    let a: [f32; 4] = Quat::from_rotation_ypr(deg(30.0), deg(60.0), deg(90.0)).into();
    let b = Quat::from_slice_unaligned(&a);
    let c: [f32; 4] = b.into();
    assert_eq!(a, c);
    let mut d = [0.0, 0.0, 0.0, 0.0];
    b.write_to_slice_unaligned(&mut d[..]);
    assert_eq!(a, d);
}

#[test]
fn test_quat_elements() {
    let x = 1.0;
    let y = 2.0;
    let z = 3.0;
    let w = 4.0;

    let a = Quat::from_xyzw(x, y, z, w);
    assert!(a.x() == x);
    assert!(a.y() == y);
    assert!(a.z() == z);
    assert!(a.w() == w);
}

#[cfg(feature = "serde")]
#[test]
fn test_quat_serde() {
    let a = Quat::from_xyzw(1.0, 2.0, 3.0, 4.0);
    let serialized = serde_json::to_string(&a).unwrap();
    assert_eq!(serialized, "[1.0,2.0,3.0,4.0]");
    let deserialized = serde_json::from_str(&serialized).unwrap();
    assert_eq!(a, deserialized);
    let deserialized = serde_json::from_str::<Quat>("[]");
    assert!(deserialized.is_err());
    let deserialized = serde_json::from_str::<Quat>("[1.0]");
    assert!(deserialized.is_err());
    let deserialized = serde_json::from_str::<Quat>("[1.0,2.0]");
    assert!(deserialized.is_err());
    let deserialized = serde_json::from_str::<Quat>("[1.0,2.0,3.0]");
    assert!(deserialized.is_err());
    let deserialized = serde_json::from_str::<Quat>("[1.0,2.0,3.0,4.0,5.0]");
    assert!(deserialized.is_err());
}

#[cfg(feature = "rand")]
#[test]
fn test_quat_rand() {
    use rand::{Rng, SeedableRng};
    use rand_xoshiro::Xoshiro256Plus;
    let mut rng1 = Xoshiro256Plus::seed_from_u64(0);
    let a: Quat = rng1.gen();
    assert!(a.is_normalized());
    let mut rng2 = Xoshiro256Plus::seed_from_u64(0);
    let b: Quat = rng2.gen();
    assert_eq!(a, b);
}
