<<<<<<< HEAD
pub trait AttributeInterpolator<T: Copy> {
    fn interpolate(&mut self, a: T, b: T, p: f32) -> T;

=======
/// Performs an interpolation of two attributes of a vertex.
pub trait AttributeInterpolator<T: Copy> {
    /// Interpolates two values given a percent.
    fn interpolate(&mut self, a: T, b: T, p: f32) -> T;

    /// Interpolates two values given 50%.
    ///
    /// This can sometimes be more optimized than calling
    /// `interpolate(a, b, 0.5)`.
>>>>>>> temp
    fn interpolate_half(&mut self, a: T, b: T) -> T {
        self.interpolate(a, b, 0.5)
    }

<<<<<<< HEAD
    fn interpolate_multiple(&mut self, a: T, b: T, indices: &[u32], points: &mut [T]) {
        for (percent, index) in indices.iter().enumerate() {
=======
    /// Interpolates points given their positions along the line of indices between `a` and `b`.
    /// ```text
    /// [a  , points[indices[0]], points[indices[1]], ..., points[indices[Q]], b  ]
    ///
    /// [N/A, 1 / (Q + 1)       , 2 / (Q + 1)       , ..., Q / (Q + 2)       , N/A]
    /// ```
    fn interpolate_multiple(&mut self, a: T, b: T, indices: &[u32], points: &mut [T]) {
        for (percent, index) in indices.iter().enumerate() {
            // `indices.len() + 1` instead of `+ 2` as the diagram suggests because
            // there are
>>>>>>> temp
            let percent = (percent + 1) as f32 / (indices.len() + 1) as f32;

            points[*index as usize] = self.interpolate(a, b, percent);
        }
    }
}

<<<<<<< HEAD
///
/// Always returns LHS.
///
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct IdentityInterpolator;

///
=======
/// Always returns LHS.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct IdentityInterpolator;

>>>>>>> temp
/// Linear interpolation:
///
/// If `t` is in `[0, 1]`, then interpolating between `a` and `b`
/// will yield:
///
/// ```ignore
/// a + t * (b - a)
/// ```
<<<<<<< HEAD
///
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct LinearInterpolator;

///
=======
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct LinearInterpolator;

>>>>>>> temp
/// Only available for `f32` attributes with more than one
/// component.
///
/// If `t` is in `[0, 1]`, then interpolating between `a` and `b`
/// will yield:
/// ```ignore
/// |a + t * (b - a)|
/// ```
/// Where `|v|` is defined as the normalization of `v`.
<<<<<<< HEAD
///
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct NormalizedLinearInterpolator;

///
=======
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct NormalizedLinearInterpolator;

>>>>>>> temp
/// Only available for `f32` attributes with more than one
/// component.
///
/// Performs geometric spherical interpolation.
<<<<<<< HEAD
///
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct SphericalInterpolator;

=======
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct SphericalInterpolator;

impl<'a, T: Copy, I: AttributeInterpolator<T>> AttributeInterpolator<T> for &'a mut I {
    #[inline(always)]
    fn interpolate(&mut self, a: T, b: T, t: f32) -> T {
        I::interpolate(*self, a, b, t)
    }

    #[inline(always)]
    fn interpolate_half(&mut self, a: T, b: T) -> T {
        I::interpolate_half(*self, a, b)
    }

    #[inline]
    fn interpolate_multiple(&mut self, a: T, b: T, indices: &[u32], points: &mut [T]) {
        I::interpolate_multiple(*self, a, b, indices, points)
    }
}

// Implementations for all of the (sensible) attribute interpolators.
>>>>>>> temp
mod impls {
    use super::{
        AttributeInterpolator, IdentityInterpolator, LinearInterpolator,
        NormalizedLinearInterpolator, SphericalInterpolator,
    };
    use bevy_math::{Vec2, Vec3, Vec4};
<<<<<<< HEAD
=======

>>>>>>> temp
    impl<T: Copy> AttributeInterpolator<T> for IdentityInterpolator {
        #[inline(always)]
        fn interpolate(&mut self, a: T, _: T, _: f32) -> T {
            a
        }

        #[inline(always)]
        fn interpolate_half(&mut self, a: T, _: T) -> T {
            a
        }

        #[inline]
        fn interpolate_multiple(&mut self, a: T, _: T, indices: &[u32], points: &mut [T]) {
            for index in indices.iter() {
                points[*index as usize] = a;
            }
        }
    }

<<<<<<< HEAD
    impl AttributeInterpolator<i32> for LinearInterpolator {
        fn interpolate(&mut self, a: i32, b: i32, p: f32) -> i32 {
            a + (p * (b - a) as f32) as i32
        }

        fn interpolate_half(&mut self, a: i32, b: i32) -> i32 {
            (a + b) / 2
        }
    }

    impl AttributeInterpolator<[i32; 2]> for LinearInterpolator {
        fn interpolate(&mut self, [aa, ab]: [i32; 2], [ba, bb]: [i32; 2], p: f32) -> [i32; 2] {
            [
                aa + (p * (ba - aa) as f32) as i32,
                ab + (p * (bb - ab) as f32) as i32,
            ]
        }

        fn interpolate_half(&mut self, [aa, ab]: [i32; 2], [ba, bb]: [i32; 2]) -> [i32; 2] {
            [(aa + ba) / 2, (ab + bb) / 2]
        }
    }

    impl AttributeInterpolator<[i32; 3]> for LinearInterpolator {
        fn interpolate(
            &mut self,
            [aa, ab, ac]: [i32; 3],
            [ba, bb, bc]: [i32; 3],
            p: f32,
        ) -> [i32; 3] {
            [
                aa + (p * (ba - aa) as f32) as i32,
                ab + (p * (bb - ab) as f32) as i32,
                ac + (p * (bc - ac) as f32) as i32,
            ]
        }

        fn interpolate_half(&mut self, [aa, ab, ac]: [i32; 3], [ba, bb, bc]: [i32; 3]) -> [i32; 3] {
            [(aa + ba) / 2, (ab + bb) / 2, (ac + bc) / 2]
        }
    }

    impl AttributeInterpolator<[i32; 4]> for LinearInterpolator {
        fn interpolate(
            &mut self,
            [aa, ab, ac, ad]: [i32; 4],
            [ba, bb, bc, bd]: [i32; 4],
            p: f32,
        ) -> [i32; 4] {
            [
                aa + (p * (ba - aa) as f32) as i32,
                ab + (p * (bb - ab) as f32) as i32,
                ac + (p * (bc - ac) as f32) as i32,
                ad + (p * (bd - ad) as f32) as i32,
            ]
        }

        fn interpolate_half(
            &mut self,
            [aa, ab, ac, ad]: [i32; 4],
            [ba, bb, bc, bd]: [i32; 4],
        ) -> [i32; 4] {
            [(aa + ba) / 2, (ab + bb) / 2, (ac + bc) / 2, (ad + bd) / 2]
        }
    }

    impl AttributeInterpolator<u32> for LinearInterpolator {
        fn interpolate(&mut self, a: u32, b: u32, p: f32) -> u32 {
            a + (p * (b - a) as f32) as u32
        }

        fn interpolate_half(&mut self, a: u32, b: u32) -> u32 {
            (a + b) / 2
        }
    }

    impl AttributeInterpolator<[u32; 2]> for LinearInterpolator {
        fn interpolate(&mut self, [aa, ab]: [u32; 2], [ba, bb]: [u32; 2], p: f32) -> [u32; 2] {
            [
                aa + (p * (ba - aa) as f32) as u32,
                ab + (p * (bb - ab) as f32) as u32,
            ]
        }

        fn interpolate_half(&mut self, [aa, ab]: [u32; 2], [ba, bb]: [u32; 2]) -> [u32; 2] {
            [(aa + ba) / 2, (ab + bb) / 2]
        }
    }

    impl AttributeInterpolator<[u32; 3]> for LinearInterpolator {
        fn interpolate(
            &mut self,
            [aa, ab, ac]: [u32; 3],
            [ba, bb, bc]: [u32; 3],
            p: f32,
        ) -> [u32; 3] {
            [
                aa + (p * (ba - aa) as f32) as u32,
                ab + (p * (bb - ab) as f32) as u32,
                ac + (p * (bc - ac) as f32) as u32,
            ]
        }

        fn interpolate_half(&mut self, [aa, ab, ac]: [u32; 3], [ba, bb, bc]: [u32; 3]) -> [u32; 3] {
            [(aa + ba) / 2, (ab + bb) / 2, (ac + bc) / 2]
        }
    }

    impl AttributeInterpolator<[u32; 4]> for LinearInterpolator {
        fn interpolate(
            &mut self,
            [aa, ab, ac, ad]: [u32; 4],
            [ba, bb, bc, bd]: [u32; 4],
            p: f32,
        ) -> [u32; 4] {
            [
                aa + (p * (ba - aa) as f32) as u32,
                ab + (p * (bb - ab) as f32) as u32,
                ac + (p * (bc - ac) as f32) as u32,
                ad + (p * (bd - ad) as f32) as u32,
            ]
        }

        fn interpolate_half(
            &mut self,
            [aa, ab, ac, ad]: [u32; 4],
            [ba, bb, bc, bd]: [u32; 4],
        ) -> [u32; 4] {
            [(aa + ba) / 2, (ab + bb) / 2, (ac + bc) / 2, (ad + bd) / 2]
        }
    }

    impl AttributeInterpolator<f32> for LinearInterpolator {
        fn interpolate(&mut self, a: f32, b: f32, p: f32) -> f32 {
            a + p * (b - a)
        }

        fn interpolate_half(&mut self, a: f32, b: f32) -> f32 {
            (a + b) * 0.5
        }
    }

    impl AttributeInterpolator<[f32; 2]> for LinearInterpolator {
        fn interpolate(&mut self, a: [f32; 2], b: [f32; 2], p: f32) -> [f32; 2] {
            let a = Vec2::from(a);
            let b = Vec2::from(b);
            let r = a + p * (b - a);
            r.into()
        }

        fn interpolate_half(&mut self, a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
            let a = Vec2::from(a);
            let b = Vec2::from(b);
            let r = (a + b) * 0.5;
            r.into()
        }
    }

    impl AttributeInterpolator<[f32; 3]> for LinearInterpolator {
        fn interpolate(&mut self, a: [f32; 3], b: [f32; 3], p: f32) -> [f32; 3] {
            let a = Vec3::from(a);
            let b = Vec3::from(b);
            let r = a + p * (b - a);
            r.into()
        }

        fn interpolate_half(&mut self, a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
            let a = Vec3::from(a);
            let b = Vec3::from(b);
            let r = (a + b) * 0.5;
            r.into()
        }
    }

    impl AttributeInterpolator<[f32; 4]> for LinearInterpolator {
        fn interpolate(&mut self, a: [f32; 4], b: [f32; 4], p: f32) -> [f32; 4] {
            let a = Vec4::from(a);
            let b = Vec4::from(b);
            let r = a + p * (b - a);
            r.into()
        }

        fn interpolate_half(&mut self, a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
            let a = Vec4::from(a);
            let b = Vec4::from(b);
            let r = (a + b) * 0.5;
            r.into()
        }
    }

    impl AttributeInterpolator<[u8; 4]> for LinearInterpolator {
        fn interpolate(
            &mut self,
            [aa, ab, ac, ad]: [u8; 4],
            [ba, bb, bc, bd]: [u8; 4],
            p: f32,
        ) -> [u8; 4] {
            [
                aa + (p * (ba - aa) as f32) as u8,
                ab + (p * (bb - ab) as f32) as u8,
                ac + (p * (bc - ac) as f32) as u8,
                ad + (p * (bd - ad) as f32) as u8,
            ]
        }

        fn interpolate_half(
            &mut self,
            [aa, ab, ac, ad]: [u8; 4],
            [ba, bb, bc, bd]: [u8; 4],
        ) -> [u8; 4] {
            [(aa + ba) / 2, (ab + bb) / 2, (ac + bc) / 2, (ad + bd) / 2]
        }
=======
    macro_rules! linear_interpolator {
        (
            $($ty2:ty),*
            $([$dims:literal # $($tys:ty),*]),*
        ) => {
            $(
                impl AttributeInterpolator<$ty2> for LinearInterpolator {
                    fn interpolate(&mut self, a: $ty2, b: $ty2, p: f32) -> $ty2 {
                        a + (p * (b - a) as f32) as $ty2
                    }

                    fn interpolate_half(&mut self, a: $ty2, b: $ty2) -> $ty2 {
                        a + (b - a) / (2 as $ty2)
                    }
                }
            )*
            $(
                $(
                    impl AttributeInterpolator<[$tys; $dims]> for LinearInterpolator {
                        fn interpolate(&mut self, mut a: [$tys; $dims], b: [$tys; $dims], p: f32) -> [$tys; $dims] {
                            use std::array::IntoIter;

                            a.iter_mut()
                                .zip(
                                    IntoIter::new(b)
                                )
                                .for_each(|(a, b)| *a += (p * (b - *a) as f32) as $tys);

                            a
                        }

                        fn interpolate_half(&mut self, mut a: [$tys; $dims], b: [$tys; $dims]) -> [$tys; $dims] {
                            use std::array::IntoIter;

                            a.iter_mut()
                                .zip(
                                    IntoIter::new(b)
                                )
                                .for_each(|(a, b)| *a += (b - *a) / (2 as $tys));

                            a
                        }
                    }
                )*
            )*
        }
    }

    linear_interpolator! {
        u32, i32, u16, i16, u8, i8, f32
        [2 # u32, i32, u16, i16, u8, i8, f32],
        [3 # u32, i32, u16, i16, u8, i8, f32],
        [4 # u32, i32, u16, i16, u8, i8, f32]
>>>>>>> temp
    }

    impl AttributeInterpolator<[f32; 2]> for NormalizedLinearInterpolator {
        fn interpolate(&mut self, a: [f32; 2], b: [f32; 2], p: f32) -> [f32; 2] {
            let a = Vec2::from(a);
            let b = Vec2::from(b);
            let r = a + p * (b - a);
            r.normalize().into()
        }

        fn interpolate_half(&mut self, a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
            let a = Vec2::from(a);
            let b = Vec2::from(b);
            let r = (a + b) * 0.5;
            r.normalize().into()
        }
    }

    impl AttributeInterpolator<[f32; 3]> for NormalizedLinearInterpolator {
        fn interpolate(&mut self, a: [f32; 3], b: [f32; 3], p: f32) -> [f32; 3] {
            let a = Vec3::from(a);
            let b = Vec3::from(b);
            let r = a + p * (b - a);
            r.normalize().into()
        }

        fn interpolate_half(&mut self, a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
            let a = Vec3::from(a);
            let b = Vec3::from(b);
            let r = (a + b) * 0.5;
            r.normalize().into()
        }
    }

    impl AttributeInterpolator<[f32; 4]> for NormalizedLinearInterpolator {
        fn interpolate(&mut self, a: [f32; 4], b: [f32; 4], p: f32) -> [f32; 4] {
            let a = Vec4::from(a);
            let b = Vec4::from(b);
            let r = a + p * (b - a);
            r.normalize().into()
        }

        fn interpolate_half(&mut self, a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
            let a = Vec4::from(a);
            let b = Vec4::from(b);
            let r = (a + b) * 0.5;
            r.normalize().into()
        }
    }

    impl AttributeInterpolator<[f32; 2]> for SphericalInterpolator {
        fn interpolate(&mut self, a: [f32; 2], b: [f32; 2], p: f32) -> [f32; 2] {
            let a = Vec2::from(a);
            let b = Vec2::from(b);
            let angle = a.dot(b).acos();

            let sin = angle.sin().recip();
            let r = a * (((1.0 - p) * angle).sin() * sin) + b * ((p * angle).sin() * sin);
            r.into()
        }

        fn interpolate_half(&mut self, a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
            let a = Vec2::from(a);
            let b = Vec2::from(b);
            let r = (a + b) * (2.0 * (1.0 + a.dot(b))).sqrt().recip();
            r.into()
        }

        fn interpolate_multiple(
            &mut self,
            a: [f32; 2],
            b: [f32; 2],
            indices: &[u32],
            points: &mut [[f32; 2]],
        ) {
            let a = Vec2::from(a);
            let b = Vec2::from(b);

            let angle = a.dot(b).acos();
            let sin = angle.sin().recip();

            for (percent, index) in indices.iter().enumerate() {
                let percent = (percent + 1) as f32 / (indices.len() + 1) as f32;

                let r = a * (((1.0 - percent) * angle).sin() * sin)
                    + b * ((percent * angle).sin() * sin);
                points[*index as usize] = r.into();
            }
        }
    }

    impl AttributeInterpolator<[f32; 3]> for SphericalInterpolator {
        fn interpolate(&mut self, a: [f32; 3], b: [f32; 3], p: f32) -> [f32; 3] {
            let a = Vec3::from(a);
            let b = Vec3::from(b);
            let angle = a.dot(b).acos();

            let sin = angle.sin().recip();
            let r = a * (((1.0 - p) * angle).sin() * sin) + b * ((p * angle).sin() * sin);
            r.into()
        }

        fn interpolate_half(&mut self, a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
            let a = Vec3::from(a);
            let b = Vec3::from(b);
            let r = (a + b) * (2.0 * (1.0 + a.dot(b))).sqrt().recip();
            r.into()
        }

        fn interpolate_multiple(
            &mut self,
            a: [f32; 3],
            b: [f32; 3],
            indices: &[u32],
            points: &mut [[f32; 3]],
        ) {
            let a = Vec3::from(a);
            let b = Vec3::from(b);

            let angle = a.dot(b).acos();
            let sin = angle.sin().recip();

            for (percent, index) in indices.iter().enumerate() {
                let percent = (percent + 1) as f32 / (indices.len() + 1) as f32;

                let r = a * (((1.0 - percent) * angle).sin() * sin)
                    + b * ((percent * angle).sin() * sin);
                points[*index as usize] = r.into();
            }
        }
    }

    impl AttributeInterpolator<[f32; 4]> for SphericalInterpolator {
        fn interpolate(&mut self, a: [f32; 4], b: [f32; 4], p: f32) -> [f32; 4] {
            let a = Vec4::from(a);
            let b = Vec4::from(b);
            let angle = a.dot(b).acos();

            let sin = angle.sin().recip();
            let r = a * (((1.0 - p) * angle).sin() * sin) + b * ((p * angle).sin() * sin);
            r.into()
        }

        fn interpolate_half(&mut self, a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
            let a = Vec4::from(a);
            let b = Vec4::from(b);
            let r = (a + b) * (2.0 * (1.0 + a.dot(b))).sqrt().recip();
            r.into()
        }

        fn interpolate_multiple(
            &mut self,
            a: [f32; 4],
            b: [f32; 4],
            indices: &[u32],
            points: &mut [[f32; 4]],
        ) {
            let a = Vec4::from(a);
            let b = Vec4::from(b);

            let angle = a.dot(b).acos();
            let sin = angle.sin().recip();

            for (percent, index) in indices.iter().enumerate() {
                let percent = (percent + 1) as f32 / (indices.len() + 1) as f32;

                let r = a * (((1.0 - percent) * angle).sin() * sin)
                    + b * ((percent * angle).sin() * sin);
                points[*index as usize] = r.into();
            }
        }
    }
}

<<<<<<< HEAD
impl<'a, T: Copy, I: AttributeInterpolator<T>> AttributeInterpolator<T> for &'a mut I {
    #[inline(always)]
    fn interpolate(&mut self, a: T, b: T, t: f32) -> T {
        I::interpolate(*self, a, b, t)
    }

    #[inline(always)]
    fn interpolate_half(&mut self, a: T, b: T) -> T {
        I::interpolate_half(*self, a, b)
    }

    #[inline]
    fn interpolate_multiple(&mut self, a: T, b: T, indices: &[u32], points: &mut [T]) {
        I::interpolate_multiple(*self, a, b, indices, points)
    }
}

pub trait Interpolator {
    type Int: AttributeInterpolator<i32>;
    type Int2: AttributeInterpolator<[i32; 2]>;
    type Int3: AttributeInterpolator<[i32; 3]>;
    type Int4: AttributeInterpolator<[i32; 4]>;
    type Uint: AttributeInterpolator<u32>;
    type Uint2: AttributeInterpolator<[u32; 2]>;
    type Uint3: AttributeInterpolator<[u32; 3]>;
    type Uint4: AttributeInterpolator<[u32; 4]>;
    type Float: AttributeInterpolator<f32>;
    type Float2: AttributeInterpolator<[f32; 2]>;
    type Float3: AttributeInterpolator<[f32; 3]>;
    type Float4: AttributeInterpolator<[f32; 4]>;
    type Uchar4Norm: AttributeInterpolator<[u8; 4]>;

    fn int(&mut self, name: &str) -> &mut Self::Int;
    fn int2(&mut self, name: &str) -> &mut Self::Int2;
    fn int3(&mut self, name: &str) -> &mut Self::Int3;
    fn int4(&mut self, name: &str) -> &mut Self::Int4;
    fn uint(&mut self, name: &str) -> &mut Self::Uint;
    fn uint2(&mut self, name: &str) -> &mut Self::Uint2;
    fn uint3(&mut self, name: &str) -> &mut Self::Uint3;
    fn uint4(&mut self, name: &str) -> &mut Self::Uint4;
    fn float(&mut self, name: &str) -> &mut Self::Float;
    fn float2(&mut self, name: &str) -> &mut Self::Float2;
    fn float3(&mut self, name: &str) -> &mut Self::Float3;
    fn float4(&mut self, name: &str) -> &mut Self::Float4;
    fn uchar4norm(&mut self, name: &str) -> &mut Self::Uchar4Norm;
=======
pub trait Interpolator {
    type Float32: AttributeInterpolator<f32>;
    type Sint32: AttributeInterpolator<i32>;
    type Uint32: AttributeInterpolator<u32>;
    type Float32x2: AttributeInterpolator<[f32; 2]>;
    type Sint32x2: AttributeInterpolator<[i32; 2]>;
    type Uint32x2: AttributeInterpolator<[u32; 2]>;
    type Float32x3: AttributeInterpolator<[f32; 3]>;
    type Sint32x3: AttributeInterpolator<[i32; 3]>;
    type Uint32x3: AttributeInterpolator<[u32; 3]>;
    type Float32x4: AttributeInterpolator<[f32; 4]>;
    type Sint32x4: AttributeInterpolator<[i32; 4]>;
    type Uint32x4: AttributeInterpolator<[u32; 4]>;
    type Sint16x2: AttributeInterpolator<[i16; 2]>;
    type Snorm16x2: AttributeInterpolator<[i16; 2]>;
    type Uint16x2: AttributeInterpolator<[u16; 2]>;
    type Unorm16x2: AttributeInterpolator<[u16; 2]>;
    type Sint16x4: AttributeInterpolator<[i16; 4]>;
    type Snorm16x4: AttributeInterpolator<[i16; 4]>;
    type Uint16x4: AttributeInterpolator<[u16; 4]>;
    type Unorm16x4: AttributeInterpolator<[u16; 4]>;
    type Sint8x2: AttributeInterpolator<[i8; 2]>;
    type Snorm8x2: AttributeInterpolator<[i8; 2]>;
    type Uint8x2: AttributeInterpolator<[u8; 2]>;
    type Unorm8x2: AttributeInterpolator<[u8; 2]>;
    type Sint8x4: AttributeInterpolator<[i8; 4]>;
    type Snorm8x4: AttributeInterpolator<[i8; 4]>;
    type Uint8x4: AttributeInterpolator<[u8; 4]>;
    type Unorm8x4: AttributeInterpolator<[u8; 4]>;

    fn float32(&mut self, name: &str) -> &mut Self::Float32;
    fn sint32(&mut self, name: &str) -> &mut Self::Sint32;
    fn uint32(&mut self, name: &str) -> &mut Self::Uint32;
    fn float32x2(&mut self, name: &str) -> &mut Self::Float32x2;
    fn sint32x2(&mut self, name: &str) -> &mut Self::Sint32x2;
    fn uint32x2(&mut self, name: &str) -> &mut Self::Uint32x2;
    fn float32x3(&mut self, name: &str) -> &mut Self::Float32x3;
    fn sint32x3(&mut self, name: &str) -> &mut Self::Sint32x3;
    fn uint32x3(&mut self, name: &str) -> &mut Self::Uint32x3;
    fn float32x4(&mut self, name: &str) -> &mut Self::Float32x4;
    fn sint32x4(&mut self, name: &str) -> &mut Self::Sint32x4;
    fn uint32x4(&mut self, name: &str) -> &mut Self::Uint32x4;
    fn sint16x2(&mut self, name: &str) -> &mut Self::Sint16x2;
    fn snorm16x2(&mut self, name: &str) -> &mut Self::Snorm16x2;
    fn uint16x2(&mut self, name: &str) -> &mut Self::Uint16x2;
    fn unorm16x2(&mut self, name: &str) -> &mut Self::Unorm16x2;
    fn sint16x4(&mut self, name: &str) -> &mut Self::Sint16x4;
    fn snorm16x4(&mut self, name: &str) -> &mut Self::Snorm16x4;
    fn uint16x4(&mut self, name: &str) -> &mut Self::Uint16x4;
    fn unorm16x4(&mut self, name: &str) -> &mut Self::Unorm16x4;
    fn sint8x2(&mut self, name: &str) -> &mut Self::Sint8x2;
    fn snorm8x2(&mut self, name: &str) -> &mut Self::Snorm8x2;
    fn uint8x2(&mut self, name: &str) -> &mut Self::Uint8x2;
    fn unorm8x2(&mut self, name: &str) -> &mut Self::Unorm8x2;
    fn sint8x4(&mut self, name: &str) -> &mut Self::Sint8x4;
    fn snorm8x4(&mut self, name: &str) -> &mut Self::Snorm8x4;
    fn uint8x4(&mut self, name: &str) -> &mut Self::Uint8x4;
    fn unorm8x4(&mut self, name: &str) -> &mut Self::Unorm8x4;
>>>>>>> temp
}

///
/// Everything is linearly interpolated.
///
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct StandardInterpolatorGroup {
    lerp: LinearInterpolator,
}

<<<<<<< HEAD
impl Interpolator for StandardInterpolatorGroup {
    type Int = LinearInterpolator;
    type Int2 = LinearInterpolator;
    type Int3 = LinearInterpolator;
    type Int4 = LinearInterpolator;
    type Uint = LinearInterpolator;
    type Uint2 = LinearInterpolator;
    type Uint3 = LinearInterpolator;
    type Uint4 = LinearInterpolator;
    type Float = LinearInterpolator;
    type Float2 = LinearInterpolator;
    type Float3 = LinearInterpolator;
    type Float4 = LinearInterpolator;
    type Uchar4Norm = LinearInterpolator;

    fn int(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn int2(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn int3(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn int4(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn uint(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn uint2(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn uint3(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn uint4(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn float(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn float2(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn float3(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn float4(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn uchar4norm(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
=======
#[rustfmt::skip]
impl Interpolator for StandardInterpolatorGroup {
    type Float32 = LinearInterpolator;
    type Sint32 = LinearInterpolator;
    type Uint32 = LinearInterpolator;
    type Float32x2 = LinearInterpolator;
    type Sint32x2 = LinearInterpolator;
    type Uint32x2 = LinearInterpolator;
    type Float32x3 = LinearInterpolator;
    type Sint32x3 = LinearInterpolator;
    type Uint32x3 = LinearInterpolator;
    type Float32x4 = LinearInterpolator;
    type Sint32x4 = LinearInterpolator;
    type Uint32x4 = LinearInterpolator;
    type Sint16x2 = LinearInterpolator;
    type Snorm16x2 = LinearInterpolator;
    type Uint16x2 = LinearInterpolator;
    type Unorm16x2 = LinearInterpolator;
    type Sint16x4 = LinearInterpolator;
    type Snorm16x4 = LinearInterpolator;
    type Uint16x4 = LinearInterpolator;
    type Unorm16x4 = LinearInterpolator;
    type Sint8x2 = LinearInterpolator;
    type Snorm8x2 = LinearInterpolator;
    type Uint8x2 = LinearInterpolator;
    type Unorm8x2 = LinearInterpolator;
    type Sint8x4 = LinearInterpolator;
    type Snorm8x4 = LinearInterpolator;
    type Uint8x4 = LinearInterpolator;
    type Unorm8x4 = LinearInterpolator;

    fn float32(&mut self, _name: &str) -> &mut Self::Float32 { &mut self.lerp }
    fn sint32(&mut self, _name: &str) -> &mut Self::Sint32 { &mut self.lerp }
    fn uint32(&mut self, _name: &str) -> &mut Self::Uint32 { &mut self.lerp }
    fn float32x2(&mut self, _name: &str) -> &mut Self::Float32x2 { &mut self.lerp }
    fn sint32x2(&mut self, _name: &str) -> &mut Self::Sint32x2 { &mut self.lerp }
    fn uint32x2(&mut self, _name: &str) -> &mut Self::Uint32x2 { &mut self.lerp }
    fn float32x3(&mut self, _name: &str) -> &mut Self::Float32x3 { &mut self.lerp }
    fn sint32x3(&mut self, _name: &str) -> &mut Self::Sint32x3 { &mut self.lerp }
    fn uint32x3(&mut self, _name: &str) -> &mut Self::Uint32x3 { &mut self.lerp }
    fn float32x4(&mut self, _name: &str) -> &mut Self::Float32x4 { &mut self.lerp }
    fn sint32x4(&mut self, _name: &str) -> &mut Self::Sint32x4 { &mut self.lerp }
    fn uint32x4(&mut self, _name: &str) -> &mut Self::Uint32x4 { &mut self.lerp }
    fn sint16x2(&mut self, _name: &str) -> &mut Self::Sint16x2 { &mut self.lerp }
    fn snorm16x2(&mut self, _name: &str) -> &mut Self::Snorm16x2 { &mut self.lerp }
    fn uint16x2(&mut self, _name: &str) -> &mut Self::Uint16x2 { &mut self.lerp }
    fn unorm16x2(&mut self, _name: &str) -> &mut Self::Unorm16x2 { &mut self.lerp }
    fn sint16x4(&mut self, _name: &str) -> &mut Self::Sint16x4 { &mut self.lerp }
    fn snorm16x4(&mut self, _name: &str) -> &mut Self::Snorm16x4 { &mut self.lerp }
    fn uint16x4(&mut self, _name: &str) -> &mut Self::Uint16x4 { &mut self.lerp }
    fn unorm16x4(&mut self, _name: &str) -> &mut Self::Unorm16x4 { &mut self.lerp }
    fn sint8x2(&mut self, _name: &str) -> &mut Self::Sint8x2 { &mut self.lerp }
    fn snorm8x2(&mut self, _name: &str) -> &mut Self::Snorm8x2 { &mut self.lerp }
    fn uint8x2(&mut self, _name: &str) -> &mut Self::Uint8x2 { &mut self.lerp }
    fn unorm8x2(&mut self, _name: &str) -> &mut Self::Unorm8x2 { &mut self.lerp }
    fn sint8x4(&mut self, _name: &str) -> &mut Self::Sint8x4 { &mut self.lerp }
    fn snorm8x4(&mut self, _name: &str) -> &mut Self::Snorm8x4 { &mut self.lerp }
    fn uint8x4(&mut self, _name: &str) -> &mut Self::Uint8x4 { &mut self.lerp }
    fn unorm8x4(&mut self, _name: &str) -> &mut Self::Unorm8x4 { &mut self.lerp }
>>>>>>> temp
}

///
/// Two, three, and four component floating point
/// attributes are spherically interpolated around
/// `(0, 0, 0)`. Everything else is linearly interpolated.
///
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct SphereInterpolatorGroup {
    slerp: SphericalInterpolator,
    lerp: LinearInterpolator,
}

<<<<<<< HEAD
impl Interpolator for SphereInterpolatorGroup {
    type Int = LinearInterpolator;
    type Int2 = LinearInterpolator;
    type Int3 = LinearInterpolator;
    type Int4 = LinearInterpolator;
    type Uint = LinearInterpolator;
    type Uint2 = LinearInterpolator;
    type Uint3 = LinearInterpolator;
    type Uint4 = LinearInterpolator;
    type Float = LinearInterpolator;
    type Float2 = SphericalInterpolator;
    type Float3 = SphericalInterpolator;
    type Float4 = SphericalInterpolator;
    type Uchar4Norm = LinearInterpolator;

    fn int(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn int2(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn int3(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn int4(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn uint(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn uint2(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn uint3(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn uint4(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn float(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn float2(&mut self, _: &str) -> &mut SphericalInterpolator {
        &mut self.slerp
    }
    fn float3(&mut self, _: &str) -> &mut SphericalInterpolator {
        &mut self.slerp
    }
    fn float4(&mut self, _: &str) -> &mut SphericalInterpolator {
        &mut self.slerp
    }
    fn uchar4norm(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
=======
#[rustfmt::skip]
impl Interpolator for SphereInterpolatorGroup {
    type Float32 = LinearInterpolator;
    type Sint32 = LinearInterpolator;
    type Uint32 = LinearInterpolator;
    type Float32x2 = SphericalInterpolator;
    type Sint32x2 = LinearInterpolator;
    type Uint32x2 = LinearInterpolator;
    type Float32x3 = SphericalInterpolator;
    type Sint32x3 = LinearInterpolator;
    type Uint32x3 = LinearInterpolator;
    type Float32x4 = SphericalInterpolator;
    type Sint32x4 = LinearInterpolator;
    type Uint32x4 = LinearInterpolator;
    type Sint16x2 = LinearInterpolator;
    type Snorm16x2 = LinearInterpolator;
    type Uint16x2 = LinearInterpolator;
    type Unorm16x2 = LinearInterpolator;
    type Sint16x4 = LinearInterpolator;
    type Snorm16x4 = LinearInterpolator;
    type Uint16x4 = LinearInterpolator;
    type Unorm16x4 = LinearInterpolator;
    type Sint8x2 = LinearInterpolator;
    type Snorm8x2 = LinearInterpolator;
    type Uint8x2 = LinearInterpolator;
    type Unorm8x2 = LinearInterpolator;
    type Sint8x4 = LinearInterpolator;
    type Snorm8x4 = LinearInterpolator;
    type Uint8x4 = LinearInterpolator;
    type Unorm8x4 = LinearInterpolator;

    fn float32(&mut self, _name: &str) -> &mut Self::Float32 { &mut self.lerp }
    fn sint32(&mut self, _name: &str) -> &mut Self::Sint32 { &mut self.lerp }
    fn uint32(&mut self, _name: &str) -> &mut Self::Uint32 { &mut self.lerp }
    fn float32x2(&mut self, _name: &str) -> &mut Self::Float32x2 { &mut self.slerp }
    fn sint32x2(&mut self, _name: &str) -> &mut Self::Sint32x2 { &mut self.lerp }
    fn uint32x2(&mut self, _name: &str) -> &mut Self::Uint32x2 { &mut self.lerp }
    fn float32x3(&mut self, _name: &str) -> &mut Self::Float32x3 { &mut self.slerp }
    fn sint32x3(&mut self, _name: &str) -> &mut Self::Sint32x3 { &mut self.lerp }
    fn uint32x3(&mut self, _name: &str) -> &mut Self::Uint32x3 { &mut self.lerp }
    fn float32x4(&mut self, _name: &str) -> &mut Self::Float32x4 { &mut self.slerp }
    fn sint32x4(&mut self, _name: &str) -> &mut Self::Sint32x4 { &mut self.lerp }
    fn uint32x4(&mut self, _name: &str) -> &mut Self::Uint32x4 { &mut self.lerp }
    fn sint16x2(&mut self, _name: &str) -> &mut Self::Sint16x2 { &mut self.lerp }
    fn snorm16x2(&mut self, _name: &str) -> &mut Self::Snorm16x2 { &mut self.lerp }
    fn uint16x2(&mut self, _name: &str) -> &mut Self::Uint16x2 { &mut self.lerp }
    fn unorm16x2(&mut self, _name: &str) -> &mut Self::Unorm16x2 { &mut self.lerp }
    fn sint16x4(&mut self, _name: &str) -> &mut Self::Sint16x4 { &mut self.lerp }
    fn snorm16x4(&mut self, _name: &str) -> &mut Self::Snorm16x4 { &mut self.lerp }
    fn uint16x4(&mut self, _name: &str) -> &mut Self::Uint16x4 { &mut self.lerp }
    fn unorm16x4(&mut self, _name: &str) -> &mut Self::Unorm16x4 { &mut self.lerp }
    fn sint8x2(&mut self, _name: &str) -> &mut Self::Sint8x2 { &mut self.lerp }
    fn snorm8x2(&mut self, _name: &str) -> &mut Self::Snorm8x2 { &mut self.lerp }
    fn uint8x2(&mut self, _name: &str) -> &mut Self::Uint8x2 { &mut self.lerp }
    fn unorm8x2(&mut self, _name: &str) -> &mut Self::Unorm8x2 { &mut self.lerp }
    fn sint8x4(&mut self, _name: &str) -> &mut Self::Sint8x4 { &mut self.lerp }
    fn snorm8x4(&mut self, _name: &str) -> &mut Self::Snorm8x4 { &mut self.lerp }
    fn uint8x4(&mut self, _name: &str) -> &mut Self::Uint8x4 { &mut self.lerp }
    fn unorm8x4(&mut self, _name: &str) -> &mut Self::Unorm8x4 { &mut self.lerp }
>>>>>>> temp
}

///
/// Two, three, and four component floating point
/// attributes are linearly interpolated and then
/// normalized. Everything else is only linearly
/// interpolated.
///
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct NormalizedInterpolatorGroup {
    nlerp: NormalizedLinearInterpolator,
    lerp: LinearInterpolator,
}

<<<<<<< HEAD
impl Interpolator for NormalizedInterpolatorGroup {
    type Int = LinearInterpolator;
    type Int2 = LinearInterpolator;
    type Int3 = LinearInterpolator;
    type Int4 = LinearInterpolator;
    type Uint = LinearInterpolator;
    type Uint2 = LinearInterpolator;
    type Uint3 = LinearInterpolator;
    type Uint4 = LinearInterpolator;
    type Float = LinearInterpolator;
    type Float2 = NormalizedLinearInterpolator;
    type Float3 = NormalizedLinearInterpolator;
    type Float4 = NormalizedLinearInterpolator;
    type Uchar4Norm = LinearInterpolator;

    fn int(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn int2(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn int3(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn int4(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn uint(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn uint2(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn uint3(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn uint4(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn float(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
    fn float2(&mut self, _: &str) -> &mut NormalizedLinearInterpolator {
        &mut self.nlerp
    }
    fn float3(&mut self, _: &str) -> &mut NormalizedLinearInterpolator {
        &mut self.nlerp
    }
    fn float4(&mut self, _: &str) -> &mut NormalizedLinearInterpolator {
        &mut self.nlerp
    }
    fn uchar4norm(&mut self, _: &str) -> &mut LinearInterpolator {
        &mut self.lerp
    }
}

///
/// Always returns the left hand side.
///
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct IdentityInterpolatorGroup {
    ilerp: IdentityInterpolator,
}

impl Interpolator for IdentityInterpolatorGroup {
    type Int = IdentityInterpolator;
    type Int2 = IdentityInterpolator;
    type Int3 = IdentityInterpolator;
    type Int4 = IdentityInterpolator;
    type Uint = IdentityInterpolator;
    type Uint2 = IdentityInterpolator;
    type Uint3 = IdentityInterpolator;
    type Uint4 = IdentityInterpolator;
    type Float = IdentityInterpolator;
    type Float2 = IdentityInterpolator;
    type Float3 = IdentityInterpolator;
    type Float4 = IdentityInterpolator;
    type Uchar4Norm = IdentityInterpolator;

    fn int(&mut self, _: &str) -> &mut IdentityInterpolator {
        &mut self.ilerp
    }
    fn int2(&mut self, _: &str) -> &mut IdentityInterpolator {
        &mut self.ilerp
    }
    fn int3(&mut self, _: &str) -> &mut IdentityInterpolator {
        &mut self.ilerp
    }
    fn int4(&mut self, _: &str) -> &mut IdentityInterpolator {
        &mut self.ilerp
    }
    fn uint(&mut self, _: &str) -> &mut IdentityInterpolator {
        &mut self.ilerp
    }
    fn uint2(&mut self, _: &str) -> &mut IdentityInterpolator {
        &mut self.ilerp
    }
    fn uint3(&mut self, _: &str) -> &mut IdentityInterpolator {
        &mut self.ilerp
    }
    fn uint4(&mut self, _: &str) -> &mut IdentityInterpolator {
        &mut self.ilerp
    }
    fn float(&mut self, _: &str) -> &mut IdentityInterpolator {
        &mut self.ilerp
    }
    fn float2(&mut self, _: &str) -> &mut IdentityInterpolator {
        &mut self.ilerp
    }
    fn float3(&mut self, _: &str) -> &mut IdentityInterpolator {
        &mut self.ilerp
    }
    fn float4(&mut self, _: &str) -> &mut IdentityInterpolator {
        &mut self.ilerp
    }
    fn uchar4norm(&mut self, _: &str) -> &mut IdentityInterpolator {
=======
#[rustfmt::skip]
impl Interpolator for NormalizedInterpolatorGroup {
    type Float32 = LinearInterpolator;
    type Sint32 = LinearInterpolator;
    type Uint32 = LinearInterpolator;
    type Float32x2 = NormalizedLinearInterpolator;
    type Sint32x2 = LinearInterpolator;
    type Uint32x2 = LinearInterpolator;
    type Float32x3 = NormalizedLinearInterpolator;
    type Sint32x3 = LinearInterpolator;
    type Uint32x3 = LinearInterpolator;
    type Float32x4 = NormalizedLinearInterpolator;
    type Sint32x4 = LinearInterpolator;
    type Uint32x4 = LinearInterpolator;
    type Sint16x2 = LinearInterpolator;
    type Snorm16x2 = LinearInterpolator;
    type Uint16x2 = LinearInterpolator;
    type Unorm16x2 = LinearInterpolator;
    type Sint16x4 = LinearInterpolator;
    type Snorm16x4 = LinearInterpolator;
    type Uint16x4 = LinearInterpolator;
    type Unorm16x4 = LinearInterpolator;
    type Sint8x2 = LinearInterpolator;
    type Snorm8x2 = LinearInterpolator;
    type Uint8x2 = LinearInterpolator;
    type Unorm8x2 = LinearInterpolator;
    type Sint8x4 = LinearInterpolator;
    type Snorm8x4 = LinearInterpolator;
    type Uint8x4 = LinearInterpolator;
    type Unorm8x4 = LinearInterpolator;

    fn float32(&mut self, _name: &str) -> &mut Self::Float32 { &mut self.lerp }
    fn sint32(&mut self, _name: &str) -> &mut Self::Sint32 { &mut self.lerp }
    fn uint32(&mut self, _name: &str) -> &mut Self::Uint32 { &mut self.lerp }
    fn float32x2(&mut self, _name: &str) -> &mut Self::Float32x2 { &mut self.nlerp }
    fn sint32x2(&mut self, _name: &str) -> &mut Self::Sint32x2 { &mut self.lerp }
    fn uint32x2(&mut self, _name: &str) -> &mut Self::Uint32x2 { &mut self.lerp }
    fn float32x3(&mut self, _name: &str) -> &mut Self::Float32x3 { &mut self.nlerp }
    fn sint32x3(&mut self, _name: &str) -> &mut Self::Sint32x3 { &mut self.lerp }
    fn uint32x3(&mut self, _name: &str) -> &mut Self::Uint32x3 { &mut self.lerp }
    fn float32x4(&mut self, _name: &str) -> &mut Self::Float32x4 { &mut self.nlerp }
    fn sint32x4(&mut self, _name: &str) -> &mut Self::Sint32x4 { &mut self.lerp }
    fn uint32x4(&mut self, _name: &str) -> &mut Self::Uint32x4 { &mut self.lerp }
    fn sint16x2(&mut self, _name: &str) -> &mut Self::Sint16x2 { &mut self.lerp }
    fn snorm16x2(&mut self, _name: &str) -> &mut Self::Snorm16x2 { &mut self.lerp }
    fn uint16x2(&mut self, _name: &str) -> &mut Self::Uint16x2 { &mut self.lerp }
    fn unorm16x2(&mut self, _name: &str) -> &mut Self::Unorm16x2 { &mut self.lerp }
    fn sint16x4(&mut self, _name: &str) -> &mut Self::Sint16x4 { &mut self.lerp }
    fn snorm16x4(&mut self, _name: &str) -> &mut Self::Snorm16x4 { &mut self.lerp }
    fn uint16x4(&mut self, _name: &str) -> &mut Self::Uint16x4 { &mut self.lerp }
    fn unorm16x4(&mut self, _name: &str) -> &mut Self::Unorm16x4 { &mut self.lerp }
    fn sint8x2(&mut self, _name: &str) -> &mut Self::Sint8x2 { &mut self.lerp }
    fn snorm8x2(&mut self, _name: &str) -> &mut Self::Snorm8x2 { &mut self.lerp }
    fn uint8x2(&mut self, _name: &str) -> &mut Self::Uint8x2 { &mut self.lerp }
    fn unorm8x2(&mut self, _name: &str) -> &mut Self::Unorm8x2 { &mut self.lerp }
    fn sint8x4(&mut self, _name: &str) -> &mut Self::Sint8x4 { &mut self.lerp }
    fn snorm8x4(&mut self, _name: &str) -> &mut Self::Snorm8x4 { &mut self.lerp }
    fn uint8x4(&mut self, _name: &str) -> &mut Self::Uint8x4 { &mut self.lerp }
    fn unorm8x4(&mut self, _name: &str) -> &mut Self::Unorm8x4 { &mut self.lerp }
}

///
/// Always returns the left hand side.
///
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct IdentityInterpolatorGroup {
    ilerp: IdentityInterpolator,
}

impl Interpolator for IdentityInterpolatorGroup {
    type Float32 = IdentityInterpolator;
    type Sint32 = IdentityInterpolator;
    type Uint32 = IdentityInterpolator;
    type Float32x2 = IdentityInterpolator;
    type Sint32x2 = IdentityInterpolator;
    type Uint32x2 = IdentityInterpolator;
    type Float32x3 = IdentityInterpolator;
    type Sint32x3 = IdentityInterpolator;
    type Uint32x3 = IdentityInterpolator;
    type Float32x4 = IdentityInterpolator;
    type Sint32x4 = IdentityInterpolator;
    type Uint32x4 = IdentityInterpolator;
    type Sint16x2 = IdentityInterpolator;
    type Snorm16x2 = IdentityInterpolator;
    type Uint16x2 = IdentityInterpolator;
    type Unorm16x2 = IdentityInterpolator;
    type Sint16x4 = IdentityInterpolator;
    type Snorm16x4 = IdentityInterpolator;
    type Uint16x4 = IdentityInterpolator;
    type Unorm16x4 = IdentityInterpolator;
    type Sint8x2 = IdentityInterpolator;
    type Snorm8x2 = IdentityInterpolator;
    type Uint8x2 = IdentityInterpolator;
    type Unorm8x2 = IdentityInterpolator;
    type Sint8x4 = IdentityInterpolator;
    type Snorm8x4 = IdentityInterpolator;
    type Uint8x4 = IdentityInterpolator;
    type Unorm8x4 = IdentityInterpolator;

    fn float32(&mut self, _name: &str) -> &mut Self::Float32 {
        &mut self.ilerp
    }
    fn sint32(&mut self, _name: &str) -> &mut Self::Sint32 {
        &mut self.ilerp
    }
    fn uint32(&mut self, _name: &str) -> &mut Self::Uint32 {
        &mut self.ilerp
    }
    fn float32x2(&mut self, _name: &str) -> &mut Self::Float32x2 {
        &mut self.ilerp
    }
    fn sint32x2(&mut self, _name: &str) -> &mut Self::Sint32x2 {
        &mut self.ilerp
    }
    fn uint32x2(&mut self, _name: &str) -> &mut Self::Uint32x2 {
        &mut self.ilerp
    }
    fn float32x3(&mut self, _name: &str) -> &mut Self::Float32x3 {
        &mut self.ilerp
    }
    fn sint32x3(&mut self, _name: &str) -> &mut Self::Sint32x3 {
        &mut self.ilerp
    }
    fn uint32x3(&mut self, _name: &str) -> &mut Self::Uint32x3 {
        &mut self.ilerp
    }
    fn float32x4(&mut self, _name: &str) -> &mut Self::Float32x4 {
        &mut self.ilerp
    }
    fn sint32x4(&mut self, _name: &str) -> &mut Self::Sint32x4 {
        &mut self.ilerp
    }
    fn uint32x4(&mut self, _name: &str) -> &mut Self::Uint32x4 {
        &mut self.ilerp
    }
    fn sint16x2(&mut self, _name: &str) -> &mut Self::Sint16x2 {
        &mut self.ilerp
    }
    fn snorm16x2(&mut self, _name: &str) -> &mut Self::Snorm16x2 {
        &mut self.ilerp
    }
    fn uint16x2(&mut self, _name: &str) -> &mut Self::Uint16x2 {
        &mut self.ilerp
    }
    fn unorm16x2(&mut self, _name: &str) -> &mut Self::Unorm16x2 {
        &mut self.ilerp
    }
    fn sint16x4(&mut self, _name: &str) -> &mut Self::Sint16x4 {
        &mut self.ilerp
    }
    fn snorm16x4(&mut self, _name: &str) -> &mut Self::Snorm16x4 {
        &mut self.ilerp
    }
    fn uint16x4(&mut self, _name: &str) -> &mut Self::Uint16x4 {
        &mut self.ilerp
    }
    fn unorm16x4(&mut self, _name: &str) -> &mut Self::Unorm16x4 {
        &mut self.ilerp
    }
    fn sint8x2(&mut self, _name: &str) -> &mut Self::Sint8x2 {
        &mut self.ilerp
    }
    fn snorm8x2(&mut self, _name: &str) -> &mut Self::Snorm8x2 {
        &mut self.ilerp
    }
    fn uint8x2(&mut self, _name: &str) -> &mut Self::Uint8x2 {
        &mut self.ilerp
    }
    fn unorm8x2(&mut self, _name: &str) -> &mut Self::Unorm8x2 {
        &mut self.ilerp
    }
    fn sint8x4(&mut self, _name: &str) -> &mut Self::Sint8x4 {
        &mut self.ilerp
    }
    fn snorm8x4(&mut self, _name: &str) -> &mut Self::Snorm8x4 {
        &mut self.ilerp
    }
    fn uint8x4(&mut self, _name: &str) -> &mut Self::Uint8x4 {
        &mut self.ilerp
    }
    fn unorm8x4(&mut self, _name: &str) -> &mut Self::Unorm8x4 {
>>>>>>> temp
        &mut self.ilerp
    }
}
