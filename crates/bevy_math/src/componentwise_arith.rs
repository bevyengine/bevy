//! Provides traits for componentwise arithmetic.
//!
//! This may be useful for types where there are no agreed upon standards for operators,
//! e.g. what would it mean to add two HSL-colors?

use glam::{Vec2, Vec3, Vec4};

/// The componentwise + operator
pub trait ComponentwiseAdd {
    /// The componentwise + operator
    fn componentwise_add(self, rhs: Self) -> Self;
}
/// The componentwise - operator
pub trait ComponentwiseSub {
    /// The componentwise - operator
    fn componentwise_sub(self, rhs: Self) -> Self;
}

/// The componentwise * operator
///
/// This should only be used for types where a componentwise multiplication makes sense.
/// For example, `Vec4` * `Vec4` or `Vec4` * `f32` allow for sensible componentwise multiplication
/// but `Vec4` * `Quat` does not.
pub trait ComponentwiseMul<T> {
    /// The componentwise * operator
    fn componentwise_mul(self, rhs: T) -> Self;
}
/// The componentwise / operator
///
/// This should only be used for types where a componentwise division makes sense.
/// For example, `Vec4` / `Vec4` or `Vec4` / `f32` allow for sensible componentwise division
/// but `Vec4` / `Quat` does not.
pub trait ComponentwiseDiv<T> {
    /// The componentwise / operator
    fn componentwise_div(self, rhs: T) -> Self;
}

#[macro_export]
/// Implements the `ComponentwiseAdd` trait for the type `ty` and its elements.
macro_rules! impl_componentwise_add {
    ($ty: ident, [$($element: ident),+]) => {
        impl $crate::componentwise_arith::ComponentwiseAdd for $ty {
            #[inline]
            fn componentwise_add(self, rhs: Self) -> Self {
                $ty {
                    $($element: self.$element + rhs.$element,)+
                }
            }
        }
    };
}
#[macro_export]
/// Implements the `ComponentwiseSub` trait for the type `ty` and its elements.
macro_rules! impl_componentwise_sub {
    ($ty: ident, [$($element: ident),+]) => {
        impl $crate::componentwise_arith::ComponentwiseSub for $ty {
            #[inline]
            fn componentwise_sub(self, rhs: Self) -> Self {
                $ty {
                    $($element: self.$element - rhs.$element,)+
                }
            }
        }
    };
}
#[macro_export]
/// Implements the `ComponentwiseMul<Self>` trait for the type `ty` and its elements.
macro_rules! impl_componentwise_mul {
    ($ty: ident, [$($element: ident),+]) => {
        impl $crate::componentwise_arith::ComponentwiseMul<Self> for $ty {
            #[inline]
            fn componentwise_mul(self, rhs: Self) -> Self {
                $ty {
                    $($element: self.$element * rhs.$element,)+
                }
            }
        }
    };
}
#[macro_export]
/// Implements the `ComponentwiseMul<scalar_ty>` trait for the type `ty` and its elements and a given scalar type `scalar_ty`.
macro_rules! impl_componentwise_scalar_mul {
    ($ty: ident, $scalar_ty: ident, [$($element: ident),+]) => {
        impl $crate::componentwise_arith::ComponentwiseMul<$scalar_ty> for $ty {
            #[inline]
            fn componentwise_mul(self, rhs: $scalar_ty) -> Self {
                $ty {
                    $($element: self.$element * rhs,)+
                }
            }
        }
    };
}
#[macro_export]
/// Implements the `ComponentwiseDiv<Self>` trait for the type `ty` and its elements.
macro_rules! impl_componentwise_div {
    ($ty: ident, [$($element: ident),+]) => {
        impl $crate::componentwise_arith::ComponentwiseDiv<Self> for $ty {
            #[inline]
            fn componentwise_div(self, rhs: Self) -> Self {
                $ty {
                    $($element: self.$element / rhs.$element,)+
                }
            }
        }
    };
}
#[macro_export]
/// Implements the `ComponentwiseDiv<scalar_ty>` trait for the type `ty` and its elements and a given scalar type `scalar_ty`.
macro_rules! impl_componentwise_scalar_div {
    ($ty: ident, $scalar_ty: ident, [$($element: ident),+]) => {
        impl $crate::componentwise_arith::ComponentwiseDiv<$scalar_ty> for $ty {
            #[inline]
            fn componentwise_div(self, rhs: $scalar_ty) -> Self {
                $ty {
                    $($element: self.$element / rhs,)+
                }
            }
        }
    };
}

impl_componentwise_add!(Vec2, [x, y]);
impl_componentwise_add!(Vec3, [x, y, z]);
impl_componentwise_add!(Vec4, [x, y, z, w]);

impl_componentwise_sub!(Vec2, [x, y]);
impl_componentwise_sub!(Vec3, [x, y, z]);
impl_componentwise_sub!(Vec4, [x, y, z, w]);

impl_componentwise_mul!(Vec2, [x, y]);
impl_componentwise_mul!(Vec3, [x, y, z]);
impl_componentwise_mul!(Vec4, [x, y, z, w]);
impl_componentwise_scalar_mul!(Vec2, f32, [x, y]);
impl_componentwise_scalar_mul!(Vec3, f32, [x, y, z]);
impl_componentwise_scalar_mul!(Vec4, f32, [x, y, z, w]);

impl_componentwise_div!(Vec2, [x, y]);
impl_componentwise_div!(Vec3, [x, y, z]);
impl_componentwise_div!(Vec4, [x, y, z, w]);
impl_componentwise_scalar_div!(Vec2, f32, [x, y]);
impl_componentwise_scalar_div!(Vec3, f32, [x, y, z]);
impl_componentwise_scalar_div!(Vec4, f32, [x, y, z, w]);
