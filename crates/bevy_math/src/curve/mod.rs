//! The [`Curve`] trait, used to describe curves in a number of different domains. This module also
//! contains the [`Interval`] type, along with a selection of core data structures used to back
//! curves that are interpolated from samples.

pub mod cores;
pub mod interval;

pub use interval::{everywhere, interval, Interval};

use crate::StableInterpolate;
use cores::{EvenCore, EvenCoreError, UnevenCore, UnevenCoreError};
use interval::{InfiniteIntervalError, InvalidIntervalError};
use std::{marker::PhantomData, ops::Deref};
use thiserror::Error;

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

/// A trait for a type that can represent values of type `T` parametrized over a fixed interval.
/// Typical examples of this are actual geometric curves where `T: VectorSpace`, but other kinds
/// of interpolable data can be represented instead (or in addition).
pub trait Curve<T> {
    /// The interval over which this curve is parametrized.
    fn domain(&self) -> Interval;

    /// Sample a point on this curve at the parameter value `t`, extracting the associated value.
    fn sample(&self, t: f32) -> T;

    /// Sample a point on this curve at the parameter value `t`, returning `None` if the point is
    /// outside of the curve's domain.
    fn sample_checked(&self, t: f32) -> Option<T> {
        match self.domain().contains(t) {
            true => Some(self.sample(t)),
            false => None,
        }
    }

    /// Sample a point on this curve at the parameter value `t`, clamping `t` to lie inside the
    /// domain of the curve.
    fn sample_clamped(&self, t: f32) -> T {
        let t = self.domain().clamp(t);
        self.sample(t)
    }

    /// Resample this [`Curve`] to produce a new one that is defined by interpolation over equally
    /// spaced values, using the provided `interpolation` to interpolate between adjacent samples.
    /// A total of `samples` samples are used, although at least two samples are required to produce
    /// well-formed output. If fewer than two samples are provided, or if this curve has an unbounded
    /// domain, then a [`ResamplingError`] is returned.
    ///
    /// The interpolation takes two values by reference together with a scalar parameter and
    /// produces an owned value. The expectation is that `interpolation(&x, &y, 0.0)` and
    /// `interpolation(&x, &y, 1.0)` are equivalent to `x` and `y` respectively.
    ///
    /// # Example
    /// ```
    /// # use bevy_math::*;
    /// # use bevy_math::curve::*;
    /// let quarter_rotation = function_curve(interval(0.0, 90.0).unwrap(), |t| Rot2::degrees(t));
    /// // A curve which only stores three data points and uses `nlerp` to interpolate them:
    /// let resampled_rotation = quarter_rotation.resample(3, |x, y, t| x.nlerp(*y, t));
    /// ```
    fn resample<I>(
        &self,
        samples: usize,
        interpolation: I,
    ) -> Result<SampleCurve<T, I>, ResamplingError>
    where
        Self: Sized,
        I: Fn(&T, &T, f32) -> T,
    {
        if samples < 2 {
            return Err(ResamplingError::NotEnoughSamples(samples));
        }
        if !self.domain().is_finite() {
            return Err(ResamplingError::InfiniteInterval(InfiniteIntervalError));
        }

        let samples: Vec<T> = self
            .domain()
            .spaced_points(samples)
            .unwrap()
            .map(|t| self.sample(t))
            .collect();
        Ok(SampleCurve {
            core: EvenCore {
                domain: self.domain(),
                samples,
            },
            interpolation,
        })
    }

    /// Resample this [`Curve`] to produce a new one that is defined by interpolation over equally
    /// spaced values. A total of `samples` samples are used, although at least two samples are
    /// required in order to produce well-formed output. If fewer than two samples are provided,
    /// or if this curve has an unbounded domain, then a [`ResamplingError`] is returned.
    fn resample_auto(&self, samples: usize) -> Result<SampleAutoCurve<T>, ResamplingError>
    where
        T: StableInterpolate,
    {
        if samples < 2 {
            return Err(ResamplingError::NotEnoughSamples(samples));
        }
        if !self.domain().is_finite() {
            return Err(ResamplingError::InfiniteInterval(InfiniteIntervalError));
        }

        let samples: Vec<T> = self
            .domain()
            .spaced_points(samples)
            .unwrap()
            .map(|t| self.sample(t))
            .collect();
        Ok(SampleAutoCurve {
            core: EvenCore {
                domain: self.domain(),
                samples,
            },
        })
    }

