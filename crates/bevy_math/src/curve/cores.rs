//! Core data structures to be used internally in Curve implementations, encapsulating storage
//! and access patterns for reuse.

use super::interval::Interval;
use thiserror::Error;

/// This type expresses the relationship of a value to a linear collection of values. It is a kind
/// of summary used intermediately by sampling operations.
pub enum Betweenness<T> {
    /// This value lies exactly on another.
    Exact(T),

    /// This value is off the left tail of the family; the inner value is the family's leftmost.
    LeftTail(T),

    /// This value is off the right tail of the family; the inner value is the family's rightmost.
    RightTail(T),

    /// This value lies on the interior, in between two points, with a third parameter expressing
    /// the interpolation factor between the two.
    Between(T, T, f32),
}

impl<T> Betweenness<T> {
    /// Map all values using a given function `f`, leaving the interpolation parameters in any
    /// [`Between`] variants unchanged.
    ///
    /// [`Between`]: `Betweenness::Between`
    #[must_use]
    pub fn map<S>(self, f: impl Fn(T) -> S) -> Betweenness<S> {
        match self {
            Betweenness::Exact(v) => Betweenness::Exact(f(v)),
            Betweenness::LeftTail(v) => Betweenness::LeftTail(f(v)),
            Betweenness::RightTail(v) => Betweenness::RightTail(f(v)),
            Betweenness::Between(u, v, s) => Betweenness::Between(f(u), f(v), s),
        }
    }
}

/// The data core of a curve derived from evenly-spaced samples. The intention is to use this
/// in addition to explicit or inferred interpolation information in user-space in order to
/// implement curves using [`domain`] and [`sample_with`]
///
/// The internals are made transparent to give curve authors freedom, but [the provided constructor]
/// enforces the required invariants.
///
/// [the provided constructor]: EvenCore::new
/// [`domain`]: EvenCore::domain
/// [`sample_with`]: EvenCore::sample_with
///
/// # Example
/// ```rust
/// # use bevy_math::curve::*;
/// # use bevy_math::curve::builders::*;
/// enum InterpolationMode {
///     Linear,
///     Step,
/// }
///
/// trait LinearInterpolate {
///     fn lerp(&self, other: &Self, t: f32) -> Self;
/// }
///
/// fn step<T: Clone>(first: &T, second: &T, t: f32) -> T {
///     if t >= 1.0 {
///         second.clone()
///     } else {
///         first.clone()
///     }
/// }
///
/// struct MyCurve<T> {
///     core: EvenCore<T>,
///     interpolation_mode: InterpolationMode,
/// }
///
/// impl<T> Curve<T> for MyCurve<T>
/// where
///     T: LinearInterpolate + Clone,
/// {
///     fn domain(&self) -> Interval {
///         self.core.domain()
///     }
///     
///     fn sample(&self, t: f32) -> T {
///         match self.interpolation_mode {
///             InterpolationMode::Linear => self.core.sample_with(t, <T as LinearInterpolate>::lerp),
///             InterpolationMode::Step => self.core.sample_with(t, step),
///         }
///     }
/// }
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct EvenCore<T> {
    /// The domain over which the samples are taken, which corresponds to the domain of the curve
    /// formed by interpolating them.
    ///
    /// # Invariants
    /// This must always be a bounded interval; i.e. its endpoints must be finite.
    pub domain: Interval,

    /// The samples that are interpolated to extract values.
    ///
    /// # Invariants
    /// This must always have a length of at least 2.
    pub samples: Vec<T>,
}

/// An error indicating that a [`EvenCore`] could not be constructed.
#[derive(Debug, Error)]
pub enum EvenCoreError {
    /// Not enough samples were provided.
    #[error("Need at least two samples to create a EvenCore, but {samples} were provided")]
    NotEnoughSamples {
        /// The number of samples that were provided.
        samples: usize,
    },

    /// Unbounded domains are not compatible with `EvenCore`.
    #[error("Cannot create a EvenCore over a domain with an infinite endpoint")]
    InfiniteDomain,
}

impl<T> EvenCore<T> {
    /// Create a new [`EvenCore`] from the specified `domain` and `samples`. An error is returned
    /// if there are not at least 2 samples or if the given domain is unbounded.
    #[inline]
    pub fn new(domain: Interval, samples: impl Into<Vec<T>>) -> Result<Self, EvenCoreError> {
        let samples: Vec<T> = samples.into();
        if samples.len() < 2 {
            return Err(EvenCoreError::NotEnoughSamples {
                samples: samples.len(),
            });
        }
        if !domain.is_finite() {
            return Err(EvenCoreError::InfiniteDomain);
        }

        Ok(EvenCore { domain, samples })
    }

