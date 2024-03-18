//! This module provides the `LinearConvexSpace` trait and some related objects
use glam::{DVec2, DVec3, DVec4, Quat, UVec2, UVec3, UVec4, Vec2, Vec3, Vec3A, Vec4};

/// A trait providing methods for spanning a linear, convex space.
///
/// A convex space or convex set is a set of points (think points in 3d or 2d space) in which every point on a line between two points A and B in that set
/// is also a part of that set. For example the set of all points inside a square is a convex set as you can draw a line between any two points in the square and never leave it.
/// For more information, please see [this wikipedia article](https://en.wikipedia.org/wiki/Convex_set).
///
/// The space should also be linear, meaning that `A*t + B*(1 - t)` with any points `A`, `B` and `t` element `0..1` should represent a straight line.
///
/// By implementing this trait, you guarantee that the above conditions hold true.
pub trait LinearConvexSpace<Scalar>: Default + Copy + Clone {
    /// Adds two elements of the space.
    fn add(self, rhs: Self) -> Self;
    /// Subtracts one element in the space from another one.
    fn sub(self, rhs: Self) -> Self;
    /// Multiplies an element of the space by a scalar.
    fn mul(self, rhs: Scalar) -> Self;
    /// Divides an element of the space by a scalar.
    fn div(self, rhs: Scalar) -> Self;
}

#[macro_export]
/// Implements `LinearConvexSpace<Scalar>` for a given type `T` using the
/// `Add`, `Sub`, `Mul` and `Div` implementations of `T`.
///
/// Please note that you still need to derive the bounds introduced by `LinearConvexSpace` separately.
macro_rules! impl_linear_convex_space {
    ($ty: ident, $scalar_ty: ident) => {
        impl $crate::linear_convex_space::LinearConvexSpace<$scalar_ty> for $ty {
            #[inline]
            fn add(self, rhs: Self) -> Self {
                self + rhs
            }
            #[inline]
            fn sub(self, rhs: Self) -> Self {
                self - rhs
            }
            #[inline]
            fn mul(self, rhs: $scalar_ty) -> Self {
                self * rhs
            }
            #[inline]
            fn div(self, rhs: $scalar_ty) -> Self {
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
