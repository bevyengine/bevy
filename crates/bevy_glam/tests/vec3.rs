mod support;

use glam::*;
use std::f32;

#[test]
fn test_vec3_align() {
    use std::mem;
    if cfg!(any(feature = "packed-vec3", feature = "scalar-math")) {
        assert_eq!(12, mem::size_of::<Vec3>());
        assert_eq!(4, mem::align_of::<Vec3>());
        assert_eq!(12, mem::size_of::<Vec3Mask>());
        assert_eq!(4, mem::align_of::<Vec3Mask>());
    } else {
        assert_eq!(16, mem::size_of::<Vec3>());
        assert_eq!(16, mem::align_of::<Vec3>());
        assert_eq!(16, mem::size_of::<Vec3Mask>());
        assert_eq!(16, mem::align_of::<Vec3Mask>());
    }
}

#[test]
fn test_vec3_new() {
    let v = vec3(1.0, 2.0, 3.0);

    assert_eq!(v.x(), 1.0);
    assert_eq!(v.y(), 2.0);
    assert_eq!(v.z(), 3.0);

    let t = (1.0, 2.0, 3.0);
    let v = Vec3::from(t);
    assert_eq!(t, v.into());

    let a = [1.0, 2.0, 3.0];
    let v = Vec3::from(a);
    let a1: [f32; 3] = v.into();
    assert_eq!(a, a1);

    let v = Vec3::new(t.0, t.1, t.2);
    assert_eq!(t, v.into());

    assert_eq!(Vec3::new(1.0, 0.0, 0.0), Vec3::unit_x());
    assert_eq!(Vec3::new(0.0, 1.0, 0.0), Vec3::unit_y());
    assert_eq!(Vec3::new(0.0, 0.0, 1.0), Vec3::unit_z());
}

#[test]
fn test_vec3_fmt() {
    let a = Vec3::new(1.0, 2.0, 3.0);
    #[cfg(all(
        target_feature = "sse2",
        not(feature = "packed-vec3"),
        not(feature = "scalar-math")
    ))]
    assert_eq!(format!("{:?}", a), "Vec3(__m128(1.0, 2.0, 3.0, 3.0))");
    #[cfg(any(
        not(target_feature = "sse2"),
        feature = "packed-vec3",
        feature = "scalar-math"
    ))]
    assert_eq!(format!("{:?}", a), "Vec3(1.0, 2.0, 3.0)");
    // assert_eq!(format!("{:#?}", a), "Vec3(\n    1.0,\n    2.0,\n    3.0\n)");
    assert_eq!(format!("{}", a), "[1, 2, 3]");
}

#[test]
fn test_vec3_zero() {
    let v = Vec3::zero();
    assert_eq!((0.0, 0.0, 0.0), v.into());
    assert_eq!(v, Vec3::default());
}

#[test]
fn test_vec3_splat() {
    let v = Vec3::splat(1.0);
    assert_eq!((1.0, 1.0, 1.0), v.into());
}

#[test]
fn test_vec3_accessors() {
    let mut a = Vec3::zero();
    a.set_x(1.0);
    a.set_y(2.0);
    a.set_z(3.0);
    assert_eq!(1.0, a.x());
    assert_eq!(2.0, a.y());
    assert_eq!(3.0, a.z());
    assert_eq!((1.0, 2.0, 3.0), a.into());

    let mut a = Vec3::zero();
    *a.x_mut() = 1.0;
    *a.y_mut() = 2.0;
    *a.z_mut() = 3.0;
    assert_eq!(1.0, a.x());
    assert_eq!(2.0, a.y());
    assert_eq!(3.0, a.z());
    assert_eq!((1.0, 2.0, 3.0), a.into());

    let mut a = Vec3::zero();
    a[0] = 1.0;
    a[1] = 2.0;
    a[2] = 3.0;
    assert_eq!(1.0, a[0]);
    assert_eq!(2.0, a[1]);
    assert_eq!(3.0, a[2]);
    assert_eq!((1.0, 2.0, 3.0), a.into());
}

