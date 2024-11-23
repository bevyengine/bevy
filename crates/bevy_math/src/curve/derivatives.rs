//! This module holds traits related to extracting derivatives from curves. In
//! applications, the derivatives of interest are chiefly the first and second;
//! in this module, these are provided by the traits [`CurveWithDerivative`]
//! and [`CurveWithTwoDerivatives`].
//!
//! These take ownership of the curve they are used on by default, so that
//! the resulting output may be used in more durable contexts. For example,
//! `CurveWithDerivative<T>` is not dyn-compatible, but `Curve<WithDerivative<T>>`
//! is, so if such a curve needs to be stored in a dynamic context, calling
//! [`with_derivative`] and then placing the result in a
//! `Box<Curve<WithDerivative<T>>>` is sensible.
//!
//! On the other hand, in more transient contexts, consuming a value merely to
//! sample derivatives is inconvenient, and in these cases, it is recommended
//! to use [`by_ref`] when possible to create a referential curve first, retaining
//! liveness of the original.
//!
//! [`with_derivative`]: CurveWithDerivative::with_derivative
//! [`by_ref`]: Curve::by_ref

use crate::{curve::Curve, HasTangent, WithDerivative, WithTwoDerivatives};

/// Trait for curves that have a well-defined notion of derivative, allowing for
/// derivatives to be extracted along with values.
pub trait CurveWithDerivative<T>: Curve<T>
where
    T: HasTangent,
{
    /// This curve, but with its first derivative included in sampling.
    fn with_derivative(self) -> impl Curve<WithDerivative<T>>;
}

/// Trait for curves that have a well-defined notion of second derivative,
/// allowing for two derivatives to be extracted along with values.
pub trait CurveWithTwoDerivatives<T>: CurveWithDerivative<T>
where
    T: HasTangent,
    T::Tangent: HasTangent,
{
    /// This curve, but with its first two derivatives included in sampling.
    fn with_two_derivatives(self) -> impl Curve<WithTwoDerivatives<T>>;
}
