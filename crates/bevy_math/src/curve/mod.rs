//! The [`Curve`] trait, used to describe curves in a number of different domains. This module also
//! contains the [`Interval`] type, along with a selection of core data structures used to back
//! curves that are interpolated from samples.

pub mod interval;

pub use interval::{interval, Interval};

use interval::InvalidIntervalError;
use std::{marker::PhantomData, ops::Deref};
use thiserror::Error;

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

/// A trait for a type that can represent values of type `T` parametrized over a fixed interval.
/// Typical examples of this are actual geometric curves where `T: VectorSpace`, but other kinds
/// of output data can be represented as well.
pub trait Curve<T> {
    /// The interval over which this curve is parametrized.
    ///
    /// This is the range of values of `t` where we can sample the curve and receive valid output.
    fn domain(&self) -> Interval;

    /// Sample a point on this curve at the parameter value `t`, extracting the associated value.
    /// This is the unchecked version of sampling, which should only be used if the sample time `t`
    /// is already known to lie within the curve's domain.
    ///
    /// Values sampled from outside of a curve's domain are generally considered invalid; data which
    /// is nonsensical or otherwise useless may be returned in such a circumstance, and extrapolation
    /// beyond a curve's domain should not be relied upon.
    fn sample_unchecked(&self, t: f32) -> T;

    /// Sample a point on this curve at the parameter value `t`, returning `None` if the point is
    /// outside of the curve's domain.
    fn sample(&self, t: f32) -> Option<T> {
        match self.domain().contains(t) {
            true => Some(self.sample_unchecked(t)),
            false => None,
        }
    }

    /// Sample a point on this curve at the parameter value `t`, clamping `t` to lie inside the
    /// domain of the curve.
    fn sample_clamped(&self, t: f32) -> T {
        let t = self.domain().clamp(t);
        self.sample_unchecked(t)
    }

    /// Create a new curve by mapping the values of this curve via a function `f`; i.e., if the
    /// sample at time `t` for this curve is `x`, the value at time `t` on the new curve will be
    /// `f(x)`.
    fn map<S, F>(self, f: F) -> MapCurve<T, S, Self, F>
    where
        Self: Sized,
        F: Fn(T) -> S,
    {
        MapCurve {
            preimage: self,
            f,
            _phantom: PhantomData,
        }
    }

    /// Create a new [`Curve`] whose parameter space is related to the parameter space of this curve
    /// by `f`. For each time `t`, the sample from the new curve at time `t` is the sample from
    /// this curve at time `f(t)`. The given `domain` will be the domain of the new curve. The
    /// function `f` is expected to take `domain` into `self.domain()`.
    ///
    /// Note that this is the opposite of what one might expect intuitively; for example, if this
    /// curve has a parameter domain of `[0, 1]`, then stretching the parameter domain to
    /// `[0, 2]` would be performed as follows, dividing by what might be perceived as the scaling
    /// factor rather than multiplying:
    /// ```
    /// # use bevy_math::curve::*;
    /// let my_curve = constant_curve(interval(0.0, 1.0).unwrap(), 1.0);
    /// let scaled_curve = my_curve.reparametrize(interval(0.0, 2.0).unwrap(), |t| t / 2.0);
    /// ```
    /// This kind of linear remapping is provided by the convenience method
    /// [`Curve::reparametrize_linear`], which requires only the desired domain for the new curve.
    ///
    /// # Examples
    /// ```
    /// // Reverse a curve:
    /// # use bevy_math::curve::*;
    /// # use bevy_math::vec2;
    /// let my_curve = constant_curve(interval(0.0, 1.0).unwrap(), 1.0);
    /// let domain = my_curve.domain();
    /// let reversed_curve = my_curve.reparametrize(domain, |t| domain.end() - t);
    ///
    /// // Take a segment of a curve:
    /// # let my_curve = constant_curve(interval(0.0, 1.0).unwrap(), 1.0);
    /// let curve_segment = my_curve.reparametrize(interval(0.0, 0.5).unwrap(), |t| 0.5 + t);
    ///
    /// // Reparametrize by an easing curve:
    /// # let my_curve = constant_curve(interval(0.0, 1.0).unwrap(), 1.0);
    /// # let easing_curve = constant_curve(interval(0.0, 1.0).unwrap(), vec2(1.0, 1.0));
    /// let domain = my_curve.domain();
    /// let eased_curve = my_curve.reparametrize(domain, |t| easing_curve.sample_unchecked(t).y);
    /// ```
    fn reparametrize<F>(self, domain: Interval, f: F) -> ReparamCurve<T, Self, F>
    where
        Self: Sized,
        F: Fn(f32) -> f32,
    {
        ReparamCurve {
            domain,
            base: self,
            f,
            _phantom: PhantomData,
        }
    }