    /// The domain of the curve derived from this core.
    #[inline]
    pub fn domain(&self) -> Interval {
        self.domain
    }

    /// Obtain a value from the held samples using the given `interpolation` to interpolate
    /// between adjacent samples.
    ///
    /// The interpolation takes two values by reference together with a scalar parameter and
    /// produces an owned value. The expectation is that `interpolation(&x, &y, 0.0)` and
    /// `interpolation(&x, &y, 1.0)` are equivalent to `x` and `y` respectively.
    #[inline]
    pub fn sample_with<I>(&self, t: f32, interpolation: I) -> T
    where
        T: Clone,
        I: Fn(&T, &T, f32) -> T,
    {
        match even_betweenness(self.domain, self.samples.len(), t) {
            Betweenness::Exact(idx) | Betweenness::LeftTail(idx) | Betweenness::RightTail(idx) => {
                self.samples[idx].clone()
            }
            Betweenness::Between(lower_idx, upper_idx, s) => {
                interpolation(&self.samples[lower_idx], &self.samples[upper_idx], s)
            }
        }
    }

    /// Given a time `t`, obtain a [`Betweenness`] which governs how interpolation might recover
    /// a sample at time `t`. For example, when a [`Between`] value is returned, its contents can
    /// be used to interpolate between the two contained values with the given parameter. The other
    /// variants give additional context about where the value is relative to the family of samples.
    ///
    /// [`Between`]: `Betweenness::Between`
    pub fn sample_betweenness(&self, t: f32) -> Betweenness<&T> {
        even_betweenness(self.domain, self.samples.len(), t).map(|idx| &self.samples[idx])
    }
}

/// Given a domain and a number of samples taken over that interval, return a [`Betweenness`]
/// that governs how samples are extracted relative to the stored data.
///
/// `domain` must be a bounded interval (i.e. `domain.is_finite() == true`).
///
/// `samples` must be at least 2.
///
/// This function will never panic, but it may return invalid indices if its assumptions are violated.
pub fn even_betweenness(domain: Interval, samples: usize, t: f32) -> Betweenness<usize> {
    let subdivs = samples - 1;
    let step = domain.length() / subdivs as f32;
    let t_shifted = t - domain.start();
    let steps_taken = t_shifted / step;

    if steps_taken <= 0.0 {
        // To the left side of all the samples.
        Betweenness::LeftTail(0)
    } else if steps_taken >= subdivs as f32 {
        // To the right side of all the samples
        Betweenness::RightTail(samples - 1)
    } else {
        let lower_index = steps_taken.floor() as usize;
        // This upper index is always valid because `steps_taken` is a finite value
        // strictly less than `samples - 1`, so its floor is at most `samples - 2`
        let upper_index = lower_index + 1;
        let s = steps_taken.fract();
        Betweenness::Between(lower_index, upper_index, s)
    }
}

/// The data core of a curve defined by unevenly-spaced samples or keyframes. The intention is to
/// use this in concert with implicitly or explicitly-defined interpolation in user-space in
/// order to implement the curve interface using [`domain`] and [`sample_with`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct UnevenCore<T> {
    /// The times for the samples of this curve.
    ///
    /// # Invariants
    /// This must always have a length of at least 2, be sorted, and have no
    /// duplicated or non-finite times.
    pub times: Vec<f32>,

    /// The samples corresponding to the times for this curve.
    ///
    /// # Invariants
    /// This must always have the same length as `times`.
    pub samples: Vec<T>,
}

/// An error indicating that an [`UnevenCore`] could not be constructed.
#[derive(Debug, Error)]
pub enum UnevenCoreError {
    /// Not enough samples were provided.
    #[error("Need at least two samples to create an UnevenCore, but {samples} were provided")]
    NotEnoughSamples {
        /// The number of samples that were provided.
        samples: usize,
    },
}

impl<T> UnevenCore<T> {
    /// Create a new [`UnevenCore`]. The given samples are filtered to finite times and
    /// sorted internally; if there are not at least 2 valid timed samples, an error will be
    /// returned.
    ///
    /// The interpolation takes two values by reference together with a scalar parameter and
    /// produces an owned value. The expectation is that `interpolation(&x, &y, 0.0)` and
    /// `interpolation(&x, &y, 1.0)` are equivalent to `x` and `y` respectively.
    pub fn new(timed_samples: impl Into<Vec<(f32, T)>>) -> Result<Self, UnevenCoreError> {
        let timed_samples: Vec<(f32, T)> = timed_samples.into();

        // Filter out non-finite sample times first so they don't interfere with sorting/deduplication.
        let mut timed_samples: Vec<(f32, T)> = timed_samples
            .into_iter()
            .filter(|(t, _)| t.is_finite())
            .collect();
        timed_samples
            .sort_by(|(t0, _), (t1, _)| t0.partial_cmp(t1).unwrap_or(std::cmp::Ordering::Equal));
        timed_samples.dedup_by_key(|(t, _)| *t);

        let (times, samples): (Vec<f32>, Vec<T>) = timed_samples.into_iter().unzip();

        if times.len() < 2 {
            return Err(UnevenCoreError::NotEnoughSamples {
                samples: times.len(),
            });
        }
        Ok(UnevenCore { times, samples })
    }

