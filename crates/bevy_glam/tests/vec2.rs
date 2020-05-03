mod support;

use glam::*;
use std::f32;

#[test]
fn test_vec2_align() {
    use core::mem;
    assert_eq!(8, mem::size_of::<Vec2>());
    assert_eq!(4, mem::align_of::<Vec2>());
    assert_eq!(8, mem::size_of::<Vec2Mask>());
    assert_eq!(4, mem::align_of::<Vec2Mask>());
}

#[test]
fn test_vec2_new() {
    let v = vec2(1.0, 2.0);

    assert_eq!(v.x(), 1.0);
    assert_eq!(v.y(), 2.0);

    let t = (1.0, 2.0);
    let v = Vec2::from(t);
    assert_eq!(t, v.into());

    let a = [1.0, 2.0];
    let v = Vec2::from(a);
    let a1: [f32; 2] = v.into();
    assert_eq!(a, a1);

    let v = Vec2::new(t.0, t.1);
    assert_eq!(t, v.into());

    assert_eq!(Vec2::new(1.0, 0.0), Vec2::unit_x());
    assert_eq!(Vec2::new(0.0, 1.0), Vec2::unit_y());
}

#[test]
fn test_vec2_fmt() {
    let a = Vec2::new(1.0, 2.0);
    assert_eq!(format!("{:?}", a), "Vec2(1.0, 2.0)");
    // assert_eq!(format!("{:#?}", a), "Vec2(\n    1.0,\n    2.0\n)");
    assert_eq!(format!("{}", a), "[1, 2]");
}

#[test]
fn test_vec2_zero() {
    let v = Vec2::zero();
    assert_eq!(vec2(0.0, 0.0), v);
    assert_eq!(v, Vec2::default());
}

#[test]
fn test_vec2_splat() {
    let v = Vec2::splat(1.0);
    assert_eq!(vec2(1.0, 1.0), v);
}

#[test]
fn test_vec2_accessors() {
    let mut a = Vec2::zero();
    a.set_x(1.0);
    a.set_y(2.0);
    assert_eq!(1.0, a.x());
    assert_eq!(2.0, a.y());
    assert_eq!(Vec2::new(1.0, 2.0), a);

    let mut a = Vec2::zero();
    *a.x_mut() = 1.0;
    *a.y_mut() = 2.0;
    assert_eq!(1.0, a.x());
    assert_eq!(2.0, a.y());
    assert_eq!(Vec2::new(1.0, 2.0), a);

    let mut a = Vec2::zero();
    a[0] = 1.0;
    a[1] = 2.0;
    assert_eq!(1.0, a[0]);
    assert_eq!(2.0, a[1]);
    assert_eq!(Vec2::new(1.0, 2.0), a);
}

#[test]
fn test_vec2_funcs() {
    let x = vec2(1.0, 0.0);
    let y = vec2(0.0, 1.0);
    assert_eq!(1.0, x.dot(x));
    assert_eq!(0.0, x.dot(y));
    assert_eq!(-1.0, x.dot(-x));
    assert_eq!(4.0, (2.0 * x).length_squared());
    assert_eq!(9.0, (-3.0 * y).length_squared());
    assert_eq!(2.0, (-2.0 * x).length());
    assert_eq!(3.0, (3.0 * y).length());
    assert_eq!(x, (2.0 * x).normalize());
    assert_eq!(1.0 * 3.0 + 2.0 * 4.0, vec2(1.0, 2.0).dot(vec2(3.0, 4.0)));
    assert_eq!(2.0 * 2.0 + 3.0 * 3.0, vec2(2.0, 3.0).length_squared());
    assert_eq!((2.0_f32 * 2.0 + 3.0 * 3.0).sqrt(), vec2(2.0, 3.0).length());
    assert_eq!(
        1.0 / (2.0_f32 * 2.0 + 3.0 * 3.0).sqrt(),
        vec2(2.0, 3.0).length_reciprocal()
    );
    assert!(vec2(2.0, 3.0).normalize().is_normalized());
    assert_eq!(
        vec2(2.0, 3.0) / (2.0_f32 * 2.0 + 3.0 * 3.0).sqrt(),
        vec2(2.0, 3.0).normalize()
    );
    assert_eq!(vec2(0.5, 0.25), vec2(2.0, 4.0).reciprocal());
}

#[test]
fn test_vec2_ops() {
    let a = vec2(1.0, 2.0);
    assert_eq!(vec2(2.0, 4.0), (a + a));
    assert_eq!(vec2(0.0, 0.0), (a - a));
    assert_eq!(vec2(1.0, 4.0), (a * a));
    assert_eq!(vec2(2.0, 4.0), (a * 2.0));
    assert_eq!(vec2(1.0, 1.0), (a / a));
    assert_eq!(vec2(0.5, 1.0), (a / 2.0));
    assert_eq!(vec2(-1.0, -2.0), (-a));
}