    /// Extract an iterator over evenly-spaced samples from this curve. If `samples` is less than 2
    /// or if this curve has unbounded domain, then an error is returned instead.
    fn samples(&self, samples: usize) -> Result<impl Iterator<Item = T>, ResamplingError> {
        if samples < 2 {
            return Err(ResamplingError::NotEnoughSamples(samples));
        }
        if !self.domain().is_finite() {
            return Err(ResamplingError::InfiniteInterval(InfiniteIntervalError));
        }

        // Unwrap on `spaced_points` always succeeds because its error conditions are handled
        // above.
        Ok(self
            .domain()
            .spaced_points(samples)
            .unwrap()
            .map(|t| self.sample(t)))
    }

    /// Resample this [`Curve`] to produce a new one that is defined by interpolation over samples
    /// taken at a given set of times. The given `interpolation` is used to interpolate adjacent
    /// samples, and the `sample_times` are expected to contain at least two valid times within the
    /// curve's domain interval.
    ///
    /// Redundant sample times, non-finite sample times, and sample times outside of the domain
    /// are simply filtered out. With an insufficient quantity of data, a [`ResamplingError`] is
    /// returned.
    ///
    /// The domain of the produced curve stretches between the first and last sample times of the
    /// iterator.
    ///
    /// The interpolation takes two values by reference together with a scalar parameter and
    /// produces an owned value. The expectation is that `interpolation(&x, &y, 0.0)` and
    /// `interpolation(&x, &y, 1.0)` are equivalent to `x` and `y` respectively.
    fn resample_uneven<I>(
        &self,
        sample_times: impl IntoIterator<Item = f32>,
        interpolation: I,
    ) -> Result<UnevenSampleCurve<T, I>, ResamplingError>
    where
        Self: Sized,
        I: Fn(&T, &T, f32) -> T,
    {
        let mut times: Vec<f32> = sample_times
            .into_iter()
            .filter(|t| t.is_finite() && self.domain().contains(*t))
            .collect();
        times.dedup_by(|t1, t2| (*t1).eq(t2));
        if times.len() < 2 {
            return Err(ResamplingError::NotEnoughSamples(times.len()));
        }
        times.sort_by(|t1, t2| t1.partial_cmp(t2).unwrap());
        let samples = times.iter().copied().map(|t| self.sample(t)).collect();
        Ok(UnevenSampleCurve {
            core: UnevenCore { times, samples },
            interpolation,
        })
    }

    /// Resample this [`Curve`] to produce a new one that is defined by interpolation over samples
    /// taken at the given set of times. The given `sample_times` are expected to contain at least
    /// two valid times within the curve's domain interval.
    ///
    /// Redundant sample times, non-finite sample times, and sample times outside of the domain
    /// are simply filtered out. With an insufficient quantity of data, a [`ResamplingError`] is
    /// returned.
    ///
    /// The domain of the produced [`UnevenSampleAutoCurve`] stretches between the first and last
    /// sample times of the iterator.
    fn resample_uneven_auto(
        &self,
        sample_times: impl IntoIterator<Item = f32>,
    ) -> Result<UnevenSampleAutoCurve<T>, ResamplingError>
    where
        Self: Sized,
        T: StableInterpolate,
    {
        let mut times: Vec<f32> = sample_times
            .into_iter()
            .filter(|t| t.is_finite() && self.domain().contains(*t))
            .collect();
        times.dedup_by(|t1, t2| (*t1).eq(t2));
        if times.len() < 2 {
            return Err(ResamplingError::NotEnoughSamples(times.len()));
        }
        times.sort_by(|t1, t2| t1.partial_cmp(t2).unwrap());
        let samples = times.iter().copied().map(|t| self.sample(t)).collect();
        Ok(UnevenSampleAutoCurve {
            core: UnevenCore { times, samples },
        })
    }