    /// The domain of the curve derived from this core.
    ///
    /// # Panics
    /// This method may panic if the type's invariants aren't satisfied.
    #[inline]
    pub fn domain(&self) -> Interval {
        let start = self.times.first().unwrap();
        let end = self.times.last().unwrap();
        Interval::new(*start, *end).unwrap()
    }

    /// Obtain a value from the held samples using the given `interpolation` to interpolate
    /// between adjacent samples.
    ///
    /// The interpolation takes two values by reference together with a scalar parameter and
    /// produces an owned value. The expectation is that `interpolation(&x, &y, 0.0)` and
    /// `interpolation(&x, &y, 1.0)` are equivalent to `x` and `y` respectively.
    #[inline]
    pub fn sample_with<I>(&self, t: f32, interpolation: I) -> T
    where
        T: Clone,
        I: Fn(&T, &T, f32) -> T,
    {
        match uneven_betweenness(&self.times, t) {
            Betweenness::Exact(idx) | Betweenness::LeftTail(idx) | Betweenness::RightTail(idx) => {
                self.samples[idx].clone()
            }
            Betweenness::Between(lower_idx, upper_idx, s) => {
                interpolation(&self.samples[lower_idx], &self.samples[upper_idx], s)
            }
        }
    }

    /// Given a time `t`, obtain a [`Betweenness`] which governs how interpolation might recover
    /// a sample at time `t`. For example, when a [`Between`] value is returned, its contents can
    /// be used to interpolate between the two contained values with the given parameter. The other
    /// variants give additional context about where the value is relative to the family of samples.
    ///
    /// [`Between`]: `Betweenness::Between`
    pub fn sample_betweenness(&self, t: f32) -> Betweenness<&T> {
        uneven_betweenness(&self.times, t).map(|idx| &self.samples[idx])
    }

    /// This core, but with the sample times moved by the map `f`.
    /// In principle, when `f` is monotone, this is equivalent to [`Curve::reparametrize`],
    /// but the function inputs to each are inverses of one another.
    ///
    /// The samples are re-sorted by time after mapping and deduplicated by output time, so
    /// the function `f` should generally be injective over the sample times of the curve.
    pub fn map_sample_times(mut self, f: impl Fn(f32) -> f32) -> UnevenCore<T> {
        let mut timed_samples: Vec<(f32, T)> =
            self.times.into_iter().map(f).zip(self.samples).collect();
        timed_samples.dedup_by(|(t1, _), (t2, _)| (*t1).eq(t2));
        timed_samples.sort_by(|(t1, _), (t2, _)| t1.partial_cmp(t2).unwrap());
        self.times = timed_samples.iter().map(|(t, _)| t).copied().collect();
        self.samples = timed_samples.into_iter().map(|(_, x)| x).collect();
        self
    }
}

/// The data core of a curve using uneven samples (i.e. keyframes), where each sample time
/// yields some fixed number of values — the [sampling width]. This may serve as storage for
/// curves that yield vectors or iterators, and in some cases, it may be useful for cache locality
/// if the sample type can effectively be encoded as a fixed-length array.
///
/// [sampling width]: ChunkedUnevenCore::width
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct ChunkedUnevenCore<T> {
    /// The times, one for each sample.
    ///
    /// # Invariants
    /// This must always have a length of at least 2, be sorted, and have no
    /// duplicated or non-finite times.
    pub times: Vec<f32>,

    /// The values that are used in sampling. Each `width` of these correspond to a single sample.
    ///
    /// # Invariants
    /// This must always have a length of `width` times that of `times`.
    pub values: Vec<T>,

    /// The sampling width, determining how many consecutive elements of `values` are taken in a
    /// single sample.
    ///
    /// # Invariants
    /// This must never be zero.
    pub width: usize,
}

/// An error that indicates that a [`ChunkedUnevenCore`] could not be formed.
#[derive(Debug, Error)]
pub enum ChunkedUnevenSampleCoreError {
    /// The width of a `ChunkedUnevenCore` cannot be zero.
    #[error("Chunk width must be at least 1")]
    ZeroWidth,