#[test]
fn test_vec3_funcs() {
    let x = vec3(1.0, 0.0, 0.0);
    let y = vec3(0.0, 1.0, 0.0);
    let z = vec3(0.0, 0.0, 1.0);
    assert_eq!(1.0, x.dot(x));
    assert_eq!(0.0, x.dot(y));
    assert_eq!(-1.0, z.dot(-z));
    assert_eq!(y, z.cross(x));
    assert_eq!(z, x.cross(y));
    assert_eq!(4.0, (2.0 * x).length_squared());
    assert_eq!(9.0, (-3.0 * y).length_squared());
    assert_eq!(16.0, (4.0 * z).length_squared());
    assert_eq!(2.0, (-2.0 * x).length());
    assert_eq!(3.0, (3.0 * y).length());
    assert_eq!(4.0, (-4.0 * z).length());
    assert_eq!(x, (2.0 * x).normalize());
    assert_eq!(
        1.0 * 4.0 + 2.0 * 5.0 + 3.0 * 6.0,
        vec3(1.0, 2.0, 3.0).dot(vec3(4.0, 5.0, 6.0))
    );
    assert_eq!(
        2.0 * 2.0 + 3.0 * 3.0 + 4.0 * 4.0,
        vec3(2.0, 3.0, 4.0).length_squared()
    );
    assert_eq!(
        (2.0_f32 * 2.0 + 3.0 * 3.0 + 4.0 * 4.0).sqrt(),
        vec3(2.0, 3.0, 4.0).length()
    );
    assert_eq!(
        1.0 / (2.0_f32 * 2.0 + 3.0 * 3.0 + 4.0 * 4.0).sqrt(),
        vec3(2.0, 3.0, 4.0).length_reciprocal()
    );
    assert!(vec3(2.0, 3.0, 4.0).normalize().is_normalized());
    assert_approx_eq!(
        vec3(2.0, 3.0, 4.0) / (2.0_f32 * 2.0 + 3.0 * 3.0 + 4.0 * 4.0).sqrt(),
        vec3(2.0, 3.0, 4.0).normalize()
    );
    assert_eq!(vec3(0.5, 0.25, 0.125), vec3(2.0, 4.0, 8.0).reciprocal());
}

#[test]
fn test_vec3_ops() {
    let a = vec3(1.0, 2.0, 3.0);
    assert_eq!((2.0, 4.0, 6.0), (a + a).into());
    assert_eq!((0.0, 0.0, 0.0), (a - a).into());
    assert_eq!((1.0, 4.0, 9.0), (a * a).into());
    assert_eq!((2.0, 4.0, 6.0), (a * 2.0).into());
    assert_eq!((1.0, 1.0, 1.0), (a / a).into());
    assert_eq!((0.5, 1.0, 1.5), (a / 2.0).into());
    assert_eq!((-1.0, -2.0, -3.0), (-a).into());
}

#[test]
fn test_vec3_assign_ops() {
    let a = vec3(1.0, 2.0, 3.0);
    let mut b = a;
    b += a;
    assert_eq!((2.0, 4.0, 6.0), b.into());
    b -= a;
    assert_eq!((1.0, 2.0, 3.0), b.into());
    b *= a;
    assert_eq!((1.0, 4.0, 9.0), b.into());
    b /= a;
    assert_eq!((1.0, 2.0, 3.0), b.into());
    b *= 2.0;
    assert_eq!((2.0, 4.0, 6.0), b.into());
    b /= 2.0;
    assert_eq!((1.0, 2.0, 3.0), b.into());
}

#[test]
fn test_vec3_min_max() {
    let a = vec3(-1.0, 2.0, -3.0);
    let b = vec3(1.0, -2.0, 3.0);
    assert_eq!((-1.0, -2.0, -3.0), a.min(b).into());
    assert_eq!((-1.0, -2.0, -3.0), b.min(a).into());
    assert_eq!((1.0, 2.0, 3.0), a.max(b).into());
    assert_eq!((1.0, 2.0, 3.0), b.max(a).into());
}

#[test]
fn test_vec3_hmin_hmax() {
    let a = vec3(-1.0, 2.0, -3.0);
    assert_eq!(-3.0, a.min_element());
    assert_eq!(2.0, a.max_element());
}