    /// Create a new curve by mapping the values of this curve via a function `f`; i.e., if the
    /// sample at time `t` for this curve is `x`, the value at time `t` on the new curve will be
    /// `f(x)`.
    fn map<S>(self, f: impl Fn(T) -> S) -> impl Curve<S>
    where
        Self: Sized,
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
    /// curve has a parameter interval of `[0, 1]`, then linearly mapping the parameter domain to
    /// `[0, 2]` would be performed as follows, dividing by what might be perceived as the scaling
    /// factor rather than multiplying:
    /// ```
    /// # use bevy_math::curve::*;
    /// let my_curve = constant_curve(interval(0.0, 1.0).unwrap(), 1.0);
    /// let domain = my_curve.domain();
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
    /// let eased_curve = my_curve.reparametrize(domain, |t| easing_curve.sample(t).y);
    /// ```
    fn reparametrize(self, domain: Interval, f: impl Fn(f32) -> f32) -> impl Curve<T>
    where
        Self: Sized,
    {
        ReparamCurve {
            domain,
            base: self,
            f,
            _phantom: PhantomData,
        }
    }

    /// Linearly reparametrize this [`Curve`], producing a new curve whose domain is the given
    /// `domain` instead of the current one. This operation is only valid for curves with finite
    /// domains; if either this curve's domain or the given `domain` is infinite, an
    /// [`InfiniteIntervalError`] is returned.
    fn reparametrize_linear(self, domain: Interval) -> Result<impl Curve<T>, InfiniteIntervalError>
    where
        Self: Sized,
    {
        let f = domain.linear_map_to(self.domain())?;
        Ok(self.reparametrize(domain, f))
    }

    /// Reparametrize this [`Curve`] by sampling from another curve.
    fn reparametrize_by_curve(self, other: &impl Curve<f32>) -> impl Curve<T>
    where
        Self: Sized,
    {
        self.reparametrize(other.domain(), |t| other.sample(t))
    }

    /// Create a new [`Curve`] which is the graph of this one; that is, its output includes the
    /// parameter itself in the samples. For example, if this curve outputs `x` at time `t`, then
    /// the produced curve will produce `(t, x)` at time `t`.
    fn graph(self) -> impl Curve<(f32, T)>
    where
        Self: Sized,
    {
        GraphCurve {
            base: self,
            _phantom: PhantomData,
        }
    }