    /// At least two sample times are necessary to interpolate in `ChunkedUnevenCore`.
    #[error("Need at least two samples to create an UnevenCore, but {samples} were provided")]
    NotEnoughSamples {
        /// The number of samples that were provided.
        samples: usize,
    },

    /// The length of the value buffer is supposed to be the `width` times the number of samples.
    #[error("Expected {expected} total values based on width, but {actual} were provided")]
    MismatchedLengths {
        /// The expected length of the value buffer.
        expected: usize,
        /// The actual length of the value buffer.
        actual: usize,
    },
}

impl<T> ChunkedUnevenCore<T> {
    /// Create a new [`ChunkedUnevenCore`]. The given `times` are sorted, filtered to finite times,
    /// and deduplicated. See the [type-level documentation] for more information about this type.
    ///
    /// Produces an error in any of the following circumstances:
    /// - `width` is zero.
    /// - `times` has less than `2` valid entries.
    /// - `values` has the incorrect length relative to `times`.
    ///
    /// [type-level documentation]: ChunkedUnevenCore
    pub fn new(
        times: impl Into<Vec<f32>>,
        values: impl Into<Vec<T>>,
        width: usize,
    ) -> Result<Self, ChunkedUnevenSampleCoreError> {
        let times: Vec<f32> = times.into();
        let values: Vec<T> = values.into();

        if width == 0 {
            return Err(ChunkedUnevenSampleCoreError::ZeroWidth);
        }

        let times = filter_sort_dedup_times(times);

        if times.len() < 2 {
            return Err(ChunkedUnevenSampleCoreError::NotEnoughSamples {
                samples: times.len(),
            });
        }

        if values.len() != times.len() * width {
            return Err(ChunkedUnevenSampleCoreError::MismatchedLengths {
                expected: times.len() * width,
                actual: values.len(),
            });
        }

        Ok(Self {
            times,
            values,
            width,
        })
    }

    /// The domain of the curve derived from this core.
    ///
    /// # Panics
    /// This may panic if this type's invariants aren't met.
    #[inline]
    pub fn domain(&self) -> Interval {
        let start = self.times.first().unwrap();
        let end = self.times.last().unwrap();
        Interval::new(*start, *end).unwrap()
    }

    /// Given a time `t`, obtain a [`Betweenness`] which governs how interpolation might recover
    /// a sample at time `t`. For example, when a [`Between`] value is returned, its contents can
    /// be used to interpolate between the two contained values with the given parameter. The other
    /// variants give additional context about where the value is relative to the family of samples.
    ///
    /// [`Between`]: `Betweenness::Between`
    #[inline]
    pub fn sample_betweenness(&self, t: f32) -> Betweenness<&[T]> {
        uneven_betweenness(&self.times, t).map(|idx| self.time_index_to_slice(idx))
    }

    /// Given an index in [times], returns the slice of [values] that correspond to the sample at
    /// that time.
    ///
    /// [times]: ChunkedUnevenCore::times
    /// [values]: ChunkedUnevenCore::values
    #[inline]
    fn time_index_to_slice(&self, idx: usize) -> &[T] {
        let lower_idx = self.width * idx;
        let upper_idx = lower_idx + self.width;
        &self.values[lower_idx..upper_idx]
    }
}

/// Sort the given times, deduplicate them, and filter them to only finite times.
fn filter_sort_dedup_times(times: Vec<f32>) -> Vec<f32> {
    // Filter before sorting/deduplication so that NAN doesn't interfere with them.
    let mut times: Vec<f32> = times.into_iter().filter(|t| t.is_finite()).collect();
    times.sort_by(|t0, t1| t0.partial_cmp(t1).unwrap());
    times.dedup();
    times
}

/// Given a list of `times` and a target value, get the betweenness relationship for the
/// target value in terms of the indices of the starting list. In a sense, this encapsulates the
/// heart of uneven/keyframe sampling.
///
/// `times` is assumed to be sorted, deduplicated, and consisting only of finite values. It is also
/// assumed to contain at least two values.
///
/// # Panics
/// This function will panic if `times` contains NAN.
pub fn uneven_betweenness(times: &[f32], t: f32) -> Betweenness<usize> {
    match times.binary_search_by(|pt| pt.partial_cmp(&t).unwrap()) {
        Ok(index) => Betweenness::Exact(index),
        Err(index) => {
            if index == 0 {
                // This is before the first keyframe.
                Betweenness::LeftTail(0)
            } else if index >= times.len() {
                // This is after the last keyframe.
                Betweenness::RightTail(times.len() - 1)
            } else {
                // This is actually in the middle somewhere.
                let t_lower = times[index - 1];
                let t_upper = times[index];
                let s = (t - t_lower) / (t_upper - t_lower);
                Betweenness::Between(index - 1, index, s)
            }
        }
    }
}