#[test]
fn test_vec3_eq() {
    let a = vec3(1.0, 1.0, 1.0);
    let b = vec3(1.0, 2.0, 3.0);
    assert!(a.cmpeq(a).all());
    assert!(b.cmpeq(b).all());
    assert!(a.cmpne(b).any());
    assert!(b.cmpne(a).any());
    assert!(b.cmpeq(a).any());
}

#[test]
fn test_vec3_cmp() {
    assert!(!Vec3Mask::default().any());
    assert!(!Vec3Mask::default().all());
    assert_eq!(Vec3Mask::default().bitmask(), 0x0);
    let a = vec3(-1.0, -1.0, -1.0);
    let b = vec3(1.0, 1.0, 1.0);
    let c = vec3(-1.0, -1.0, 1.0);
    let d = vec3(1.0, -1.0, -1.0);
    assert_eq!(a.cmplt(a).bitmask(), 0x0);
    assert_eq!(a.cmplt(b).bitmask(), 0x7);
    assert_eq!(a.cmplt(c).bitmask(), 0x4);
    assert_eq!(c.cmple(a).bitmask(), 0x3);
    assert_eq!(a.cmplt(d).bitmask(), 0x1);
    assert!(a.cmplt(b).all());
    assert!(a.cmplt(c).any());
    assert!(a.cmple(b).all());
    assert!(a.cmple(a).all());
    assert!(b.cmpgt(a).all());
    assert!(b.cmpge(a).all());
    assert!(b.cmpge(b).all());
    assert!(!(a.cmpge(c).all()));
    assert!(c.cmple(c).all());
    assert!(c.cmpge(c).all());
    assert!(a == a);
    assert!(a < b);
    assert!(b > a);
}

#[test]
fn test_extend_truncate() {
    let a = vec3(1.0, 2.0, 3.0);
    let b = a.extend(4.0);
    assert_eq!((1.0, 2.0, 3.0, 4.0), b.into());
    let c = b.truncate();
    assert_eq!(a, c);
}

#[test]
fn test_vec3b() {
    // make sure the unused 'w' value doesn't break Vec3b behaviour
    let a = Vec4::zero();
    let mut b = a.truncate();
    b.set_x(1.0);
    b.set_y(1.0);
    b.set_z(1.0);
    assert!(!b.cmpeq(Vec3::zero()).any());
    assert!(b.cmpeq(Vec3::splat(1.0)).all());
}

#[test]
fn test_vec3mask_as_ref() {
    assert_eq!(Vec3Mask::new(false, false, false).as_ref(), &[0, 0, 0]);
    assert_eq!(Vec3Mask::new(true, false, false).as_ref(), &[!0, 0, 0]);
    assert_eq!(Vec3Mask::new(false, true, true).as_ref(), &[0, !0, !0]);
    assert_eq!(Vec3Mask::new(false, true, false).as_ref(), &[0, !0, 0]);
    assert_eq!(Vec3Mask::new(true, false, true).as_ref(), &[!0, 0, !0]);
    assert_eq!(Vec3Mask::new(true, true, true).as_ref(), &[!0, !0, !0]);
}

#[test]
fn test_vec3mask_from() {
    assert_eq!(
        Into::<[u32; 3]>::into(Vec3Mask::new(false, false, false)),
        [0, 0, 0]
    );
    assert_eq!(
        Into::<[u32; 3]>::into(Vec3Mask::new(true, false, false)),
        [!0, 0, 0]
    );
    assert_eq!(
        Into::<[u32; 3]>::into(Vec3Mask::new(false, true, true)),
        [0, !0, !0]
    );
    assert_eq!(
        Into::<[u32; 3]>::into(Vec3Mask::new(false, true, false)),
        [0, !0, 0]
    );
    assert_eq!(
        Into::<[u32; 3]>::into(Vec3Mask::new(true, false, true)),
        [!0, 0, !0]
    );
    assert_eq!(
        Into::<[u32; 3]>::into(Vec3Mask::new(true, true, true)),
        [!0, !0, !0]
    );
}

