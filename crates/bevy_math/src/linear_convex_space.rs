//! This module provides the `LinearConvexSpace` trait and some related objects
use glam::{DVec2, DVec3, DVec4, Quat, UVec2, UVec3, UVec4, Vec2, Vec3, Vec3A, Vec4};

/// A trait providing methods for spanning a linear, convex space.
///
/// These are a generally useful set of properties, but in particular, are needed to ensure spline interpolation is well-behaved.
/// A convex space or convex set is a set of points (think points in 3d or 2d space) in which every point on a line between two points A and B in that set
/// is also a part of that set. For example the set of all points inside a square is a convex set as you can draw a line between any two points in the square and never leave it.
/// For more information, please see [this wikipedia article](https://en.wikipedia.org/wiki/Convex_set).
///
/// The space must also be linear,  allowing you to interpolate between any two points `A` and `B` via the parameter
/// `t`  according to the formula `A*t + B*(1-t)`. This formula produces a new point on the straight line joining `A` and `B`,
/// no matter what value of `t` between 0 (yielding B) and 1 (yielding A) is selected.
///
/// By implementing this trait, you guarantee that the above conditions hold true.
pub trait LinearConvexSpace: Default + Copy + Clone {
    /// The scalar type to be used with this space.
    type Scalar: Copy + Clone;

    /// Adds two elements of the space.
    fn add(self, rhs: Self) -> Self;
    /// Subtracts one element in the space from another one.
    fn sub(self, rhs: Self) -> Self;
    /// Scales an element of the space by a scalar.
    fn scale(self, rhs: Self::Scalar) -> Self;
    /// Scales an element of the space by the reciprocal of a scalar, effectifely dividing by that scalar.
    fn scale_recip(self, rhs: Self::Scalar) -> Self;
}

#[macro_export]
/// Implements `LinearConvexSpace<Scalar>` for a given type `T` using the
/// `Add`, `Sub`, `Mul` and `Div` implementations of `T`.
///
/// Please note that you still need to derive the bounds introduced by `LinearConvexSpace` separately.
macro_rules! impl_linear_convex_space {
    ($ty: ident, $scalar_ty: ident) => {
        impl $crate::linear_convex_space::LinearConvexSpace for $ty {
            type Scalar = $scalar_ty;

            #[inline]
            fn add(self, rhs: Self) -> Self {
                self + rhs
            }
            #[inline]
            fn sub(self, rhs: Self) -> Self {
                self - rhs
            }
            #[inline]
            fn scale(self, rhs: $scalar_ty) -> Self {
                self * rhs
            }
            #[inline]
            fn scale_recip(self, rhs: $scalar_ty) -> Self {
                self / rhs
            }
        }
    };
}

pub use impl_linear_convex_space;

impl_linear_convex_space!(f32, f32);
impl_linear_convex_space!(Vec2, f32);
impl_linear_convex_space!(Vec3, f32);
impl_linear_convex_space!(Vec3A, f32);
impl_linear_convex_space!(Vec4, f32);
impl_linear_convex_space!(Quat, f32);

impl_linear_convex_space!(f64, f64);
impl_linear_convex_space!(DVec2, f64);
impl_linear_convex_space!(DVec3, f64);
impl_linear_convex_space!(DVec4, f64);

impl_linear_convex_space!(u32, u32);
impl_linear_convex_space!(UVec2, u32);
impl_linear_convex_space!(UVec3, u32);
impl_linear_convex_space!(UVec4, u32);