    /// Create a new [`Curve`] by zipping this curve together with another. The sample at time `t`
    /// in the new curve is `(x, y)`, where `x` is the sample of `self` at time `t` and `y` is the
    /// sample of `other` at time `t`. The domain of the new curve is the intersection of the
    /// domains of its constituents. If the domain intersection would be empty, an
    /// [`InvalidIntervalError`] is returned.
    fn zip<S, C>(self, other: C) -> Result<impl Curve<(T, S)>, InvalidIntervalError>
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
    /// coincides with where this curve ends. A [`CompositionError`] is returned if this curve's domain
    /// doesn't have a finite right endpoint or if `other`'s domain doesn't have a finite left endpoint.
    fn compose<C>(self, other: C) -> Result<impl Curve<T>, CompositionError>
    where
        Self: Sized,
        C: Curve<T>,
    {
        if !self.domain().is_right_finite() {
            return Err(CompositionError::RightInfiniteFirst);
        }
        if !other.domain().is_left_finite() {
            return Err(CompositionError::LeftInfiniteSecond);
        }
        Ok(ComposeCurve {
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
    /// ```
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
}

impl<T, C, D> Curve<T> for D
where
    C: Curve<T> + ?Sized,
    D: Deref<Target = C>,
{
    fn domain(&self) -> Interval {
        <C as Curve<T>>::domain(self)
    }

    fn sample(&self, t: f32) -> T {
        <C as Curve<T>>::sample(self, t)
    }
}

/// An error indicating that a resampling operation could not be performed because of
/// malformed inputs.
#[derive(Debug, Error)]
#[error("Could not resample from this curve because of bad inputs")]
pub enum ResamplingError {
    /// This resampling operation was not provided with enough samples to have well-formed output.
    #[error("Not enough samples to construct resampled curve")]
    NotEnoughSamples(usize),

    /// This resampling operation failed because of an unbounded interval.
    #[error("Could not resample because this curve has unbounded domain")]
    InfiniteInterval(InfiniteIntervalError),
}

/// An error indicating that an end-to-end composition couldn't be performed because of
/// malformed inputs.
#[derive(Debug, Error)]
#[error("Could not compose these curves together")]
pub enum CompositionError {
    /// The right endpoint of the first curve was infinite.
    #[error("The first curve has an infinite right endpoint")]
    RightInfiniteFirst,

    /// The left endpoint of the second curve was infinite.
    #[error("The second curve has an infinite left endpoint")]
    LeftInfiniteSecond,
}

/// A [`Curve`] which takes a constant value over its domain.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct ConstantCurve<T>
where
    T: Clone,
{
    domain: Interval,
    value: T,
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
    fn sample(&self, _t: f32) -> T {
        self.value.clone()
    }
}

/// A [`Curve`] defined by a function.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct FunctionCurve<T, F>
where
    F: Fn(f32) -> T,
{
    domain: Interval,
    f: F,
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
    fn sample(&self, t: f32) -> T {
        (self.f)(t)
    }
}

/// A [`Curve`] that is defined by explicit neighbor interpolation over a set of samples.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct SampleCurve<T, I> {
    core: EvenCore<T>,
    interpolation: I,
}

impl<T, I> Curve<T> for SampleCurve<T, I>
where
    T: Clone,
    I: Fn(&T, &T, f32) -> T,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.core.domain()
    }

    #[inline]
    fn sample(&self, t: f32) -> T {
        self.core.sample_with(t, &self.interpolation)
    }
}

impl<T, I> SampleCurve<T, I> {
    /// Create a new [`SampleCurve`] using the specified `interpolation` to interpolate between
    /// the given `samples`. An error is returned if there are not at least 2 samples or if the
    /// given `domain` is unbounded.
    ///
    /// The interpolation takes two values by reference together with a scalar parameter and
    /// produces an owned value. The expectation is that `interpolation(&x, &y, 0.0)` and
    /// `interpolation(&x, &y, 1.0)` are equivalent to `x` and `y` respectively.
    pub fn new(
        domain: Interval,
        samples: impl Into<Vec<T>>,
        interpolation: I,
    ) -> Result<Self, EvenCoreError>
    where
        I: Fn(&T, &T, f32) -> T,
    {
        Ok(Self {
            core: EvenCore::new(domain, samples)?,
            interpolation,
        })
    }
}

/// A [`Curve`] that is defined by neighbor interpolation over a set of samples.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct SampleAutoCurve<T> {
    core: EvenCore<T>,
}

impl<T> Curve<T> for SampleAutoCurve<T>
where
    T: StableInterpolate,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.core.domain()
    }

    #[inline]
    fn sample(&self, t: f32) -> T {
        self.core
            .sample_with(t, <T as StableInterpolate>::interpolate_stable)
    }
}

impl<T> SampleAutoCurve<T> {
    /// Create a new [`SampleCurve`] using type-inferred interpolation to interpolate between
    /// the given `samples`. An error is returned if there are not at least 2 samples or if the
    /// given `domain` is unbounded.
    pub fn new(domain: Interval, samples: impl Into<Vec<T>>) -> Result<Self, EvenCoreError> {
        Ok(Self {
            core: EvenCore::new(domain, samples)?,
        })
    }
}

/// A [`Curve`] that is defined by interpolation over unevenly spaced samples with explicit
/// interpolation.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct UnevenSampleCurve<T, I> {
    core: UnevenCore<T>,
    interpolation: I,
}

impl<T, I> Curve<T> for UnevenSampleCurve<T, I>
where
    T: Clone,
    I: Fn(&T, &T, f32) -> T,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.core.domain()
    }

    #[inline]
    fn sample(&self, t: f32) -> T {
        self.core.sample_with(t, &self.interpolation)
    }
}

