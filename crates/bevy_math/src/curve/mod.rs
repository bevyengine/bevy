//! The [`Curve`] trait, used to describe curves in a number of different domains. This module also
//! contains the [`Interpolable`] trait and the [`Interval`] type.

pub mod interpolable;
pub mod interval;

pub use interpolable::Interpolable;
pub use interval::{everywhere, interval, Interval};

use interval::{InfiniteIntervalError, InvalidIntervalError};
use std::{marker::PhantomData, ops::Deref};
use thiserror::Error;

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
    /// spaced values. A total of `samples` samples are used, although at least two samples are
    /// required in order to produce well-formed output. If fewer than two samples are provided,
    /// or if this curve has an unbounded domain, then a [`ResamplingError`] is returned.
    fn resample(&self, samples: usize) -> Result<SampleCurve<T>, ResamplingError>
    where
        T: Interpolable,
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
            domain: self.domain(),
            samples,
        })
    }

    /// Resample this [`Curve`] to produce a new one that is defined by interpolation over samples
    /// taken at the given set of times. The given `sample_times` are expected to contain at least
    /// two valid times within the curve's domain range.
    ///
    /// Irredundant sample times, non-finite sample times, and sample times outside of the domain
    /// are simply filtered out. With an insufficient quantity of data, a [`ResamplingError`] is
    /// returned.
    ///
    /// The domain of the produced [`UnevenSampleCurve`] stretches between the first and last
    /// sample times of the iterator.
    fn resample_uneven(
        &self,
        sample_times: impl IntoIterator<Item = f32>,
    ) -> Result<UnevenSampleCurve<T>, ResamplingError>
    where
        Self: Sized,
        T: Interpolable,
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
        Ok(UnevenSampleCurve { times, samples })
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

    /// Create a new [`Curve`] by joining this curve together with another. The sample at time `t`
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
    /// let samples = my_curve.by_ref().map(|x| x * 2.0).resample(100).unwrap();
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

/// A [`Curve`] which takes a constant value over its domain.
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

/// A [`Curve`] that is defined by neighbor interpolation over a set of samples.
pub struct SampleCurve<T>
where
    T: Interpolable,
{
    domain: Interval,
    /// The samples that make up this [`SampleCurve`] by interpolation.
    ///
    /// Invariant: this must always have a length of at least 2.
    samples: Vec<T>,
}

impl<T> SampleCurve<T>
where
    T: Interpolable,
{
    /// Like [`Curve::map`], but with a concrete return type. Unlike that function, this one is
    /// not lazy, and `f` is evaluated immediately on samples to produce the result.
    pub fn map_concrete<S>(self, f: impl Fn(T) -> S) -> SampleCurve<S>
    where
        S: Interpolable,
    {
        let new_samples: Vec<S> = self.samples.into_iter().map(f).collect();
        SampleCurve {
            domain: self.domain,
            samples: new_samples,
        }
    }

    /// Like [`Curve::graph`], but with a concrete return type.
    pub fn graph_concrete(self) -> SampleCurve<(f32, T)> {
        let times = self.domain().spaced_points(self.samples.len()).unwrap();
        let new_samples: Vec<(f32, T)> = times.zip(self.samples).collect();
        SampleCurve {
            domain: self.domain,
            samples: new_samples,
        }
    }
}

impl<T> Curve<T> for SampleCurve<T>
where
    T: Interpolable,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.domain
    }

    #[inline]
    fn sample(&self, t: f32) -> T {
        // Inside the curve itself, we interpolate between the two nearest sample values.
        let subdivs = self.samples.len() - 1;
        let step = self.domain.length() / subdivs as f32;
        let t_shifted = t - self.domain.start();
        let steps_taken = t_shifted / step;

        // Using `steps_taken` as the source of truth, clamp to the range of valid indices.
        if steps_taken <= 0.0 {
            self.samples.first().unwrap().clone()
        } else if steps_taken >= (self.samples.len() - 1) as f32 {
            self.samples.last().unwrap().clone()
        } else {
            // Here we use only the floor and the fractional part of `steps_taken` to interpolate
            // between the two nearby sample points.
            let lower_index = steps_taken.floor() as usize;

            // Explicitly clamp the lower index just in case.
            let lower_index = lower_index.min(self.samples.len() - 2);
            let upper_index = lower_index + 1;
            let fract = steps_taken.fract();
            self.samples[lower_index].interpolate(&self.samples[upper_index], fract)
        }
    }
}

/// A [`Curve`] that is defined by interpolation over unevenly spaced samples.
pub struct UnevenSampleCurve<T>
where
    T: Interpolable,
{
    /// The times for the samples of this curve.
    ///
    /// Invariants: This must always have a length of at least 2, be sorted, and have no
    /// duplicated or non-finite times.
    times: Vec<f32>,

    /// The samples corresponding to the times for this curve.
    ///
    /// Invariants: This must always have the same length as `times`.
    samples: Vec<T>,
}

