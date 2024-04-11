//! Houses the [`Curve`] trait together with the [`Interpolable`] trait that it depends on.

use std::{cmp::max, marker::PhantomData};
use crate::Quat;
// use serde::{de::DeserializeOwned, Serialize};

use crate::VectorSpace;

/// A trait for types whose values can be intermediately interpolated between two given values
/// with an auxiliary parameter.
pub trait Interpolable: Clone {
    /// Interpolate between this value and the `other` given value using the parameter `t`.
    /// Note that the parameter `t` is not necessarily clamped to lie between `0` and `1`.
    fn interpolate(&self, other: &Self, t: f32) -> Self;
}

impl<S, T> Interpolable for (S, T)
where
    S: Interpolable,
    T: Interpolable,
{
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        (
            self.0.interpolate(&other.0, t),
            self.1.interpolate(&other.1, t),
        )
    }
}

impl<T> Interpolable for T
where
    T: VectorSpace,
{
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        self.lerp(*other, t)
    }
}

impl Interpolable for Quat {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        self.slerp(*other, t)
    }
}


/// A trait for a type that can represent values of type `T` parametrized over a fixed interval.
/// Typical examples of this are actual geometric curves where `T: VectorSpace`, but other kinds
/// of interpolable data can be represented instead (or in addition).
pub trait Curve<T>
where
    T: Interpolable,
{
    /// The point at which parameter values of this curve end. That is, this curve is parametrized
    /// on the interval `[0, self.duration()]`.
    fn duration(&self) -> f32;

    /// Sample a point on this curve at the parameter value `t`, extracting the associated value.
    fn sample(&self, t: f32) -> T;

    /// Resample this [`Curve`] to produce a new one that is defined by interpolation over equally
    /// spaced values. A total of `samples` samples are used.
    ///
    /// Panics if `samples == 0`.
    fn resample(&self, samples: usize) -> SampleCurve<T> {
        assert!(samples != 0);

        // When `samples` is 1, we just record the starting point, and `step` doesn't matter.
        let subdivisions = max(1, samples - 1);
        let step = self.duration() / subdivisions as f32;
        let samples: Vec<T> = (0..samples).map(|s| self.sample(s as f32 * step)).collect();
        SampleCurve {
            duration: self.duration(),
            samples,
        }
    }

    /// Resample this [`Curve`] to produce a new one that is defined by interpolation over samples
    /// taken at the given set of times. The given `sample_times` are expected to be strictly
    /// increasing and nonempty.
    fn resample_uneven(&self, sample_times: impl IntoIterator<Item = f32>) -> UnevenSampleCurve<T> {
        let mut iter = sample_times.into_iter();
        let Some(first) = iter.next() else {
            panic!("Empty iterator supplied")
        };
        // Offset by the first element so that we get a curve starting at zero.
        let first_sample = self.sample(first);
        let mut timed_samples = vec![(0.0, first_sample)];
        timed_samples.extend(iter.map(|t| (t - first, self.sample(t))));
        UnevenSampleCurve { timed_samples }
    }

    /// Create a new curve by mapping the values of this curve via a function `f`; i.e., if the
    /// sample at time `t` for this curve is `x`, the value at time `t` on the new curve will be
    /// `f(x)`.
    fn map<S>(self, f: impl Fn(T) -> S) -> impl Curve<S>
    where
        Self: Sized,
        S: Interpolable,
    {
        MapCurve {
            preimage: self,
            f,
            _phantom: PhantomData,
        }
    }

    /// Create a new [`Curve`] whose parameter space is related to the parameter space of this curve
    /// by `f`. For each time `t`, the sample from the new curve at time `t` is the sample from
    /// this curve at time `f(t)`. The given `duration` will be the duration of the new curve. The
    /// function `f` is expected to take `[0, duration]` into `[0, self.duration]`.
    ///
    /// Note that this is the opposite of what one might expect intuitively; for example, if this
    /// curve has a parameter interval of `[0, 1]`, then linearly mapping the parameter domain to
    /// `[0, 2]` would be performed as follows, dividing by what might be perceived as the scaling
    /// factor rather than multiplying:
    /// ```
    /// # use bevy_math::curve::*;
    /// # let my_curve = constant_curve(1.0, 1.0);
    /// let dur = my_curve.duration();
    /// let scaled_curve = my_curve.reparametrize(dur * 2.0, |t| t / 2.0);
    /// ```
    /// This kind of linear remapping is provided by the convenience method
    /// [`Curve::reparametrize_linear`], which requires only the desired duration for the new curve.
    ///
    /// # Examples
    /// ```
    /// // Reverse a curve:
    /// # use bevy_math::curve::*;
    /// # use bevy_math::vec2;
    /// # let my_curve = constant_curve(1.0, 1.0);
    /// let dur = my_curve.duration();
    /// let reversed_curve = my_curve.reparametrize(dur, |t| dur - t);
    ///
    /// // Take a segment of a curve:
    /// # let my_curve = constant_curve(1.0, 1.0);
    /// let curve_segment = my_curve.reparametrize(0.5, |t| 0.5 + t);
    ///
    /// // Reparametrize by an easing curve:
    /// # let my_curve = constant_curve(1.0, 1.0);
    /// # let easing_curve = constant_curve(1.0, vec2(1.0, 1.0));
    /// let dur = my_curve.duration();
    /// let eased_curve = my_curve.reparametrize(dur, |t| easing_curve.sample(t).y);
    /// ```
    ///
    /// # Panics
    /// Panics if `duration` is not greater than `0.0`.
    fn reparametrize(self, duration: f32, f: impl Fn(f32) -> f32) -> impl Curve<T>
    where
        Self: Sized,
    {
        assert!(duration > 0.0);
        ReparamCurve {
            duration,
            base: self,
            f,
            _phantom: PhantomData,
        }
    }

    /// Linearly reparametrize this [`Curve`], producing a new curve whose duration is the given
    /// `duration` instead of the current one.
    fn reparametrize_linear(self, duration: f32) -> impl Curve<T>
    where
        Self: Sized,
    {
        assert!(duration > 0.0);
        let old_duration = self.duration();
        Curve::reparametrize(self, duration, move |t| t * (old_duration / duration))
    }

    /// Reparametrize this [`Curve`] by sampling from another curve.
    fn reparametrize_by_curve(self, other: &impl Curve<f32>) -> impl Curve<T>
    where
        Self: Sized,
    {
        self.reparametrize(other.duration(), |t| other.sample(t))
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
    /// sample of `other` at time `t`. The duration of the new curve is the smaller of the two
    /// between `self` and `other`.
    fn and<S, C>(self, other: C) -> impl Curve<(T, S)>
    where
        Self: Sized,
        S: Interpolable,
        C: Curve<S> + Sized,
    {
        ProductCurve {
            first: self,
            second: other,
            _phantom: PhantomData,
        }
    }
}

/// A [`Curve`] which takes a constant value over its duration.
pub struct ConstantCurve<T>
where
    T: Interpolable,
{
    duration: f32,
    value: T,
}

impl<T> Curve<T> for ConstantCurve<T>
where
    T: Interpolable,
{
    #[inline]
    fn duration(&self) -> f32 {
        self.duration
    }

    #[inline]
    fn sample(&self, _t: f32) -> T {
        self.value.clone()
    }
}

/// A [`Curve`] defined by a function.
pub struct FunctionCurve<T, F> 
where
    T: Interpolable,
    F: Fn(f32) -> T,
{
    duration: f32,
    f: F,
}

impl<T, F> Curve<T> for FunctionCurve<T, F> 
where
    T: Interpolable,
    F: Fn(f32) -> T,
{
    #[inline]
    fn duration(&self) -> f32 {
        self.duration
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
    duration: f32,

    /// The list of samples that define this curve by interpolation.
    pub samples: Vec<T>,
}

impl<T> SampleCurve<T>
where
    T: Interpolable,
{
    /// Like [`Curve::map`], but with a concrete return type.
    pub fn map_concrete<S>(self, f: impl Fn(T) -> S) -> SampleCurve<S>
    where
        S: Interpolable,
    {
        let new_samples: Vec<S> = self.samples.into_iter().map(f).collect();
        SampleCurve {
            duration: self.duration,
            samples: new_samples,
        }
    }

    /// Like [`Curve::graph`], but with a concrete return type.
    pub fn graph_concrete(self) -> SampleCurve<(f32, T)> {
        let subdivisions = max(1, self.samples.len() - 1);
        let step = self.duration() / subdivisions as f32;
        let times: Vec<f32> = (0..self.samples.len()).map(|s| s as f32 * step).collect();
        let new_samples: Vec<(f32, T)> = times.into_iter().zip(self.samples).collect();
        SampleCurve {
            duration: self.duration,
            samples: new_samples,
        }
    }
}

impl<T> Curve<T> for SampleCurve<T>
where
    T: Interpolable,
{
    #[inline]
    fn duration(&self) -> f32 {
        self.duration
    }

    #[inline]
    fn sample(&self, t: f32) -> T {
        let num_samples = self.samples.len();
        // If there is only one sample, then we return the single sample point. We also clamp `t`
        // to `[0, self.duration]` here.
        if num_samples == 1 || t <= 0.0 {
            return self.samples[0].clone();
        }
        if t >= self.duration {
            return self.samples[self.samples.len() - 1].clone();
        }

        // Inside the curve itself, interpolate between the two nearest sample values.
        let subdivs = num_samples - 1;
        let step = self.duration / subdivs as f32;
        let lower_index = (t / step).floor() as usize;
        let upper_index = (t / step).ceil() as usize;
        let f = (t / step).fract();
        self.samples[lower_index].interpolate(&self.samples[upper_index], f)
    }

    fn map<S>(self, f: impl Fn(T) -> S) -> impl Curve<S>
    where
        Self: Sized,
        S: Interpolable,
    {
        self.map_concrete(f)
    }

    fn graph(self) -> impl Curve<(f32, T)>
    where
        Self: Sized,
    {
        self.graph_concrete()
    }
}

/// A [`Curve`] that is defined by interpolation over unevenly spaced samples.
pub struct UnevenSampleCurve<T>
where
    T: Interpolable,
{
    timed_samples: Vec<(f32, T)>,
}

impl<T> UnevenSampleCurve<T>
where
    T: Interpolable,
{
    /// Like [`Curve::map`], but with a concrete return type..
    pub fn map_concrete<S>(self, f: impl Fn(T) -> S) -> UnevenSampleCurve<S>
    where
        S: Interpolable,
    {
        let new_samples: Vec<(f32, S)> = self
            .timed_samples
            .into_iter()
            .map(|(t, x)| (t, f(x)))
            .collect();
        UnevenSampleCurve {
            timed_samples: new_samples,
        }
    }

    /// Like [`Curve::graph`], but with a concrete return type.
    pub fn graph_concrete(self) -> UnevenSampleCurve<(f32, T)> {
        let new_samples: Vec<(f32, (f32, T))> = self
            .timed_samples
            .into_iter()
            .map(|(t, x)| (t, (t, x)))
            .collect();
        UnevenSampleCurve {
            timed_samples: new_samples,
        }
    }
}

impl<T> Curve<T> for UnevenSampleCurve<T>
where
    T: Interpolable,
{
    #[inline]
    fn duration(&self) -> f32 {
        self.timed_samples.last().unwrap().0
    }

    #[inline]
    fn sample(&self, t: f32) -> T {
        match self
            .timed_samples
            .binary_search_by(|(pt, _)| pt.partial_cmp(&t).unwrap())
        {
            Ok(index) => self.timed_samples[index].1.clone(),
            Err(index) => {
                if index == 0 {
                    self.timed_samples.first().unwrap().1.clone()
                } else if index == self.timed_samples.len() {
                    self.timed_samples.last().unwrap().1.clone()
                } else {
                    let (t_lower, v_lower) = self.timed_samples.get(index - 1).unwrap();
                    let (t_upper, v_upper) = self.timed_samples.get(index).unwrap();
                    let s = (t - t_lower) / (t_upper - t_lower);
                    v_lower.interpolate(v_upper, s)
                }
            }
        }
    }

    fn map<S>(self, f: impl Fn(T) -> S) -> impl Curve<S>
    where
        Self: Sized,
        S: Interpolable,
    {
        self.map_concrete(f)
    }

    fn graph(self) -> impl Curve<(f32, T)>
    where
        Self: Sized,
    {
        self.graph_concrete()
    }
}

/// A [`Curve`] whose samples are defined by mapping samples from another curve through a
/// given function.
pub struct MapCurve<S, T, C, F>
where
    S: Interpolable,
    T: Interpolable,
    C: Curve<S>,
    F: Fn(S) -> T,
{
    preimage: C,
    f: F,
    _phantom: PhantomData<(S, T)>,
}

impl<S, T, C, F> Curve<T> for MapCurve<S, T, C, F>
where
    S: Interpolable,
    T: Interpolable,
    C: Curve<S>,
    F: Fn(S) -> T,
{
    #[inline]
    fn duration(&self) -> f32 {
        self.preimage.duration()
    }

    #[inline]
    fn sample(&self, t: f32) -> T {
        (self.f)(self.preimage.sample(t))
    }
}

/// A [`Curve`] whose sample space is mapped onto that of some base curve's before sampling.
pub struct ReparamCurve<T, C, F>
where
    T: Interpolable,
    C: Curve<T>,
    F: Fn(f32) -> f32,
{
    duration: f32,
    base: C,
    f: F,
    _phantom: PhantomData<T>,
}

impl<T, C, F> Curve<T> for ReparamCurve<T, C, F>
where
    T: Interpolable,
    C: Curve<T>,
    F: Fn(f32) -> f32,
{
    #[inline]
    fn duration(&self) -> f32 {
        self.duration
    }

    #[inline]
    fn sample(&self, t: f32) -> T {
        self.base.sample((self.f)(t))
    }
}

/// A [`Curve`] that is the graph of another curve over its parameter space.
pub struct GraphCurve<T, C>
where
    T: Interpolable,
    C: Curve<T>,
{
    base: C,
    _phantom: PhantomData<T>,
}

impl<T, C> Curve<(f32, T)> for GraphCurve<T, C>
where
    T: Interpolable,
    C: Curve<T>,
{
    #[inline]
    fn duration(&self) -> f32 {
        self.base.duration()
    }

    #[inline]
    fn sample(&self, t: f32) -> (f32, T) {
        (t, self.base.sample(t))
    }
}

/// A [`Curve`] that combines the data from two constituent curves into a tuple output type.
pub struct ProductCurve<S, T, C, D>
where
    S: Interpolable,
    T: Interpolable,
    C: Curve<S>,
    D: Curve<T>,
{
    first: C,
    second: D,
    _phantom: PhantomData<(S, T)>,
}

impl<S, T, C, D> Curve<(S, T)> for ProductCurve<S, T, C, D>
where
    S: Interpolable,
    T: Interpolable,
    C: Curve<S>,
    D: Curve<T>,
{
    #[inline]
    fn duration(&self) -> f32 {
        f32::min(self.first.duration(), self.second.duration())
    }

    #[inline]
    fn sample(&self, t: f32) -> (S, T) {
        (self.first.sample(t), self.second.sample(t))
    }
}

// Experimental stuff:

// TODO: See how much this needs to be extended / whether it's actually useful.
// The actual point here is to give access to additional trait constraints that are
// satisfied by the output, but not guaranteed depending on the actual data
// that underpins the invoking implementation.

// pub trait MapConcreteCurve<T>: Curve<T> + Serialize + DeserializeOwned
// where T: Interpolable {
//     fn map_concrete<S>(self, f: impl Fn(T) -> S) -> impl MapConcreteCurve<S>
//     where S: Interpolable;
// }

// Library functions:

/// Create a [`Curve`] that constantly takes the given `value` over the given `duration`.
pub fn constant_curve<T: Interpolable>(duration: f32, value: T) -> impl Curve<T> {
    ConstantCurve { duration, value }
}

/// Convert the given function `f` into a [`Curve`] with the given `duration`, sampled by
/// evaluating the function.
pub fn function_curve<T, F>(duration: f32, f: F) -> impl Curve<T>
where 
    T: Interpolable,
    F: Fn(f32) -> T,
{
    FunctionCurve { duration, f }
}

/// Flip a curve that outputs tuples so that the tuples are arranged the other way.
pub fn flip<S, T>(curve: impl Curve<(S, T)>) -> impl Curve<(T, S)>
where
    S: Interpolable,
    T: Interpolable,
{
    curve.map(|(s, t)| (t, s))
}

/// An error indicating that the implicit function theorem algorithm failed to apply because
/// the input curve did not meet its criteria.
pub struct IftError;

/// Given a monotone `curve`, produces the curve that it is the graph of, up to reparametrization.
/// This is an algorithmic manifestation of the implicit function theorem; it is a numerical
/// procedure which is only performed to the specified resolutions.
///
/// The `search_resolution` dictates how many samples are taken of the input curve; linear
/// interpolation is used between these samples to estimate the inverse image.
///
/// The `outgoing_resolution` dictates the number of samples that are used in the construction of
/// the output itself.
///
/// The input curve must have its first x-value be `0` or an error will be returned. Furthermore,
/// if the curve is non-monotone, the output of this function may be nonsensical even if an error
/// does not occur.
pub fn ift<T>(
    curve: &impl Curve<(f32, T)>,
    search_resolution: usize,
    outgoing_resolution: usize,
) -> Result<SampleCurve<T>, IftError>
where
    T: Interpolable,
{
    // The duration of the output curve is the maximum x-value of the input curve.
    let (duration, _) = curve.sample(curve.duration());
    let discrete_curve = curve.resample(search_resolution);

    let subdivisions = max(1, outgoing_resolution - 1);
    let step = duration / subdivisions as f32;
    let times: Vec<f32> = (0..outgoing_resolution).map(|s| s as f32 * step).collect();

    let mut values: Vec<T> = vec![];
    for t in times {
        // Find a value on the curve where the x-value is close to `t`.
        match discrete_curve
            .samples
            .binary_search_by(|(x, _y)| x.partial_cmp(&t).unwrap())
        {
            // We found an exact match in our samples (pretty unlikely).
            Ok(index) => {
                let y = discrete_curve.samples[index].1.clone();
                values.push(y);
            }

            // We did not find an exact match, so we must interpolate.
            Err(index) => {
                // The value should be between `index - 1` and `index`.
                // If `index` is the sample length or 0, then something went wrong; `t` is outside
                // of the range of the function projection.
                if index == 0 || index == search_resolution {
                    return Err(IftError);
                } else {
                    let (t_lower, y_lower) = discrete_curve.samples.get(index - 1).unwrap();
                    let (t_upper, y_upper) = discrete_curve.samples.get(index).unwrap();
                    if t_lower >= t_upper {
                        return Err(IftError);
                    }
                    // Inverse lerp on projected values to interpolate the y-value.
                    let s = (t - t_lower) / (t_upper - t_lower);
                    let value = y_lower.interpolate(y_upper, s);
                    values.push(value);
                }
            }
        }
    }
    Ok(SampleCurve {
        duration,
        samples: values,
    })
}