#[test]
fn test_vec3mask_bitmask() {
    assert_eq!(Vec3Mask::new(false, false, false).bitmask(), 0b000);
    assert_eq!(Vec3Mask::new(true, false, false).bitmask(), 0b001);
    assert_eq!(Vec3Mask::new(false, true, true).bitmask(), 0b110);
    assert_eq!(Vec3Mask::new(false, true, false).bitmask(), 0b010);
    assert_eq!(Vec3Mask::new(true, false, true).bitmask(), 0b101);
    assert_eq!(Vec3Mask::new(true, true, true).bitmask(), 0b111);
}

#[test]
fn test_vec3mask_any() {
    assert_eq!(Vec3Mask::new(false, false, false).any(), false);
    assert_eq!(Vec3Mask::new(true, false, false).any(), true);
    assert_eq!(Vec3Mask::new(false, true, false).any(), true);
    assert_eq!(Vec3Mask::new(false, false, true).any(), true);
}

#[test]
fn test_vec3mask_all() {
    assert_eq!(Vec3Mask::new(true, true, true).all(), true);
    assert_eq!(Vec3Mask::new(false, true, true).all(), false);
    assert_eq!(Vec3Mask::new(true, false, true).all(), false);
    assert_eq!(Vec3Mask::new(true, true, false).all(), false);
}

#[test]
fn test_vec3mask_select() {
    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(4.0, 5.0, 6.0);
    assert_eq!(
        Vec3Mask::new(true, true, true).select(a, b),
        Vec3::new(1.0, 2.0, 3.0),
    );
    assert_eq!(
        Vec3Mask::new(true, false, true).select(a, b),
        Vec3::new(1.0, 5.0, 3.0),
    );
    assert_eq!(
        Vec3Mask::new(false, true, false).select(a, b),
        Vec3::new(4.0, 2.0, 6.0),
    );
    assert_eq!(
        Vec3Mask::new(false, false, false).select(a, b),
        Vec3::new(4.0, 5.0, 6.0),
    );
}

#[test]
fn test_vec3mask_and() {
    assert_eq!(
        (Vec3Mask::new(false, false, false) & Vec3Mask::new(false, false, false)).bitmask(),
        0b000,
    );
    assert_eq!(
        (Vec3Mask::new(true, true, true) & Vec3Mask::new(true, true, true)).bitmask(),
        0b111,
    );
    assert_eq!(
        (Vec3Mask::new(true, false, true) & Vec3Mask::new(false, true, false)).bitmask(),
        0b000,
    );
    assert_eq!(
        (Vec3Mask::new(true, false, true) & Vec3Mask::new(true, true, true)).bitmask(),
        0b101,
    );

    let mut mask = Vec3Mask::new(true, true, false);
    mask &= Vec3Mask::new(true, false, false);
    assert_eq!(mask.bitmask(), 0b001);
}

#[test]
fn test_vec3mask_or() {
    assert_eq!(
        (Vec3Mask::new(false, false, false) | Vec3Mask::new(false, false, false)).bitmask(),
        0b000,
    );
    assert_eq!(
        (Vec3Mask::new(true, true, true) | Vec3Mask::new(true, true, true)).bitmask(),
        0b111,
    );
    assert_eq!(
        (Vec3Mask::new(true, false, true) | Vec3Mask::new(false, true, false)).bitmask(),
        0b111,
    );
    assert_eq!(
        (Vec3Mask::new(true, false, true) | Vec3Mask::new(true, false, true)).bitmask(),
        0b101,
    );

    let mut mask = Vec3Mask::new(true, true, false);
    mask |= Vec3Mask::new(true, false, false);
    assert_eq!(mask.bitmask(), 0b011);
}

#[test]
fn test_vec3mask_not() {
    assert_eq!((!Vec3Mask::new(false, false, false)).bitmask(), 0b111);
    assert_eq!((!Vec3Mask::new(true, true, true)).bitmask(), 0b000);
    assert_eq!((!Vec3Mask::new(true, false, true)).bitmask(), 0b010);
    assert_eq!((!Vec3Mask::new(false, true, false)).bitmask(), 0b101);
}

