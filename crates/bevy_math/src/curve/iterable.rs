//! Iterable curves, which sample in the form of an iterator in order to support `Vec`-like
//! output whose length cannot be known statically.

use super::Interval;

#[cfg(feature = "alloc")]
use {super::ConstantCurve, alloc::vec::Vec};

/// A curve which provides samples in the form of [`Iterator`]s.
///
/// This is an abstraction that provides an interface for curves which look like `Curve<Vec<T>>`
/// but side-stepping issues with allocation on sampling. This happens when the size of an output
/// array cannot be known statically.
pub trait IterableCurve<T> {
    /// The interval over which this curve is parametrized.
    fn domain(&self) -> Interval;

    /// Sample a point on this curve at the parameter value `t`, producing an iterator over values.
    /// This is the unchecked version of sampling, which should only be used if the sample time `t`
    /// is already known to lie within the curve's domain.
    ///
    /// Values sampled from outside of a curve's domain are generally considered invalid; data which
    /// is nonsensical or otherwise useless may be returned in such a circumstance, and extrapolation
    /// beyond a curve's domain should not be relied upon.
    fn sample_iter_unchecked(&self, t: f32) -> impl Iterator<Item = T>;

    /// Sample this curve at a specified time `t`, producing an iterator over sampled values.
    /// The parameter `t` is clamped to the domain of the curve.
    fn sample_iter_clamped(&self, t: f32) -> impl Iterator<Item = T> {
        let t_clamped = self.domain().clamp(t);
        self.sample_iter_unchecked(t_clamped)
    }

    /// Sample this curve at a specified time `t`, producing an iterator over sampled values.
    /// If the parameter `t` does not lie in the curve's domain, `None` is returned.
    fn sample_iter(&self, t: f32) -> Option<impl Iterator<Item = T>> {
        if self.domain().contains(t) {
            Some(self.sample_iter_unchecked(t))
        } else {
            None
        }
    }
}

#[cfg(feature = "alloc")]
impl<T> IterableCurve<T> for ConstantCurve<Vec<T>>
where
    T: Clone,
{
    fn domain(&self) -> Interval {
        self.domain
    }

    fn sample_iter_unchecked(&self, _t: f32) -> impl Iterator<Item = T> {
        self.value.iter().cloned()
    }
}
