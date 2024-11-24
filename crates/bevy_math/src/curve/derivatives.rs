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

use crate::{
    common_traits::{HasTangent, WithDerivative, WithTwoDerivatives},
    curve::{Curve, Interval},
};
use core::ops::Deref;

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

pub trait SimpleDerivativeCurve<T>: Curve<T>
where
    T: HasTangent,
{
    fn sample_with_derivative_unchecked(&self, t: f32) -> WithDerivative<T>;

    fn sample_with_derivative(&self, t: f32) -> Option<WithDerivative<T>> {
        match self.domain().contains(t) {
            true => Some(self.sample_with_derivative_unchecked(t)),
            false => None,
        }
    }

    fn sample_with_derivative_clamped(&self, t: f32) -> WithDerivative<T> {
        let t = self.domain().clamp(t);
        self.sample_with_derivative_unchecked(t)
    }
}

impl<T, C, D> SimpleDerivativeCurve<T> for D
where
    T: HasTangent,
    C: SimpleDerivativeCurve<T> + ?Sized,
    D: Deref<Target = C>,
{
    fn sample_with_derivative_unchecked(&self, t: f32) -> WithDerivative<T> {
        <C as SimpleDerivativeCurve<T>>::sample_with_derivative_unchecked(self, t)
    }
}

pub trait SimpleTwoDerivativesCurve<T>: Curve<T>
where
    T: HasTangent,
    <T as HasTangent>::Tangent: HasTangent,
{
    fn sample_with_two_derivatives_unchecked(&self, t: f32) -> WithTwoDerivatives<T>;

    fn sample_with_two_derivatives(&self, t: f32) -> Option<WithTwoDerivatives<T>> {
        match self.domain().contains(t) {
            true => Some(self.sample_with_two_derivatives_unchecked(t)),
            false => None,
        }
    }

    fn sample_with_two_derivatives_clamped(&self, t: f32) -> WithTwoDerivatives<T> {
        let t = self.domain().clamp(t);
        self.sample_with_two_derivatives_unchecked(t)
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct SimpleDerivativeWrapper<C>(C);

impl<T, C> Curve<WithDerivative<T>> for SimpleDerivativeWrapper<C>
where
    T: HasTangent,
    C: SimpleDerivativeCurve<T>,
{
    fn domain(&self) -> Interval {
        self.0.domain()
    }

    fn sample_unchecked(&self, t: f32) -> WithDerivative<T> {
        self.0.sample_with_derivative_unchecked(t)
    }

    fn sample(&self, t: f32) -> Option<WithDerivative<T>> {
        self.0.sample_with_derivative(t)
    }

    fn sample_clamped(&self, t: f32) -> WithDerivative<T> {
        self.0.sample_with_derivative_clamped(t)
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct SimpleTwoDerivativesWrapper<C>(C);

impl<T, C> Curve<WithTwoDerivatives<T>> for SimpleTwoDerivativesWrapper<C>
where
    T: HasTangent,
    <T as HasTangent>::Tangent: HasTangent,
    C: SimpleTwoDerivativesCurve<T>,
{
    fn domain(&self) -> Interval {
        self.0.domain()
    }

    fn sample_unchecked(&self, t: f32) -> WithTwoDerivatives<T> {
        self.0.sample_with_two_derivatives_unchecked(t)
    }

    fn sample(&self, t: f32) -> Option<WithTwoDerivatives<T>> {
        self.0.sample_with_two_derivatives(t)
    }

    fn sample_clamped(&self, t: f32) -> WithTwoDerivatives<T> {
        self.0.sample_with_two_derivatives_clamped(t)
    }
}

impl<T, C> CurveWithDerivative<T> for C
where
    T: HasTangent,
    C: SimpleDerivativeCurve<T>,
{
    fn with_derivative(self) -> impl Curve<WithDerivative<T>> {
        SimpleDerivativeWrapper(self)
    }
}

impl<T, C> CurveWithTwoDerivatives<T> for C
where
    T: HasTangent,
    <T as HasTangent>::Tangent: HasTangent,
    C: SimpleTwoDerivativesCurve<T> + CurveWithDerivative<T>,
{
    fn with_two_derivatives(self) -> impl Curve<WithTwoDerivatives<T>> {
        SimpleTwoDerivativesWrapper(self)
    }
}