#[test]
fn test_vec3mask_fmt() {
    let a = Vec3Mask::new(true, false, false);

    // debug fmt
    #[cfg(all(
        target_feature = "sse2",
        not(feature = "packed-vec3"),
        not(feature = "scalar-math")
    ))]
    assert_eq!(format!("{:?}", a), "Vec3Mask(0xffffffff, 0x0, 0x0)");

    #[cfg(any(
        not(target_feature = "sse2"),
        feature = "packed-vec3",
        feature = "scalar-math"
    ))]
    assert_eq!(format!("{:?}", a), "Vec3Mask(0xffffffff, 0x0, 0x0)");

    // display fmt
    #[cfg(all(
        target_feature = "sse2",
        not(feature = "packed-vec3"),
        not(feature = "scalar-math")
    ))]
    assert_eq!(format!("{}", a), "[true, false, false]");

    #[cfg(any(
        not(target_feature = "sse2"),
        feature = "packed-vec3",
        feature = "scalar-math"
    ))]
    assert_eq!(format!("{}", a), "[true, false, false]");
}

#[test]
fn test_vec3mask_eq() {
    let a = Vec3Mask::new(true, false, true);
    let b = Vec3Mask::new(true, false, true);
    let c = Vec3Mask::new(false, true, true);

    assert_eq!(a, b);
    assert_eq!(b, a);
    assert_ne!(a, c);
    assert_ne!(b, c);

    assert!(a > c);
    assert!(c < a);
}

#[test]
fn test_vec3mask_hash() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hash;
    use std::hash::Hasher;

    let a = Vec3Mask::new(true, false, true);
    let b = Vec3Mask::new(true, false, true);
    let c = Vec3Mask::new(false, true, true);

    let mut hasher = DefaultHasher::new();
    a.hash(&mut hasher);
    let a_hashed = hasher.finish();

    let mut hasher = DefaultHasher::new();
    b.hash(&mut hasher);
    let b_hashed = hasher.finish();

    let mut hasher = DefaultHasher::new();
    c.hash(&mut hasher);
    let c_hashed = hasher.finish();

    assert_eq!(a, b);
    assert_eq!(a_hashed, b_hashed);
    assert_ne!(a, c);
    assert_ne!(a_hashed, c_hashed);
}

#[test]
fn test_vec3_sign() {
    assert_eq!(Vec3::zero().sign(), Vec3::one());
    assert_eq!(-Vec3::zero().sign(), -Vec3::one());
    assert_eq!(Vec3::one().sign(), Vec3::one());
    assert_eq!((-Vec3::one()).sign(), -Vec3::one());
    assert_eq!(Vec3::splat(core::f32::NEG_INFINITY).sign(), -Vec3::one());
}

#[test]
fn test_vec3_abs() {
    assert_eq!(Vec3::zero().abs(), Vec3::zero());
    assert_eq!(Vec3::one().abs(), Vec3::one());
    assert_eq!((-Vec3::one()).abs(), Vec3::one());
}

#[test]
fn test_vec3_round() {
    assert_eq!(Vec3::new(1.35, 0.0, 0.0).round().x(), 1.0);
    assert_eq!(Vec3::new(0.0, 1.5, 0.0).round().y(), 2.0);
    assert_eq!(Vec3::new(0.0, 0.0, -15.5).round().z(), -16.0);
    assert_eq!(Vec3::new(0.0, 0.0, 0.0).round().z(), 0.0);
    assert_eq!(Vec3::new(0.0, 21.1, 0.0).round().y(), 21.0);
    assert_eq!(Vec3::new(0.0, 11.123, 0.0).round().y(), 11.0);
    assert_eq!(Vec3::new(0.0, 11.499, 0.0).round().y(), 11.0);
    assert_eq!(
        Vec3::new(f32::NEG_INFINITY, f32::INFINITY, 0.0).round(),
        Vec3::new(f32::NEG_INFINITY, f32::INFINITY, 0.0)
    );
    assert!(Vec3::new(f32::NAN, 0.0, 0.0).round().x().is_nan());
}