    /// Linearly reparametrize this [`Curve`], producing a new curve whose domain is the given
    /// `domain` instead of the current one. This operation is only valid for curves with bounded
    /// domains; if either this curve's domain or the given `domain` is unbounded, an error is
    /// returned.
    fn reparametrize_linear(
        self,
        domain: Interval,
    ) -> Result<LinearReparamCurve<T, Self>, LinearReparamError>
    where
        Self: Sized,
    {
        if !self.domain().is_bounded() {
            return Err(LinearReparamError::SourceCurveUnbounded);
        }

        if !domain.is_bounded() {
            return Err(LinearReparamError::TargetIntervalUnbounded);
        }

        Ok(LinearReparamCurve {
            base: self,
            new_domain: domain,
            _phantom: PhantomData,
        })
    }

    /// Reparametrize this [`Curve`] by sampling from another curve.
    ///
    /// The resulting curve samples at time `t` by first sampling `other` at time `t`, which produces
    /// another sample time `s` which is then used to sample this curve. The domain of the resulting
    /// curve is the domain of `other`.
    fn reparametrize_by_curve<C>(self, other: C) -> CurveReparamCurve<T, Self, C>
    where
        Self: Sized,
        C: Curve<f32>,
    {
        CurveReparamCurve {
            base: self,
            reparam_curve: other,
            _phantom: PhantomData,
        }
    }

    /// Create a new [`Curve`] which is the graph of this one; that is, its output echoes the sample
    /// time as part of a tuple.
    ///
    /// For example, if this curve outputs `x` at time `t`, then the produced curve will produce
    /// `(t, x)` at time `t`. In particular, if this curve is a `Curve<T>`, the output of this method
    /// is a `Curve<(f32, T)>`.
    fn graph(self) -> GraphCurve<T, Self>
    where
        Self: Sized,
    {
        GraphCurve {
            base: self,
            _phantom: PhantomData,
        }
    }

    /// Create a new [`Curve`] by zipping this curve together with another.
    ///
    /// The sample at time `t` in the new curve is `(x, y)`, where `x` is the sample of `self` at
    /// time `t` and `y` is the sample of `other` at time `t`. The domain of the new curve is the
    /// intersection of the domains of its constituents. If the domain intersection would be empty,
    /// an error is returned.
    fn zip<S, C>(self, other: C) -> Result<ProductCurve<T, S, Self, C>, InvalidIntervalError>
    where
        Self: Sized,
        C: Curve<S> + Sized,
    {
        let domain = self.domain().intersect(other.domain())?;
        Ok(ProductCurve {
            domain,
            first: self,
            second: other,
            _phantom: PhantomData,
        })
    }

    /// Create a new [`Curve`] by composing this curve end-to-end with another, producing another curve
    /// with outputs of the same type. The domain of the other curve is translated so that its start
    /// coincides with where this curve ends. A [`ChainError`] is returned if this curve's domain
    /// doesn't have a finite end or if `other`'s domain doesn't have a finite start.
    fn chain<C>(self, other: C) -> Result<ChainCurve<T, Self, C>, ChainError>
    where
        Self: Sized,
        C: Curve<T>,
    {
        if !self.domain().has_finite_end() {
            return Err(ChainError::FirstEndInfinite);
        }
        if !other.domain().has_finite_start() {
            return Err(ChainError::SecondStartInfinite);
        }
        Ok(ChainCurve {
            first: self,
            second: other,
            _phantom: PhantomData,
        })
    }

    /// Borrow this curve rather than taking ownership of it. This is essentially an alias for a
    /// prefix `&`; the point is that intermediate operations can be performed while retaining
    /// access to the original curve.
    ///
    /// # Example
    /// ```ignore
    /// # use bevy_math::curve::*;
    /// let my_curve = function_curve(interval(0.0, 1.0).unwrap(), |t| t * t + 1.0);
    /// // Borrow `my_curve` long enough to resample a mapped version. Note that `map` takes
    /// // ownership of its input.
    /// let samples = my_curve.by_ref().map(|x| x * 2.0).resample_auto(100).unwrap();
    /// // Do something else with `my_curve` since we retained ownership:
    /// let new_curve = my_curve.reparametrize_linear(interval(-1.0, 1.0).unwrap()).unwrap();
    /// ```
    fn by_ref(&self) -> &Self
    where
        Self: Sized,
    {
        self
    }

