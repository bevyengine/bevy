//! This module holds marker traits for continuous and differentiable curves.
//!
//! These serve as guard rails to prevent using curves for operations that require differentiability
//! where it cannot be guaranteed by construction. On the other hand, these requirements may be
//! circumvented by a blessing procedure which can treat any curve with appropriate data as
//! continuous/differentiable (see [`Blessed`]).

use crate::{curve::Curve, HasTangent, WithDerivative, WithTwoDerivatives};

/// Trait for curves that have a well-defined notion of derivative, allowing for derivatives
/// to be extracted along with values.
pub trait CurveWithDerivative<T>: Curve<T>
where
    T: HasTangent,
{
    fn with_derivative(self) -> impl Curve<WithDerivative<T>>;
}

/// Trait for curves that have a well-defined notion of second derivative, allowing for two
/// derivatives to be extracted along with values.
pub trait CurveWithTwoDerivatives<T>: CurveWithDerivative<T>
where
    T: HasTangent,
    T::Tangent: HasTangent,
{
    fn with_two_derivatives(self) -> impl Curve<WithTwoDerivatives<T>>;
}