#[test]
fn test_vec3_floor() {
    assert_eq!(
        Vec3::new(1.35, 1.5, -1.5).floor(),
        Vec3::new(1.0, 1.0, -2.0)
    );
    assert_eq!(
        Vec3::new(f32::INFINITY, f32::NEG_INFINITY, 0.0).floor(),
        Vec3::new(f32::INFINITY, f32::NEG_INFINITY, 0.0)
    );
    assert!(Vec3::new(f32::NAN, 0.0, 0.0).floor().x().is_nan());
    assert_eq!(
        Vec3::new(-2000000.123, 10000000.123, 1000.9).floor(),
        Vec3::new(-2000001.0, 10000000.0, 1000.0)
    );
}

#[test]
fn test_vec3_ceil() {
    assert_eq!(Vec3::new(1.35, 1.5, -1.5).ceil(), Vec3::new(2.0, 2.0, -1.0));
    assert_eq!(
        Vec3::new(f32::INFINITY, f32::NEG_INFINITY, 0.0).ceil(),
        Vec3::new(f32::INFINITY, f32::NEG_INFINITY, 0.0)
    );
    assert!(Vec3::new(f32::NAN, 0.0, 0.0).ceil().x().is_nan());
    assert_eq!(
        Vec3::new(-2000000.123, 1000000.123, 1000.9).ceil(),
        Vec3::new(-2000000.0, 1000001.0, 1001.0)
    );
}

#[test]
fn test_vec3_lerp() {
    let v0 = Vec3::new(-1.0, -1.0, -1.0);
    let v1 = Vec3::new(1.0, 1.0, 1.0);
    assert_approx_eq!(v0, v0.lerp(v1, 0.0));
    assert_approx_eq!(v1, v0.lerp(v1, 1.0));
    assert_approx_eq!(Vec3::zero(), v0.lerp(v1, 0.5));
}

#[test]
fn test_vec3_to_from_slice() {
    let v = Vec3::new(1.0, 2.0, 3.0);
    let mut a = [0.0, 0.0, 0.0];
    v.write_to_slice_unaligned(&mut a);
    assert_eq!(v, Vec3::from_slice_unaligned(&a));
}

#[test]
fn test_vec3_angle_between() {
    let angle = Vec3::new(1.0, 0.0, 1.0).angle_between(Vec3::new(1.0, 1.0, 0.0));
    assert_approx_eq!(f32::consts::FRAC_PI_3, angle, 1e-6);

    let angle = Vec3::new(10.0, 0.0, 10.0).angle_between(Vec3::new(5.0, 5.0, 0.0));
    assert_approx_eq!(f32::consts::FRAC_PI_3, angle, 1e-6);

    let angle = Vec3::new(-1.0, 0.0, -1.0).angle_between(Vec3::new(1.0, -1.0, 0.0));
    assert_approx_eq!(2.0 * f32::consts::FRAC_PI_3, angle, 1e-6);
}

#[cfg(feature = "serde")]
#[test]
fn test_vec3_serde() {
    let a = Vec3::new(1.0, 2.0, 3.0);
    let serialized = serde_json::to_string(&a).unwrap();
    assert_eq!(serialized, "[1.0,2.0,3.0]");
    let deserialized = serde_json::from_str(&serialized).unwrap();
    assert_eq!(a, deserialized);
    let deserialized = serde_json::from_str::<Vec3>("[]");
    assert!(deserialized.is_err());
    let deserialized = serde_json::from_str::<Vec3>("[1.0]");
    assert!(deserialized.is_err());
    let deserialized = serde_json::from_str::<Vec3>("[1.0,2.0]");
    assert!(deserialized.is_err());
    let deserialized = serde_json::from_str::<Vec3>("[1.0,2.0,3.0,4.0]");
    assert!(deserialized.is_err());
}

#[cfg(feature = "rand")]
#[test]
fn test_vec3_rand() {
    use rand::{Rng, SeedableRng};
    use rand_xoshiro::Xoshiro256Plus;
    let mut rng1 = Xoshiro256Plus::seed_from_u64(0);
    let a: (f32, f32, f32) = rng1.gen();
    let mut rng2 = Xoshiro256Plus::seed_from_u64(0);
    let b: Vec3 = rng2.gen();
    assert_eq!(a, b.into());
}