impl<T, I> UnevenSampleCurve<T, I> {
    /// Create a new [`UnevenSampleCurve`] using the provided `interpolation` to interpolate
    /// between adjacent `timed_samples`. The given samples are filtered to finite times and
    /// sorted internally; if there are not at least 2 valid timed samples, an error will be
    /// returned.
    ///
    /// The interpolation takes two values by reference together with a scalar parameter and
    /// produces an owned value. The expectation is that `interpolation(&x, &y, 0.0)` and
    /// `interpolation(&x, &y, 1.0)` are equivalent to `x` and `y` respectively.
    pub fn new(
        timed_samples: impl Into<Vec<(f32, T)>>,
        interpolation: I,
    ) -> Result<Self, UnevenCoreError> {
        Ok(Self {
            core: UnevenCore::new(timed_samples)?,
            interpolation,
        })
    }

    /// This [`UnevenSampleAutoCurve`], but with the sample times moved by the map `f`.
    /// In principle, when `f` is monotone, this is equivalent to [`Curve::reparametrize`],
    /// but the function inputs to each are inverses of one another.
    ///
    /// The samples are re-sorted by time after mapping and deduplicated by output time, so
    /// the function `f` should generally be injective over the sample times of the curve.
    pub fn map_sample_times(self, f: impl Fn(f32) -> f32) -> UnevenSampleCurve<T, I> {
        Self {
            core: self.core.map_sample_times(f),
            interpolation: self.interpolation,
        }
    }
}

/// A [`Curve`] that is defined by interpolation over unevenly spaced samples.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct UnevenSampleAutoCurve<T> {
    core: UnevenCore<T>,
}

impl<T> Curve<T> for UnevenSampleAutoCurve<T>
where
    T: StableInterpolate,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.core.domain()
    }

    #[inline]
    fn sample(&self, t: f32) -> T {
        self.core
            .sample_with(t, <T as StableInterpolate>::interpolate_stable)
    }
}

impl<T> UnevenSampleAutoCurve<T> {
    /// Create a new [`UnevenSampleAutoCurve`] from a given set of timed samples, interpolated
    /// using the  The samples are filtered to finite times and
    /// sorted internally; if there are not at least 2 valid timed samples, an error will be
    /// returned.
    pub fn new(timed_samples: impl Into<Vec<(f32, T)>>) -> Result<Self, UnevenCoreError> {
        Ok(Self {
            core: UnevenCore::new(timed_samples)?,
        })
    }

    /// This [`UnevenSampleAutoCurve`], but with the sample times moved by the map `f`.
    /// In principle, when `f` is monotone, this is equivalent to [`Curve::reparametrize`],
    /// but the function inputs to each are inverses of one another.
    ///
    /// The samples are re-sorted by time after mapping and deduplicated by output time, so
    /// the function `f` should generally be injective over the sample times of the curve.
    pub fn map_sample_times(self, f: impl Fn(f32) -> f32) -> UnevenSampleAutoCurve<T> {
        Self {
            core: self.core.map_sample_times(f),
        }
    }
}

/// A [`Curve`] whose samples are defined by mapping samples from another curve through a
/// given function.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct MapCurve<S, T, C, F>
where
    C: Curve<S>,
    F: Fn(S) -> T,
{
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
    fn sample(&self, t: f32) -> T {
        (self.f)(self.preimage.sample(t))
    }

    #[inline]
    fn map<R>(self, g: impl Fn(T) -> R) -> impl Curve<R>
    where
        Self: Sized,
    {
        let gf = move |x| g((self.f)(x));
        MapCurve {
            preimage: self.preimage,
            f: gf,
            _phantom: PhantomData,
        }
    }

    #[inline]
    fn reparametrize(self, domain: Interval, g: impl Fn(f32) -> f32) -> impl Curve<T>
    where
        Self: Sized,
    {
        MapReparamCurve {
            reparam_domain: domain,
            base: self.preimage,
            forward_map: self.f,
            reparam_map: g,
            _phantom: PhantomData,
        }
    }
}