    /// Flip this curve so that its tuple output is arranged the other way.
    fn flip<U, V>(self) -> impl Curve<(V, U)>
    where
        Self: Sized + Curve<(U, V)>,
    {
        self.map(|(u, v)| (v, u))
    }
}

impl<T, C, D> Curve<T> for D
where
    C: Curve<T> + ?Sized,
    D: Deref<Target = C>,
{
    fn domain(&self) -> Interval {
        <C as Curve<T>>::domain(self)
    }

    fn sample_unchecked(&self, t: f32) -> T {
        <C as Curve<T>>::sample_unchecked(self, t)
    }
}

/// An error indicating that a linear reparametrization couldn't be performed because of
/// malformed inputs.
#[derive(Debug, Error)]
#[error("Could not build a linear function to reparametrize this curve")]
pub enum LinearReparamError {
    /// The source curve that was to be reparametrized had unbounded domain.
    #[error("This curve has unbounded domain")]
    SourceCurveUnbounded,

    /// The target interval for reparametrization was unbounded.
    #[error("The target interval for reparametrization is unbounded")]
    TargetIntervalUnbounded,
}

/// An error indicating that an end-to-end composition couldn't be performed because of
/// malformed inputs.
#[derive(Debug, Error)]
#[error("Could not compose these curves together")]
pub enum ChainError {
    /// The right endpoint of the first curve was infinite.
    #[error("The first curve's domain has an infinite end")]
    FirstEndInfinite,

    /// The left endpoint of the second curve was infinite.
    #[error("The second curve's domain has an infinite start")]
    SecondStartInfinite,
}

/// A curve with a constant value over its domain.
///
/// This is a curve that holds an inner value and always produces a clone of that value when sampled.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct ConstantCurve<T> {
    domain: Interval,
    value: T,
}

impl<T> ConstantCurve<T>
where
    T: Clone,
{
    /// Create a constant curve, which has the given `domain` and always produces the given `value`
    /// when sampled.
    pub fn new(domain: Interval, value: T) -> Self {
        Self { domain, value }
    }
}

impl<T> Curve<T> for ConstantCurve<T>
where
    T: Clone,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.domain
    }

    #[inline]
    fn sample_unchecked(&self, _t: f32) -> T {
        self.value.clone()
    }
}

/// A curve defined by a function together with a fixed domain.
///
/// This is a curve that holds an inner function `f` which takes numbers (`f32`) as input and produces
/// output of type `T`. The value of this curve when sampled at time `t` is just `f(t)`.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct FunctionCurve<T, F> {
    domain: Interval,
    f: F,
    _phantom: PhantomData<T>,
}

impl<T, F> FunctionCurve<T, F>
where
    F: Fn(f32) -> T,
{
    /// Create a new curve with the given `domain` from the given `function`. When sampled, the
    /// `function` is evaluated at the sample time to compute the output.
    pub fn new(domain: Interval, function: F) -> Self {
        FunctionCurve {
            domain,
            f: function,
            _phantom: PhantomData,
        }
    }
}

impl<T, F> Curve<T> for FunctionCurve<T, F>
where
    F: Fn(f32) -> T,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.domain
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        (self.f)(t)
    }
}

/// A curve whose samples are defined by mapping samples from another curve through a
/// given function. Curves of this type are produced by [`Curve::map`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct MapCurve<S, T, C, F> {
    preimage: C,
    f: F,
    _phantom: PhantomData<(S, T)>,
}

impl<S, T, C, F> Curve<T> for MapCurve<S, T, C, F>
where
    C: Curve<S>,
    F: Fn(S) -> T,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.preimage.domain()
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        (self.f)(self.preimage.sample_unchecked(t))
    }
}

/// A curve whose sample space is mapped onto that of some base curve's before sampling.
/// Curves of this type are produced by [`Curve::reparametrize`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct ReparamCurve<T, C, F> {
    domain: Interval,
    base: C,
    f: F,
    _phantom: PhantomData<T>,
}