impl<T> UnevenSampleCurve<T>
where
    T: Interpolable,
{
    /// Like [`Curve::map`], but with a concrete return type. Unlike that function, this one is
    /// not lazy, and `f` is evaluated immediately on samples to produce the result.
    pub fn map_concrete<S>(self, f: impl Fn(T) -> S) -> UnevenSampleCurve<S>
    where
        S: Interpolable,
    {
        let new_samples: Vec<S> = self.samples.into_iter().map(f).collect();
        UnevenSampleCurve {
            times: self.times,
            samples: new_samples,
        }
    }

    /// Like [`Curve::graph`], but with a concrete return type.
    pub fn graph_concrete(self) -> UnevenSampleCurve<(f32, T)> {
        let new_samples = self.times.iter().copied().zip(self.samples).collect();
        UnevenSampleCurve {
            times: self.times,
            samples: new_samples,
        }
    }

    /// This [`UnevenSampleCurve`], but with the sample times moved by the map `f`.
    /// In principle, when `f` is monotone, this is equivalent to [`Curve::reparametrize`],
    /// but the function inputs to each are inverses of one another.
    ///
    /// The samples are resorted by time after mapping and deduplicated by output time, so
    /// the function `f` should generally be injective over the sample times of the curve.
    pub fn map_sample_times(mut self, f: impl Fn(f32) -> f32) -> UnevenSampleCurve<T> {
        let mut timed_samples: Vec<(f32, T)> =
            self.times.into_iter().map(f).zip(self.samples).collect();
        timed_samples.dedup_by(|(t1, _), (t2, _)| (*t1).eq(t2));
        timed_samples.sort_by(|(t1, _), (t2, _)| t1.partial_cmp(t2).unwrap());
        self.times = timed_samples.iter().map(|(t, _)| t).copied().collect();
        self.samples = timed_samples.into_iter().map(|(_, x)| x).collect();
        self
    }
}

impl<T> Curve<T> for UnevenSampleCurve<T>
where
    T: Interpolable,
{
    #[inline]
    fn domain(&self) -> Interval {
        let start = self.times.first().unwrap();
        let end = self.times.last().unwrap();
        Interval::new(*start, *end).unwrap()
    }

    #[inline]
    fn sample(&self, t: f32) -> T {
        match self
            .times
            .binary_search_by(|pt| pt.partial_cmp(&t).unwrap())
        {
            Ok(index) => self.samples[index].clone(),
            Err(index) => {
                if index == 0 {
                    self.samples.first().unwrap().clone()
                } else if index == self.times.len() {
                    self.samples.last().unwrap().clone()
                } else {
                    let t_lower = self.times[index - 1];
                    let v_lower = self.samples.get(index - 1).unwrap();
                    let t_upper = self.times[index];
                    let v_upper = self.samples.get(index).unwrap();
                    let s = (t - t_lower) / (t_upper - t_lower);
                    v_lower.interpolate(v_upper, s)
                }
            }
        }
    }
}

/// A [`Curve`] whose samples are defined by mapping samples from another curve through a
/// given function.
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
        let nice_try = curve.by_ref().resample(1);
        assert!(nice_try.is_err());

        // The values of a resampled curve should be very close at the sample points.
        // Because of denominators, it's not literally equal.
        // (This is a tradeoff against O(1) sampling.)
        let resampled_curve = curve.by_ref().resample(101).unwrap();
        let step = curve.domain().length() / 100.0;
        for index in 0..101 {
            let test_pt = curve.domain().start() + index as f32 * step;
            let expected = curve.sample(test_pt);
            assert_abs_diff_eq!(resampled_curve.sample(test_pt), expected, epsilon = 1e-6);
        }

        // Another example.
        let curve = function_curve(interval(0.0, TAU).unwrap(), |t| t.cos());
        let resampled_curve = curve.by_ref().resample(1001).unwrap();
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
        let nice_try = curve.by_ref().resample_uneven([1.0; 1]);
        assert!(nice_try.is_err());

        // Uneven sampling should produce literal equality at the sample points.
        // (This is part of what you get in exchange for O(log(n)) sampling.)
        let sample_points = (0..100).map(|idx| idx as f32 * 0.1);
        let resampled_curve = curve.by_ref().resample_uneven(sample_points).unwrap();
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
        let resampled_curve = curve.by_ref().resample_uneven(sample_points).unwrap();
        for idx in 0..10 {
            let test_pt = (idx as f32).exp2();
            let expected = curve.sample(test_pt);
            assert_eq!(resampled_curve.sample(test_pt), expected);
        }
        assert_abs_diff_eq!(resampled_curve.domain().start(), 1.0);
        assert_abs_diff_eq!(resampled_curve.domain().end(), 512.0);
    }
}

// Haha... you thought the file was over!

/// A curve which provides samples in the form of [`Iterator`]s.
///
/// This is an abstraction that provides an interface for curves which look like `Curve<Vec<T>>`
/// but side-stepping issues with allocation on sampling. This happens when the size of an output
/// array cannot be known statically.
pub trait IterableCurve<T> {
    /// The interval over which this curve is parametrized.
    fn domain(&self) -> Interval;

    /// Sample this curve at a specified time `t`, producing an iterator over sampled values.
    fn sample_iter<'a>(&self, t: f32) -> impl Iterator<Item = T>
    where
        Self: 'a;
}

impl<T> IterableCurve<T> for ConstantCurve<Vec<T>>
where
    T: Clone,
{
    fn domain(&self) -> Interval {
        self.domain
    }

    fn sample_iter<'a>(&self, _t: f32) -> impl Iterator<Item = T>
    where
        Self: 'a,
    {
        self.value.iter().cloned()
    }
}