/// A [`Curve`] whose sample space is mapped onto that of some base curve's before sampling.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct ReparamCurve<T, C, F>
where
    C: Curve<T>,
    F: Fn(f32) -> f32,
{
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
    fn sample(&self, t: f32) -> T {
        self.base.sample((self.f)(t))
    }

    #[inline]
    fn reparametrize(self, domain: Interval, g: impl Fn(f32) -> f32) -> impl Curve<T>
    where
        Self: Sized,
    {
        let fg = move |t| (self.f)(g(t));
        ReparamCurve {
            domain,
            base: self.base,
            f: fg,
            _phantom: PhantomData,
        }
    }

    #[inline]
    fn map<S>(self, g: impl Fn(T) -> S) -> impl Curve<S>
    where
        Self: Sized,
    {
        MapReparamCurve {
            reparam_domain: self.domain,
            base: self.base,
            forward_map: g,
            reparam_map: self.f,
            _phantom: PhantomData,
        }
    }
}

/// A [`Curve`] structure that holds both forward and backward remapping information
/// in order to optimize repeated calls of [`Curve::map`] and [`Curve::reparametrize`].
///
/// Briefly, the point is that the curve just absorbs new functions instead of rebasing
/// itself inside new structs.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct MapReparamCurve<S, T, C, F, G>
where
    C: Curve<S>,
    F: Fn(S) -> T,
    G: Fn(f32) -> f32,
{
    reparam_domain: Interval,
    base: C,
    forward_map: F,
    reparam_map: G,
    _phantom: PhantomData<(S, T)>,
}

impl<S, T, C, F, G> Curve<T> for MapReparamCurve<S, T, C, F, G>
where
    C: Curve<S>,
    F: Fn(S) -> T,
    G: Fn(f32) -> f32,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.reparam_domain
    }

    #[inline]
    fn sample(&self, t: f32) -> T {
        (self.forward_map)(self.base.sample((self.reparam_map)(t)))
    }

    #[inline]
    fn map<R>(self, g: impl Fn(T) -> R) -> impl Curve<R>
    where
        Self: Sized,
    {
        let gf = move |x| g((self.forward_map)(x));
        MapReparamCurve {
            reparam_domain: self.reparam_domain,
            base: self.base,
            forward_map: gf,
            reparam_map: self.reparam_map,
            _phantom: PhantomData,
        }
    }

    #[inline]
    fn reparametrize(self, domain: Interval, g: impl Fn(f32) -> f32) -> impl Curve<T>
    where
        Self: Sized,
    {
        let fg = move |t| (self.reparam_map)(g(t));
        MapReparamCurve {
            reparam_domain: domain,
            base: self.base,
            forward_map: self.forward_map,
            reparam_map: fg,
            _phantom: PhantomData,
        }
    }
}

/// A [`Curve`] that is the graph of another curve over its parameter space.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct GraphCurve<T, C>
where
    C: Curve<T>,
{
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
    fn sample(&self, t: f32) -> (f32, T) {
        (t, self.base.sample(t))
    }
}

/// A [`Curve`] that combines the data from two constituent curves into a tuple output type.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct ProductCurve<S, T, C, D>
where
    C: Curve<S>,
    D: Curve<T>,
{
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
    fn sample(&self, t: f32) -> (S, T) {
        (self.first.sample(t), self.second.sample(t))
    }
}

/// The [`Curve`] that results from composing one curve with another. The second curve is
/// effectively reparametrized so that its start is at the end of the first.
///
/// For this to be well-formed, the first curve's domain must be right-finite and the second's
/// must be left-finite.
pub struct ComposeCurve<T, C, D>
where
    C: Curve<T>,
    D: Curve<T>,
{
    first: C,
    second: D,
    _phantom: PhantomData<T>,
}

impl<T, C, D> Curve<T> for ComposeCurve<T, C, D>
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
    fn sample(&self, t: f32) -> T {
        if t > self.first.domain().end() {
            self.second.sample(
                // `t - first.domain.end` computes the offset into the domain of the second.
                t - self.first.domain().end() + self.second.domain().start(),
            )
        } else {
            self.first.sample(t)
        }
    }
}