#[test]
fn test_vec2_assign_ops() {
    let a = vec2(1.0, 2.0);
    let mut b = a;
    b += a;
    assert_eq!(vec2(2.0, 4.0), b);
    b -= a;
    assert_eq!(vec2(1.0, 2.0), b);
    b *= a;
    assert_eq!(vec2(1.0, 4.0), b);
    b /= a;
    assert_eq!(vec2(1.0, 2.0), b);
    b *= 2.0;
    assert_eq!(vec2(2.0, 4.0), b);
    b /= 2.0;
    assert_eq!(vec2(1.0, 2.0), b);
}

#[test]
fn test_vec2_min_max() {
    let a = vec2(-1.0, 2.0);
    let b = vec2(1.0, -2.0);
    assert_eq!(vec2(-1.0, -2.0), a.min(b));
    assert_eq!(vec2(-1.0, -2.0), b.min(a));
    assert_eq!(vec2(1.0, 2.0), a.max(b));
    assert_eq!(vec2(1.0, 2.0), b.max(a));
}

#[test]
fn test_vec2_hmin_hmax() {
    let a = vec2(-1.0, 2.0);
    assert_eq!(-1.0, a.min_element());
    assert_eq!(2.0, a.max_element());
}

#[test]
fn test_vec2_eq() {
    let a = vec2(1.0, 1.0);
    let b = vec2(1.0, 2.0);
    assert!(a.cmpeq(a).all());
    assert!(b.cmpeq(b).all());
    assert!(a.cmpne(b).any());
    assert!(b.cmpne(a).any());
    assert!(b.cmpeq(a).any());
}

#[test]
fn test_vec2_cmp() {
    assert!(!Vec2Mask::default().any());
    assert!(!Vec2Mask::default().all());
    assert_eq!(Vec2Mask::default().bitmask(), 0x0);
    let a = vec2(-1.0, -1.0);
    let b = vec2(1.0, 1.0);
    let c = vec2(-1.0, -1.0);
    let d = vec2(1.0, -1.0);
    assert_eq!(a.cmplt(a).bitmask(), 0x0);
    assert_eq!(a.cmplt(b).bitmask(), 0x3);
    assert_eq!(a.cmplt(d).bitmask(), 0x1);
    assert_eq!(c.cmple(a).bitmask(), 0x3);
    assert!(a.cmplt(b).all());
    assert!(a.cmplt(d).any());
    assert!(a.cmple(b).all());
    assert!(a.cmple(a).all());
    assert!(b.cmpgt(a).all());
    assert!(b.cmpge(a).all());
    assert!(b.cmpge(b).all());
    assert!(!(a.cmpge(d).all()));
    assert!(c.cmple(c).all());
    assert!(c.cmpge(c).all());
    assert!(a == a);
    assert!(a < b);
    assert!(b > a);
}

#[test]
fn test_extend_truncate() {
    let a = vec2(1.0, 2.0);
    let b = a.extend(3.0);
    assert_eq!(vec3(1.0, 2.0, 3.0), b);
}

#[test]
fn test_vec2b() {
    // make sure the unused 'w' value doesn't break Vec2b behaviour
    let a = Vec3::zero();
    let mut b = a.truncate();
    b.set_x(1.0);
    b.set_y(1.0);
    assert!(!b.cmpeq(Vec2::zero()).any());
    assert!(b.cmpeq(Vec2::splat(1.0)).all());
}

#[test]
fn test_vec2mask_as_ref() {
    assert_eq!(Vec2Mask::new(false, false).as_ref(), &[0, 0]);
    assert_eq!(Vec2Mask::new(true, false).as_ref(), &[!0, 0]);
    assert_eq!(Vec2Mask::new(false, true).as_ref(), &[0, !0]);
    assert_eq!(Vec2Mask::new(true, true).as_ref(), &[!0, !0]);
}

#[test]
fn test_vec2mask_from() {
    assert_eq!(Into::<[u32; 2]>::into(Vec2Mask::new(false, false)), [0, 0]);
    assert_eq!(Into::<[u32; 2]>::into(Vec2Mask::new(true, false)), [!0, 0]);
    assert_eq!(Into::<[u32; 2]>::into(Vec2Mask::new(false, true)), [0, !0]);
    assert_eq!(Into::<[u32; 2]>::into(Vec2Mask::new(true, true)), [!0, !0]);
}

#[test]
fn test_vec2mask_bitmask() {
    assert_eq!(Vec2Mask::new(false, false).bitmask(), 0b00);
    assert_eq!(Vec2Mask::new(true, false).bitmask(), 0b01);
    assert_eq!(Vec2Mask::new(false, true).bitmask(), 0b10);
    assert_eq!(Vec2Mask::new(true, true).bitmask(), 0b11);
}

