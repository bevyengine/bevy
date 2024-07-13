//! This module holds marker traits for continuous and differentiable curves.
//!
//! These serve as guard rails to prevent using curves for operations that require differentiability
//! where it cannot be guaranteed by construction. On the other hand, these requirements may be
//! circumvented by a blessing procedure which can treat any curve with appropriate data as
//! continuous/differentiable (see [`Blessed`]).

use crate::{curve::Curve, HasTangent, WithDerivative, WithTwoDerivatives};
use std::ops::Deref;

/// Marker trait for curves used to express continuity.
pub trait ContinuousCurve<T>: Curve<T> {}

/// Marker trait for curves used to express differentiability. In using [`WithDerivative`], this
/// bakes in the computation of the derivative with the notion of differentiability.
///
/// The name is imprecise in that it is intended to connote C1 in the formal mathematical
/// sense — i.e. the derivative is expected not just to exist but also to be continuous.
pub trait DifferentiableCurve<T>: Curve<WithDerivative<T>>
where
    T: HasTangent,
{
}

/// Marker trait for curves used to express twice-differentiability. In using [`WithTwoDerivatives`],
/// this bakes in the computation of the two derivatives with the notion of differentiability.
///
/// Like [`DifferentiableCurve`], the name is mathematically imprecise: the second derivative is
/// required to be continuous, so this really connotes C2 in the formal mathematical sense.
pub trait TwiceDifferentiableCurve<T>: Curve<WithTwoDerivatives<T>>
where
    T: HasTangent,
    T::Tangent: HasTangent,
{
}

// Note: We cannot blanket implement these markers over `Deref` because there are conflicts with the
// implementations for `Blessed`, but we can do them for specific types to get coverage.
//
// In particular, `&C` covers the case of `Curve::by_ref`, which is the most important.
impl<T, C> ContinuousCurve<T> for &C where C: Curve<T> {}

impl<T, C> DifferentiableCurve<T> for &C
where
    T: HasTangent,
    C: Curve<WithDerivative<T>>,
{
}

impl<T, C> TwiceDifferentiableCurve<T> for &C
where
    T: HasTangent,
    T::Tangent: HasTangent,
    C: Curve<WithTwoDerivatives<T>>,
{
}

/// A wrapper that implements marker traits to circumvent the lack of known guarantees on the
/// underlying curve. This is usually used by invoking [`bless`] on the curve itself.
///
/// For instance, a `Curve<WithDerivative<T>>` may be known to be differentiable to the user without
/// this invariant being guaranteed by the constructions used to produce it. In such cases, the
/// wrapper is used to treat the curve as differentiable anyway.
///
/// [`bless`]: CurveBlessing::bless
pub struct Blessed<C>(pub C);

impl<C> Deref for Blessed<C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, C> ContinuousCurve<T> for Blessed<C> where C: Curve<T> {}

impl<T, C> DifferentiableCurve<T> for Blessed<C>
where
    T: HasTangent,
    C: Curve<WithDerivative<T>>,
{
}

impl<T, C> TwiceDifferentiableCurve<T> for Blessed<C>
where
    T: HasTangent,
    T::Tangent: HasTangent,
    C: Curve<WithTwoDerivatives<T>>,
{
}

/// Extension trait for curves which provides an ergonomic means of wrapping curves in [`Blessed`].
pub trait CurveBlessing<T>: Curve<T> + Sized {
    /// Bless this curve, allowing it to be treated as continuous, differentiable, and so on.
    fn bless(self) -> Blessed<Self> {
        Blessed(self)
    }
}

impl<T, C> CurveBlessing<T> for C where C: Curve<T> {}