/// Create a [`Curve`] that constantly takes the given `value` over the given `domain`.
pub fn constant_curve<T: Clone>(domain: Interval, value: T) -> impl Curve<T> {
    ConstantCurve { domain, value }
}

/// Convert the given function `f` into a [`Curve`] with the given `domain`, sampled by
/// evaluating the function.
pub fn function_curve<T, F>(domain: Interval, f: F) -> impl Curve<T>
where
    F: Fn(f32) -> T,
{
    FunctionCurve { domain, f }
}

/// Flip a curve that outputs tuples so that the tuples are arranged the other way.
pub fn flip<S, T>(curve: impl Curve<(S, T)>) -> impl Curve<(T, S)> {
    curve.map(|(s, t)| (t, s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Quat;
    use approx::{assert_abs_diff_eq, AbsDiffEq};
    use std::f32::consts::TAU;

    #[test]
    fn constant_curves() {
        let curve = constant_curve(everywhere(), 5.0);
        assert!(curve.sample(-35.0) == 5.0);

        let curve = constant_curve(interval(0.0, 1.0).unwrap(), true);
        assert!(curve.sample(2.0));
        assert!(curve.sample_checked(2.0).is_none());
    }

    #[test]
    fn function_curves() {
        let curve = function_curve(everywhere(), |t| t * t);
        assert!(curve.sample(2.0).abs_diff_eq(&4.0, f32::EPSILON));
        assert!(curve.sample(-3.0).abs_diff_eq(&9.0, f32::EPSILON));

        let curve = function_curve(interval(0.0, f32::INFINITY).unwrap(), |t| t.log2());
        assert_eq!(curve.sample(3.5), f32::log2(3.5));
        assert!(curve.sample(-1.0).is_nan());
        assert!(curve.sample_checked(-1.0).is_none());
    }

    #[test]
    fn mapping() {
        let curve = function_curve(everywhere(), |t| t * 3.0 + 1.0);
        let mapped_curve = curve.map(|x| x / 7.0);
        assert_eq!(mapped_curve.sample(3.5), (3.5 * 3.0 + 1.0) / 7.0);
        assert_eq!(mapped_curve.sample(-1.0), (-1.0 * 3.0 + 1.0) / 7.0);
        assert_eq!(mapped_curve.domain(), everywhere());

        let curve = function_curve(interval(0.0, 1.0).unwrap(), |t| t * TAU);
        let mapped_curve = curve.map(Quat::from_rotation_z);
        assert_eq!(mapped_curve.sample(0.0), Quat::IDENTITY);
        assert!(mapped_curve.sample(1.0).is_near_identity());
        assert_eq!(mapped_curve.domain(), interval(0.0, 1.0).unwrap());
    }

    #[test]
    fn reparametrization() {
        let curve = function_curve(interval(1.0, f32::INFINITY).unwrap(), |t| t.log2());
        let reparametrized_curve = curve
            .by_ref()
            .reparametrize(interval(0.0, f32::INFINITY).unwrap(), |t| t.exp2());
        assert_abs_diff_eq!(reparametrized_curve.sample(3.5), 3.5);
        assert_abs_diff_eq!(reparametrized_curve.sample(100.0), 100.0);
        assert_eq!(
            reparametrized_curve.domain(),
            interval(0.0, f32::INFINITY).unwrap()
        );

        let reparametrized_curve = curve
            .by_ref()
            .reparametrize(interval(0.0, 1.0).unwrap(), |t| t + 1.0);
        assert_abs_diff_eq!(reparametrized_curve.sample(0.0), 0.0);
        assert_abs_diff_eq!(reparametrized_curve.sample(1.0), 1.0);
        assert_eq!(reparametrized_curve.domain(), interval(0.0, 1.0).unwrap());
    }

    #[test]
    fn multiple_maps() {
        // Make sure these actually happen in the right order.
        let curve = function_curve(interval(0.0, 1.0).unwrap(), |t| t.exp2());
        let first_mapped = curve.map(|x| x.log2());
        let second_mapped = first_mapped.map(|x| x * -2.0);
        assert_abs_diff_eq!(second_mapped.sample(0.0), 0.0);
        assert_abs_diff_eq!(second_mapped.sample(0.5), -1.0);
        assert_abs_diff_eq!(second_mapped.sample(1.0), -2.0);
    }

    #[test]
    fn multiple_reparams() {
        // Make sure these happen in the right order too.
        let curve = function_curve(interval(0.0, 1.0).unwrap(), |t| t.exp2());
        let first_reparam = curve.reparametrize(interval(1.0, 2.0).unwrap(), |t| t.log2());
        let second_reparam = first_reparam.reparametrize(interval(0.0, 1.0).unwrap(), |t| t + 1.0);
        assert_abs_diff_eq!(second_reparam.sample(0.0), 1.0);
        assert_abs_diff_eq!(second_reparam.sample(0.5), 1.5);
        assert_abs_diff_eq!(second_reparam.sample(1.0), 2.0);
    }

    #[test]
    fn resampling() {
        let curve = function_curve(interval(1.0, 4.0).unwrap(), |t| t.log2());

        // Need at least two points to sample.
        let nice_try = curve.by_ref().resample_auto(1);
        assert!(nice_try.is_err());

        // The values of a resampled curve should be very close at the sample points.
        // Because of denominators, it's not literally equal.
        // (This is a tradeoff against O(1) sampling.)
        let resampled_curve = curve.by_ref().resample_auto(101).unwrap();
        let step = curve.domain().length() / 100.0;
        for index in 0..101 {
            let test_pt = curve.domain().start() + index as f32 * step;
            let expected = curve.sample(test_pt);
            assert_abs_diff_eq!(resampled_curve.sample(test_pt), expected, epsilon = 1e-6);
        }

        // Another example.
        let curve = function_curve(interval(0.0, TAU).unwrap(), |t| t.cos());
        let resampled_curve = curve.by_ref().resample_auto(1001).unwrap();
        let step = curve.domain().length() / 1000.0;
        for index in 0..1001 {
            let test_pt = curve.domain().start() + index as f32 * step;
            let expected = curve.sample(test_pt);
            assert_abs_diff_eq!(resampled_curve.sample(test_pt), expected, epsilon = 1e-6);
        }
    }

    #[test]
    fn uneven_resampling() {
        let curve = function_curve(interval(0.0, f32::INFINITY).unwrap(), |t| t.exp());

        // Need at least two points to resample.
        let nice_try = curve.by_ref().resample_uneven_auto([1.0; 1]);
        assert!(nice_try.is_err());

        // Uneven sampling should produce literal equality at the sample points.
        // (This is part of what you get in exchange for O(log(n)) sampling.)
        let sample_points = (0..100).map(|idx| idx as f32 * 0.1);
        let resampled_curve = curve.by_ref().resample_uneven_auto(sample_points).unwrap();
        for idx in 0..100 {
            let test_pt = idx as f32 * 0.1;
            let expected = curve.sample(test_pt);
            assert_eq!(resampled_curve.sample(test_pt), expected);
        }
        assert_abs_diff_eq!(resampled_curve.domain().start(), 0.0);
        assert_abs_diff_eq!(resampled_curve.domain().end(), 9.9, epsilon = 1e-6);

        // Another example.
        let curve = function_curve(interval(1.0, f32::INFINITY).unwrap(), |t| t.log2());
        let sample_points = (0..10).map(|idx| (idx as f32).exp2());
        let resampled_curve = curve.by_ref().resample_uneven_auto(sample_points).unwrap();
        for idx in 0..10 {
            let test_pt = (idx as f32).exp2();
            let expected = curve.sample(test_pt);
            assert_eq!(resampled_curve.sample(test_pt), expected);
        }
        assert_abs_diff_eq!(resampled_curve.domain().start(), 1.0);
        assert_abs_diff_eq!(resampled_curve.domain().end(), 512.0);
    }
}