impl<T, C, F> Curve<T> for ReparamCurve<T, C, F>
where
    C: Curve<T>,
    F: Fn(f32) -> f32,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.domain
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        self.base.sample_unchecked((self.f)(t))
    }
}

/// A curve that has had its domain changed by a linear reparametrization (stretching and scaling).
/// Curves of this type are produced by [`Curve::reparametrize_linear`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct LinearReparamCurve<T, C> {
    /// Invariants: The domain of this curve must always be bounded.
    base: C,
    /// Invariants: This interval must always be bounded.
    new_domain: Interval,
    _phantom: PhantomData<T>,
}

impl<T, C> Curve<T> for LinearReparamCurve<T, C>
where
    C: Curve<T>,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.new_domain
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        // The invariants imply this unwrap always succeeds.
        let f = self.new_domain.linear_map_to(self.base.domain()).unwrap();
        self.base.sample_unchecked(f(t))
    }
}

/// A curve that has been reparametrized by another curve, using that curve to transform the
/// sample times before sampling. Curves of this type are produced by [`Curve::reparametrize_by_curve`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct CurveReparamCurve<T, C, D> {
    base: C,
    reparam_curve: D,
    _phantom: PhantomData<T>,
}

impl<T, C, D> Curve<T> for CurveReparamCurve<T, C, D>
where
    C: Curve<T>,
    D: Curve<f32>,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.reparam_curve.domain()
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        let sample_time = self.reparam_curve.sample_unchecked(t);
        self.base.sample_unchecked(sample_time)
    }
}

/// A curve that is the graph of another curve over its parameter space. Curves of this type are
/// produced by [`Curve::graph`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct GraphCurve<T, C> {
    base: C,
    _phantom: PhantomData<T>,
}

impl<T, C> Curve<(f32, T)> for GraphCurve<T, C>
where
    C: Curve<T>,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.base.domain()
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> (f32, T) {
        (t, self.base.sample_unchecked(t))
    }
}

/// A curve that combines the output data from two constituent curves into a tuple output. Curves
/// of this type are produced by [`Curve::zip`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct ProductCurve<S, T, C, D> {
    domain: Interval,
    first: C,
    second: D,
    _phantom: PhantomData<(S, T)>,
}

impl<S, T, C, D> Curve<(S, T)> for ProductCurve<S, T, C, D>
where
    C: Curve<S>,
    D: Curve<T>,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.domain
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> (S, T) {
        (
            self.first.sample_unchecked(t),
            self.second.sample_unchecked(t),
        )
    }
}

/// The curve that results from chaining one curve with another. The second curve is
/// effectively reparametrized so that its start is at the end of the first.
///
/// For this to be well-formed, the first curve's domain must be right-finite and the second's
/// must be left-finite.
///
/// Curves of this type are produced by [`Curve::chain`].
pub struct ChainCurve<T, C, D> {
    first: C,
    second: D,
    _phantom: PhantomData<T>,
}

impl<T, C, D> Curve<T> for ChainCurve<T, C, D>
where
    C: Curve<T>,
    D: Curve<T>,
{
    #[inline]
    fn domain(&self) -> Interval {
        // This unwrap always succeeds because `first` has a valid Interval as its domain and the
        // length of `second` cannot be NAN. It's still fine if it's infinity.
        Interval::new(
            self.first.domain().start(),
            self.first.domain().end() + self.second.domain().length(),
        )
        .unwrap()
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        if t > self.first.domain().end() {
            self.second.sample_unchecked(
                // `t - first.domain.end` computes the offset into the domain of the second.
                t - self.first.domain().end() + self.second.domain().start(),
            )
        } else {
            self.first.sample_unchecked(t)
        }
    }
}

/// Create a [`Curve`] that constantly takes the given `value` over the given `domain`.
pub fn constant_curve<T: Clone>(domain: Interval, value: T) -> ConstantCurve<T> {
    ConstantCurve { domain, value }
}