#[test]
fn test_vec2mask_any() {
    assert_eq!(Vec2Mask::new(false, false).any(), false);
    assert_eq!(Vec2Mask::new(true, false).any(), true);
    assert_eq!(Vec2Mask::new(false, true).any(), true);
    assert_eq!(Vec2Mask::new(true, true).any(), true);
}

#[test]
fn test_vec2mask_all() {
    assert_eq!(Vec2Mask::new(false, false).all(), false);
    assert_eq!(Vec2Mask::new(true, false).all(), false);
    assert_eq!(Vec2Mask::new(false, true).all(), false);
    assert_eq!(Vec2Mask::new(true, true).all(), true);
}

#[test]
fn test_vec2mask_select() {
    let a = Vec2::new(1.0, 2.0);
    let b = Vec2::new(3.0, 4.0);
    assert_eq!(Vec2Mask::new(true, true).select(a, b), Vec2::new(1.0, 2.0),);
    assert_eq!(Vec2Mask::new(true, false).select(a, b), Vec2::new(1.0, 4.0),);
    assert_eq!(Vec2Mask::new(false, true).select(a, b), Vec2::new(3.0, 2.0),);
    assert_eq!(
        Vec2Mask::new(false, false).select(a, b),
        Vec2::new(3.0, 4.0),
    );
}

#[test]
fn test_vec2mask_and() {
    assert_eq!(
        (Vec2Mask::new(false, false) & Vec2Mask::new(false, false)).bitmask(),
        0b00,
    );
    assert_eq!(
        (Vec2Mask::new(true, true) & Vec2Mask::new(true, false)).bitmask(),
        0b01,
    );
    assert_eq!(
        (Vec2Mask::new(true, false) & Vec2Mask::new(false, true)).bitmask(),
        0b00,
    );
    assert_eq!(
        (Vec2Mask::new(true, true) & Vec2Mask::new(true, true)).bitmask(),
        0b11,
    );

    let mut mask = Vec2Mask::new(true, true);
    mask &= Vec2Mask::new(true, false);
    assert_eq!(mask.bitmask(), 0b01);
}

#[test]
fn test_vec2mask_or() {
    assert_eq!(
        (Vec2Mask::new(false, false) | Vec2Mask::new(false, false)).bitmask(),
        0b00,
    );
    assert_eq!(
        (Vec2Mask::new(false, false) | Vec2Mask::new(false, true)).bitmask(),
        0b10,
    );
    assert_eq!(
        (Vec2Mask::new(true, false) | Vec2Mask::new(false, true)).bitmask(),
        0b11,
    );
    assert_eq!(
        (Vec2Mask::new(true, true) | Vec2Mask::new(true, true)).bitmask(),
        0b11,
    );

    let mut mask = Vec2Mask::new(true, true);
    mask |= Vec2Mask::new(true, false);
    assert_eq!(mask.bitmask(), 0b11);
}

#[test]
fn test_vec2mask_not() {
    assert_eq!((!Vec2Mask::new(false, false)).bitmask(), 0b11);
    assert_eq!((!Vec2Mask::new(true, false)).bitmask(), 0b10);
    assert_eq!((!Vec2Mask::new(false, true)).bitmask(), 0b01);
    assert_eq!((!Vec2Mask::new(true, true)).bitmask(), 0b00);
}

#[test]
fn test_vec2mask_fmt() {
    let a = Vec2Mask::new(true, false);

    assert_eq!(format!("{:?}", a), "Vec2Mask(0xffffffff, 0x0)");
    assert_eq!(format!("{}", a), "[true, false]");
}

#[test]
fn test_vec2mask_eq() {
    let a = Vec2Mask::new(true, false);
    let b = Vec2Mask::new(true, false);
    let c = Vec2Mask::new(false, true);

    assert_eq!(a, b);
    assert_eq!(b, a);
    assert_ne!(a, c);
    assert_ne!(b, c);

    assert!(a > c);
    assert!(c < a);
}

#[test]
fn test_vec2mask_hash() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hash;
    use std::hash::Hasher;

    let a = Vec2Mask::new(true, false);
    let b = Vec2Mask::new(true, false);
    let c = Vec2Mask::new(false, true);

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
fn test_vec2_sign() {
    assert_eq!(Vec2::zero().sign(), Vec2::one());
    assert_eq!(-Vec2::zero().sign(), -Vec2::one());
    assert_eq!(Vec2::one().sign(), Vec2::one());
    assert_eq!((-Vec2::one()).sign(), -Vec2::one());
    assert_eq!(Vec2::splat(core::f32::NEG_INFINITY).sign(), -Vec2::one());
}

#[test]
fn test_vec2_abs() {
    assert_eq!(Vec2::zero().abs(), Vec2::zero());
    assert_eq!(Vec2::one().abs(), Vec2::one());
    assert_eq!((-Vec2::one()).abs(), Vec2::one());
}

