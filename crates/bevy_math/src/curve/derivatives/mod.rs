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
//! This module also holds the [`SampleDerivative`] and [`SampleTwoDerivatives`]
//! traits, which can be used to easily implement `CurveWithDerivative` and its
//! counterpart.
//!
//! [`with_derivative`]: CurveWithDerivative::with_derivative
//! [`by_ref`]: crate::curve::CurveExt::by_ref

pub mod adaptor_impls;

use crate::{
    common_traits::{HasTangent, WithDerivative, WithTwoDerivatives},
    curve::{Curve, Interval},
};
use core::ops::Deref;

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{FromReflect, Reflect};

/// Trait for curves that have a well-defined notion of derivative, allowing for
/// derivatives to be extracted along with values.
///
/// This is implemented by implementing [`SampleDerivative`].
pub trait CurveWithDerivative<T>: SampleDerivative<T> + Sized
where
    T: HasTangent,
{
    /// This curve, but with its first derivative included in sampling.
    ///
    /// Notably, the output type is a `Curve<WithDerivative<T>>`.
    fn with_derivative(self) -> SampleDerivativeWrapper<Self>;
}

/// Trait for curves that have a well-defined notion of second derivative,
/// allowing for two derivatives to be extracted along with values.
///
/// This is implemented by implementing [`SampleTwoDerivatives`].
pub trait CurveWithTwoDerivatives<T>: SampleTwoDerivatives<T> + Sized
where
    T: HasTangent,
{
    /// This curve, but with its first two derivatives included in sampling.
    ///
    /// Notably, the output type is a `Curve<WithTwoDerivatives<T>>`.
    fn with_two_derivatives(self) -> SampleTwoDerivativesWrapper<Self>;
}

/// A trait for curves that can sample derivatives in addition to values.
///
/// Types that implement this trait automatically implement [`CurveWithDerivative`];
/// the curve produced by [`with_derivative`] uses the sampling defined in the trait
/// implementation.
///
/// [`with_derivative`]: CurveWithDerivative::with_derivative
pub trait SampleDerivative<T>: Curve<T>
where
    T: HasTangent,
{
    /// Sample this curve at the parameter value `t`, extracting the associated value
    /// in addition to its derivative. This is the unchecked version of sampling, which
    /// should only be used if the sample time `t` is already known to lie within the
    /// curve's domain.
    ///
    /// See [`Curve::sample_unchecked`] for more information.
    fn sample_with_derivative_unchecked(&self, t: f32) -> WithDerivative<T>;

    /// Sample this curve's value and derivative at the parameter value `t`, returning
    /// `None` if the point is outside of the curve's domain.
    fn sample_with_derivative(&self, t: f32) -> Option<WithDerivative<T>> {
        match self.domain().contains(t) {
            true => Some(self.sample_with_derivative_unchecked(t)),
            false => None,
        }
    }

    /// Sample this curve's value and derivative at the parameter value `t`, clamping `t`
    /// to lie inside the domain of the curve.
    fn sample_with_derivative_clamped(&self, t: f32) -> WithDerivative<T> {
        let t = self.domain().clamp(t);
        self.sample_with_derivative_unchecked(t)
    }
}

impl<T, C, D> SampleDerivative<T> for D
where
    T: HasTangent,
    C: SampleDerivative<T> + ?Sized,
    D: Deref<Target = C>,
{
    fn sample_with_derivative_unchecked(&self, t: f32) -> WithDerivative<T> {
        <C as SampleDerivative<T>>::sample_with_derivative_unchecked(self, t)
    }
}

/// A trait for curves that can sample two derivatives in addition to values.
///
/// Types that implement this trait automatically implement [`CurveWithTwoDerivatives`];
/// the curve produced by [`with_two_derivatives`] uses the sampling defined in the trait
/// implementation.
///
/// [`with_two_derivatives`]: CurveWithTwoDerivatives::with_two_derivatives
pub trait SampleTwoDerivatives<T>: Curve<T>
where
    T: HasTangent,
{
    /// Sample this curve at the parameter value `t`, extracting the associated value
    /// in addition to two derivatives. This is the unchecked version of sampling, which
    /// should only be used if the sample time `t` is already known to lie within the
    /// curve's domain.
    ///
    /// See [`Curve::sample_unchecked`] for more information.
    fn sample_with_two_derivatives_unchecked(&self, t: f32) -> WithTwoDerivatives<T>;

    /// Sample this curve's value and two derivatives at the parameter value `t`, returning
    /// `None` if the point is outside of the curve's domain.
    fn sample_with_two_derivatives(&self, t: f32) -> Option<WithTwoDerivatives<T>> {
        match self.domain().contains(t) {
            true => Some(self.sample_with_two_derivatives_unchecked(t)),
            false => None,
        }
    }

    /// Sample this curve's value and two derivatives at the parameter value `t`, clamping `t`
    /// to lie inside the domain of the curve.
    fn sample_with_two_derivatives_clamped(&self, t: f32) -> WithTwoDerivatives<T> {
        let t = self.domain().clamp(t);
        self.sample_with_two_derivatives_unchecked(t)
    }
}

/// A wrapper that uses a [`SampleDerivative<T>`] curve to produce a `Curve<WithDerivative<T>>`.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect, FromReflect),
    reflect(from_reflect = false)
)]
pub struct SampleDerivativeWrapper<C>(C);

impl<T, C> Curve<WithDerivative<T>> for SampleDerivativeWrapper<C>
where
    T: HasTangent,
    C: SampleDerivative<T>,
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

/// A wrapper that uses a [`SampleTwoDerivatives<T>`] curve to produce a
/// `Curve<WithTwoDerivatives<T>>`.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect, FromReflect),
    reflect(from_reflect = false)
)]
pub struct SampleTwoDerivativesWrapper<C>(C);

impl<T, C> Curve<WithTwoDerivatives<T>> for SampleTwoDerivativesWrapper<C>
where
    T: HasTangent,
    C: SampleTwoDerivatives<T>,
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
    C: SampleDerivative<T>,
{
    fn with_derivative(self) -> SampleDerivativeWrapper<Self> {
        SampleDerivativeWrapper(self)
    }
}

impl<T, C> CurveWithTwoDerivatives<T> for C
where
    T: HasTangent,
    C: SampleTwoDerivatives<T> + CurveWithDerivative<T>,
{
    fn with_two_derivatives(self) -> SampleTwoDerivativesWrapper<Self> {
        SampleTwoDerivativesWrapper(self)
    }
}
