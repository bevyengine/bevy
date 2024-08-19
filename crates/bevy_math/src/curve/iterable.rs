//! Iterable curves, which sample in the form of an iterator in order to support `Vec`-like
//! output whose length cannot be known statically.

use super::{ConstantCurve, Interval};

/// A curve which provides samples in the form of [`Iterator`]s.
///
/// This is an abstraction that provides an interface for curves which look like `Curve<Vec<T>>`
/// but side-stepping issues with allocation on sampling. This happens when the size of an output
/// array cannot be known statically.
pub trait IterableCurve<T> {
    /// The interval over which this curve is parametrized.
    fn domain(&self) -> Interval;

    /// Sample this curve at a specified time `t`, producing an iterator over sampled values.
    fn sample_iter_unchecked<'a>(&self, t: f32) -> impl Iterator<Item = T>
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

    fn sample_iter_unchecked<'a>(&self, _t: f32) -> impl Iterator<Item = T>
    where
        Self: 'a,
    {
        self.value.iter().cloned()
    }
}
