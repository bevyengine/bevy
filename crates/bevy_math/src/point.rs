//! Provides interfaces for points in space of any dimension that support the math ops needed for cubic spline
//! interpolation.
use std::fmt::Debug;

use glam::{Quat, Vec2, Vec3, Vec3A, Vec4};

/// A point in space of any dimension that supports the math ops needed for cubic spline
/// interpolation.
pub trait Point:
    PointAdd + PointSub + PointMul + PointDiv + Default + Debug + Clone + Copy
{
}

/// The + operator that should be used for points on cubic splines.
pub trait PointAdd {
    /// The + operator that should be used for points on cubic splines.
    fn point_add(self, rhs: Self) -> Self;
}

/// The - operator that should be used for points on cubic splines.
pub trait PointSub {
    /// The - operator that should be used for points on cubic splines.
    fn point_sub(self, rhs: Self) -> Self;
}

/// The * operator that should be used for points on cubic splines.
pub trait PointMul {
    /// The * operator that should be used for points on cubic splines.
    fn point_mul(self, rhs: f32) -> Self;
}

/// The / operator that should be used for points on cubic splines.
pub trait PointDiv {
    /// The / operator that should be used for points on cubic splines.
    fn point_div(self, rhs: f32) -> Self;
}

#[macro_export]
/// Implements `PointAdd`, `PointSub`, `PointMul`, `PointDiv` and `Point` for the provided type using the 
/// `ComponentwiseAdd`, `ComponentwiseSub`, `ComponentwiseMul` and `ComponentwiseDiv` implementations of that type.
/// 
/// Please note that this macro does not implement any componentwise traits or other bounds introduced by `Point`.
macro_rules! impl_componentwise_point {
    ($ty: ident) => {
        impl $crate::point::PointAdd for $ty {
            #[inline]
            fn point_add(self, rhs: $ty) -> $ty {
                $crate::componentwise_arith::ComponentwiseAdd::componentwise_add(self, rhs)
            }
        }
        impl $crate::point::PointSub for $ty {
            #[inline]
            fn point_sub(self, rhs: $ty) -> $ty {
                $crate::componentwise_arith::ComponentwiseSub::componentwise_sub(self, rhs)
            }
        }
        impl $crate::point::PointMul for $ty {
            #[inline]
            fn point_mul(self, rhs: f32) -> $ty {
                $crate::componentwise_arith::ComponentwiseMul::componentwise_mul(self, rhs)
            }
        }
        impl $crate::point::PointDiv for $ty {
            #[inline]
            fn point_div(self, rhs: f32) -> $ty {
                $crate::componentwise_arith::ComponentwiseDiv::componentwise_div(self, rhs)
            }
        }
        impl $crate::point::Point for $ty {}
    };
}

#[macro_export]
/// Implements `PointAdd`, `PointSub`, `PointMul`, `PointDiv` and `Point` for the provided type using the 
/// `std::ops::Add`, `std::ops::Sub`, `std::ops::Mul` and `std::ops::Div` implementations of that type.
/// 
/// Please note that this macro does not implement any of the `std::ops::*` traits or other bounds introduced by `Point`.
macro_rules! impl_std_ops_point {
    ($ty: ident) => {
        impl $crate::point::PointAdd for $ty {
            #[inline]
            fn point_add(self, rhs: $ty) -> $ty {
                self + rhs
            }
        }
        impl $crate::point::PointSub for $ty {
            #[inline]
            fn point_sub(self, rhs: $ty) -> $ty {
                self - rhs
            }
        }
        impl $crate::point::PointMul for $ty {
            #[inline]
            fn point_mul(self, rhs: f32) -> $ty {
                self * rhs
            }
        }
        impl $crate::point::PointDiv for $ty {
            #[inline]
            fn point_div(self, rhs: f32) -> $ty {
                self / rhs
            }
        }
        impl $crate::point::Point for $ty {}
    };
}

impl_std_ops_point!(Quat);
impl_std_ops_point!(Vec4);
impl_std_ops_point!(Vec3);
impl_std_ops_point!(Vec3A);
impl_std_ops_point!(Vec2);
impl_std_ops_point!(f32);