#[test]
fn test_vec2_round() {
    assert_eq!(Vec2::new(1.35, 0.0).round().x(), 1.0);
    assert_eq!(Vec2::new(0.0, 1.5).round().y(), 2.0);
    assert_eq!(Vec2::new(0.0, -15.5).round().y(), -16.0);
    assert_eq!(Vec2::new(0.0, 0.0).round().y(), 0.0);
    assert_eq!(Vec2::new(0.0, 21.1).round().y(), 21.0);
    assert_eq!(Vec2::new(0.0, 11.123).round().y(), 11.0);
    assert_eq!(Vec2::new(0.0, 11.499).round().y(), 11.0);
    assert_eq!(
        Vec2::new(f32::NEG_INFINITY, f32::INFINITY).round(),
        Vec2::new(f32::NEG_INFINITY, f32::INFINITY)
    );
    assert!(Vec2::new(f32::NAN, 0.0).round().x().is_nan());
}

#[test]
fn test_vec2_floor() {
    assert_eq!(Vec2::new(1.35, -1.5).floor(), Vec2::new(1.0, -2.0));
    assert_eq!(
        Vec2::new(f32::INFINITY, f32::NEG_INFINITY).floor(),
        Vec2::new(f32::INFINITY, f32::NEG_INFINITY)
    );
    assert!(Vec2::new(f32::NAN, 0.0).floor().x().is_nan());
    assert_eq!(
        Vec2::new(-2000000.123, 10000000.123).floor(),
        Vec2::new(-2000001.0, 10000000.0)
    );
}

#[test]
fn test_vec2_ceil() {
    assert_eq!(Vec2::new(1.35, -1.5).ceil(), Vec2::new(2.0, -1.0));
    assert_eq!(
        Vec2::new(f32::INFINITY, f32::NEG_INFINITY).ceil(),
        Vec2::new(f32::INFINITY, f32::NEG_INFINITY)
    );
    assert!(Vec2::new(f32::NAN, 0.0).ceil().x().is_nan());
    assert_eq!(
        Vec2::new(-2000000.123, 1000000.123).ceil(),
        Vec2::new(-2000000.0, 1000001.0)
    );
}

#[test]
fn test_vec2_lerp() {
    let v0 = Vec2::new(-1.0, -1.0);
    let v1 = Vec2::new(1.0, 1.0);
    assert_approx_eq!(v0, v0.lerp(v1, 0.0));
    assert_approx_eq!(v1, v0.lerp(v1, 1.0));
    assert_approx_eq!(Vec2::zero(), v0.lerp(v1, 0.5));
}

#[test]
fn test_vec2_to_from_slice() {
    let v = Vec2::new(1.0, 2.0);
    let mut a = [0.0, 0.0];
    v.write_to_slice_unaligned(&mut a);
    assert_eq!(v, Vec2::from_slice_unaligned(&a));
}

#[test]
fn test_vec2_angle_between() {
    let angle = Vec2::new(1.0, 0.0).angle_between(Vec2::new(0.0, 1.0));
    assert_approx_eq!(f32::consts::FRAC_PI_2, angle, 1e-6);

    let angle = Vec2::new(10.0, 0.0).angle_between(Vec2::new(0.0, 5.0));
    assert_approx_eq!(f32::consts::FRAC_PI_2, angle, 1e-6);

    let angle = Vec2::new(-1.0, 0.0).angle_between(Vec2::new(0.0, 1.0));
    assert_approx_eq!(-f32::consts::FRAC_PI_2, angle, 1e-6);
}

#[cfg(feature = "serde")]
#[test]
fn test_vec2_serde() {
    let a = Vec2::new(1.0, 2.0);
    let serialized = serde_json::to_string(&a).unwrap();
    assert_eq!(serialized, "[1.0,2.0]");
    let deserialized = serde_json::from_str(&serialized).unwrap();
    assert_eq!(a, deserialized);
    let deserialized = serde_json::from_str::<Vec2>("[]");
    assert!(deserialized.is_err());
    let deserialized = serde_json::from_str::<Vec2>("[1.0]");
    assert!(deserialized.is_err());
    let deserialized = serde_json::from_str::<Vec2>("[1.0,2.0,3.0]");
    assert!(deserialized.is_err());
}

#[cfg(feature = "rand")]
#[test]
fn test_vec2_rand() {
    use rand::{Rng, SeedableRng};
    use rand_xoshiro::Xoshiro256Plus;
    let mut rng1 = Xoshiro256Plus::seed_from_u64(0);
    let a: (f32, f32) = rng1.gen();
    let mut rng2 = Xoshiro256Plus::seed_from_u64(0);
    let b: Vec2 = rng2.gen();
    assert_eq!(a, b.into());
}