/// Convert the given function `f` into a [`Curve`] with the given `domain`, sampled by
/// evaluating the function.
pub fn function_curve<T, F>(domain: Interval, f: F) -> FunctionCurve<T, F>
where
    F: Fn(f32) -> T,
{
    FunctionCurve {
        domain,
        f,
        _phantom: PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ops, Quat};
    use approx::{assert_abs_diff_eq, AbsDiffEq};
    use std::f32::consts::TAU;

    #[test]
    fn constant_curves() {
        let curve = constant_curve(Interval::EVERYWHERE, 5.0);
        assert!(curve.sample_unchecked(-35.0) == 5.0);

        let curve = constant_curve(interval(0.0, 1.0).unwrap(), true);
        assert!(curve.sample_unchecked(2.0));
        assert!(curve.sample(2.0).is_none());
    }

    #[test]
    fn function_curves() {
        let curve = function_curve(Interval::EVERYWHERE, |t| t * t);
        assert!(curve.sample_unchecked(2.0).abs_diff_eq(&4.0, f32::EPSILON));
        assert!(curve.sample_unchecked(-3.0).abs_diff_eq(&9.0, f32::EPSILON));

        let curve = function_curve(interval(0.0, f32::INFINITY).unwrap(), ops::log2);
        assert_eq!(curve.sample_unchecked(3.5), ops::log2(3.5));
        assert!(curve.sample_unchecked(-1.0).is_nan());
        assert!(curve.sample(-1.0).is_none());
    }

    #[test]
    fn mapping() {
        let curve = function_curve(Interval::EVERYWHERE, |t| t * 3.0 + 1.0);
        let mapped_curve = curve.map(|x| x / 7.0);
        assert_eq!(mapped_curve.sample_unchecked(3.5), (3.5 * 3.0 + 1.0) / 7.0);
        assert_eq!(
            mapped_curve.sample_unchecked(-1.0),
            (-1.0 * 3.0 + 1.0) / 7.0
        );
        assert_eq!(mapped_curve.domain(), Interval::EVERYWHERE);

        let curve = function_curve(interval(0.0, 1.0).unwrap(), |t| t * TAU);
        let mapped_curve = curve.map(Quat::from_rotation_z);
        assert_eq!(mapped_curve.sample_unchecked(0.0), Quat::IDENTITY);
        assert!(mapped_curve.sample_unchecked(1.0).is_near_identity());
        assert_eq!(mapped_curve.domain(), interval(0.0, 1.0).unwrap());
    }

    #[test]
    fn reparametrization() {
        let curve = function_curve(interval(1.0, f32::INFINITY).unwrap(), ops::log2);
        let reparametrized_curve = curve
            .by_ref()
            .reparametrize(interval(0.0, f32::INFINITY).unwrap(), ops::exp2);
        assert_abs_diff_eq!(reparametrized_curve.sample_unchecked(3.5), 3.5);
        assert_abs_diff_eq!(reparametrized_curve.sample_unchecked(100.0), 100.0);
        assert_eq!(
            reparametrized_curve.domain(),
            interval(0.0, f32::INFINITY).unwrap()
        );

        let reparametrized_curve = curve
            .by_ref()
            .reparametrize(interval(0.0, 1.0).unwrap(), |t| t + 1.0);
        assert_abs_diff_eq!(reparametrized_curve.sample_unchecked(0.0), 0.0);
        assert_abs_diff_eq!(reparametrized_curve.sample_unchecked(1.0), 1.0);
        assert_eq!(reparametrized_curve.domain(), interval(0.0, 1.0).unwrap());
    }

    #[test]
    fn multiple_maps() {
        // Make sure these actually happen in the right order.
        let curve = function_curve(interval(0.0, 1.0).unwrap(), ops::exp2);
        let first_mapped = curve.map(ops::log2);
        let second_mapped = first_mapped.map(|x| x * -2.0);
        assert_abs_diff_eq!(second_mapped.sample_unchecked(0.0), 0.0);
        assert_abs_diff_eq!(second_mapped.sample_unchecked(0.5), -1.0);
        assert_abs_diff_eq!(second_mapped.sample_unchecked(1.0), -2.0);
    }

    #[test]
    fn multiple_reparams() {
        // Make sure these happen in the right order too.
        let curve = function_curve(interval(0.0, 1.0).unwrap(), ops::exp2);
        let first_reparam = curve.reparametrize(interval(1.0, 2.0).unwrap(), ops::log2);
        let second_reparam = first_reparam.reparametrize(interval(0.0, 1.0).unwrap(), |t| t + 1.0);
        assert_abs_diff_eq!(second_reparam.sample_unchecked(0.0), 1.0);
        assert_abs_diff_eq!(second_reparam.sample_unchecked(0.5), 1.5);
        assert_abs_diff_eq!(second_reparam.sample_unchecked(1.0), 2.0);
    }
}
