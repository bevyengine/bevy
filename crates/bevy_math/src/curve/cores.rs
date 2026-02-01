//! Core data structures to be used internally in Curve implementations, encapsulating storage
//! and access patterns for reuse.
//!
//! The `Core` types here expose their fields publicly so that it is easier to manipulate and
//! extend them, but in doing so, you must maintain the invariants of those fields yourself. The
//! provided methods all maintain the invariants, so this is only a concern if you manually mutate
//! the fields.

use crate::ops;

use super::interval::Interval;
use core::fmt::Debug;
use thiserror::Error;

#[cfg(feature = "alloc")]
use {alloc::vec::Vec, itertools::Itertools};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

/// This type expresses the relationship of a value to a fixed collection of values. It is a kind
/// of summary used intermediately by sampling operations.
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub enum InterpolationDatum<T> {
    /// This value lies exactly on a value in the family.
    Exact(T),

    /// This value is off the left tail of the family; the inner value is the family's leftmost.
    LeftTail(T),

    /// This value is off the right tail of the family; the inner value is the family's rightmost.
    RightTail(T),

    /// This value lies on the interior, in between two points, with a third parameter expressing
    /// the interpolation factor between the two.
    Between(T, T, f32),
}

impl<T> InterpolationDatum<T> {
    /// Map all values using a given function `f`, leaving the interpolation parameters in any
    /// [`Between`] variants unchanged.
    ///
    /// [`Between`]: `InterpolationDatum::Between`
    #[must_use]
    pub fn map<S>(self, f: impl Fn(T) -> S) -> InterpolationDatum<S> {
        match self {
            InterpolationDatum::Exact(v) => InterpolationDatum::Exact(f(v)),
            InterpolationDatum::LeftTail(v) => InterpolationDatum::LeftTail(f(v)),
            InterpolationDatum::RightTail(v) => InterpolationDatum::RightTail(f(v)),
            InterpolationDatum::Between(u, v, s) => InterpolationDatum::Between(f(u), f(v), s),
        }
    }
}

/// The data core of a curve derived from evenly-spaced samples. The intention is to use this
/// in addition to explicit or inferred interpolation information in user-space in order to
/// implement curves using [`domain`] and [`sample_with`].
///
/// The internals are made transparent to give curve authors freedom, but [the provided constructor]
/// enforces the required invariants, and the methods maintain those invariants.
///
/// [the provided constructor]: EvenCore::new
/// [`domain`]: EvenCore::domain
/// [`sample_with`]: EvenCore::sample_with
///
/// # Example
/// ```rust
/// # use bevy_math::curve::*;
/// # use bevy_math::curve::cores::*;
/// // Let's make a curve that interpolates evenly spaced samples using either linear interpolation
/// // or step "interpolation" — i.e. just using the most recent sample as the source of truth.
/// enum InterpolationMode {
///     Linear,
///     Step,
/// }
///
/// // Linear interpolation mode is driven by a trait.
/// trait LinearInterpolate {
///     fn lerp(&self, other: &Self, t: f32) -> Self;
/// }
///
/// // Step interpolation just uses an explicit function.
/// fn step<T: Clone>(first: &T, second: &T, t: f32) -> T {
///     if t >= 1.0 {
///         second.clone()
///     } else {
///         first.clone()
///     }
/// }
///
/// // Omitted: Implementing `LinearInterpolate` on relevant types; e.g. `f32`, `Vec3`, and so on.
///
/// // The curve itself uses `EvenCore` to hold the evenly-spaced samples, and the `sample_with`
/// // function will do all the work of interpolating once given a function to do it with.
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
///     fn sample_unchecked(&self, t: f32) -> T {
///         // To sample this curve, check the interpolation mode and dispatch accordingly.
///         match self.interpolation_mode {
///             InterpolationMode::Linear => self.core.sample_with(t, <T as LinearInterpolate>::lerp),
///             InterpolationMode::Step => self.core.sample_with(t, step),
///         }
///     }
/// }
/// ```
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
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

/// An error indicating that an [`EvenCore`] could not be constructed.
#[derive(Debug, Error)]
#[error("Could not construct an EvenCore")]
pub enum EvenCoreError {
    /// Not enough samples were provided.
    #[error("Need at least two samples to create an EvenCore, but {samples} were provided")]
    NotEnoughSamples {
        /// The number of samples that were provided.
        samples: usize,
    },

    /// Unbounded domains are not compatible with `EvenCore`.
    #[error("Cannot create an EvenCore over an unbounded domain")]
    UnboundedDomain,
}

#[cfg(feature = "alloc")]
impl<T> EvenCore<T> {
    /// Create a new [`EvenCore`] from the specified `domain` and `samples`. The samples are
    /// regarded to be evenly spaced within the given domain interval, so that the outermost
    /// samples form the boundary of that interval. An error is returned if there are not at
    /// least 2 samples or if the given domain is unbounded.
    #[inline]
    pub fn new(
        domain: Interval,
        samples: impl IntoIterator<Item = T>,
    ) -> Result<Self, EvenCoreError> {
        let samples: Vec<T> = samples.into_iter().collect();
        if samples.len() < 2 {
            return Err(EvenCoreError::NotEnoughSamples {
                samples: samples.len(),
            });
        }
        if !domain.is_bounded() {
            return Err(EvenCoreError::UnboundedDomain);
        }

        Ok(EvenCore { domain, samples })
    }

    /// The domain of the curve derived from this core.
    #[inline]
    pub const fn domain(&self) -> Interval {
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
        match even_interp(self.domain, self.samples.len(), t) {
            InterpolationDatum::Exact(idx)
            | InterpolationDatum::LeftTail(idx)
            | InterpolationDatum::RightTail(idx) => self.samples[idx].clone(),
            InterpolationDatum::Between(lower_idx, upper_idx, s) => {
                interpolation(&self.samples[lower_idx], &self.samples[upper_idx], s)
            }
        }
    }

    /// Given a time `t`, obtain a [`InterpolationDatum`] which governs how interpolation might recover
    /// a sample at time `t`. For example, when a [`Between`] value is returned, its contents can
    /// be used to interpolate between the two contained values with the given parameter. The other
    /// variants give additional context about where the value is relative to the family of samples.
    ///
    /// [`Between`]: `InterpolationDatum::Between`
    pub fn sample_interp(&self, t: f32) -> InterpolationDatum<&T> {
        even_interp(self.domain, self.samples.len(), t).map(|idx| &self.samples[idx])
    }

    /// Like [`sample_interp`], but the returned values include the sample times. This can be
    /// useful when sample interpolation is not scale-invariant.
    ///
    /// [`sample_interp`]: EvenCore::sample_interp
    pub fn sample_interp_timed(&self, t: f32) -> InterpolationDatum<(f32, &T)> {
        let segment_len = self.domain.length() / (self.samples.len() - 1) as f32;
        even_interp(self.domain, self.samples.len(), t).map(|idx| {
            (
                self.domain.start() + segment_len * idx as f32,
                &self.samples[idx],
            )
        })
    }
}

/// Given a domain and a number of samples taken over that interval, return an [`InterpolationDatum`]
/// that governs how samples are extracted relative to the stored data.
///
/// `domain` must be a bounded interval (i.e. `domain.is_bounded() == true`).
///
/// `samples` must be at least 2.
///
/// This function will never panic, but it may return invalid indices if its assumptions are violated.
pub fn even_interp(domain: Interval, samples: usize, t: f32) -> InterpolationDatum<usize> {
    let subdivs = samples - 1;
    let step = domain.length() / subdivs as f32;
    let t_shifted = t - domain.start();
    let steps_taken = t_shifted / step;

    if steps_taken <= 0.0 {
        // To the left side of all the samples.
        InterpolationDatum::LeftTail(0)
    } else if steps_taken >= subdivs as f32 {
        // To the right side of all the samples
        InterpolationDatum::RightTail(samples - 1)
    } else {
        let lower_index = ops::floor(steps_taken) as usize;
        // This upper index is always valid because `steps_taken` is a finite value
        // strictly less than `samples - 1`, so its floor is at most `samples - 2`
        let upper_index = lower_index + 1;
        let s = ops::fract(steps_taken);
        InterpolationDatum::Between(lower_index, upper_index, s)
    }
}

/// The data core of a curve defined by unevenly-spaced samples or keyframes. The intention is to
/// use this in concert with implicitly or explicitly-defined interpolation in user-space in
/// order to implement the curve interface using [`domain`] and [`sample_with`].
///
/// The internals are made transparent to give curve authors freedom, but [the provided constructor]
/// enforces the required invariants, and the methods maintain those invariants.
///
/// # Example
/// ```rust
/// # use bevy_math::curve::*;
/// # use bevy_math::curve::cores::*;
/// // Let's make a curve formed by interpolating rotations.
/// // We'll support two common modes of interpolation:
/// // - Normalized linear: First do linear interpolation, then normalize to get a valid rotation.
/// // - Spherical linear: Interpolate through valid rotations with constant angular velocity.
/// enum InterpolationMode {
///     NormalizedLinear,
///     SphericalLinear,
/// }
///
/// // Our interpolation modes will be driven by traits.
/// trait NormalizedLinearInterpolate {
///     fn nlerp(&self, other: &Self, t: f32) -> Self;
/// }
///
/// trait SphericalLinearInterpolate {
///     fn slerp(&self, other: &Self, t: f32) -> Self;
/// }
///
/// // Omitted: These traits would be implemented for `Rot2`, `Quat`, and other rotation representations.
///
/// // The curve itself just needs to use the curve core for keyframes, `UnevenCore`, which handles
/// // everything except for the explicit interpolation used.
/// struct RotationCurve<T> {
///     core: UnevenCore<T>,
///     interpolation_mode: InterpolationMode,
/// }
///
/// impl<T> Curve<T> for RotationCurve<T>
/// where
///     T: NormalizedLinearInterpolate + SphericalLinearInterpolate + Clone,
/// {
///     fn domain(&self) -> Interval {
///         self.core.domain()
///     }
///     
///     fn sample_unchecked(&self, t: f32) -> T {
///         // To sample the curve, we just look at the interpolation mode and
///         // dispatch accordingly.
///         match self.interpolation_mode {
///             InterpolationMode::NormalizedLinear =>
///                 self.core.sample_with(t, <T as NormalizedLinearInterpolate>::nlerp),
///             InterpolationMode::SphericalLinear =>
///                 self.core.sample_with(t, <T as SphericalLinearInterpolate>::slerp),
///         }
///     }
/// }
/// ```
///
/// [`domain`]: UnevenCore::domain
/// [`sample_with`]: UnevenCore::sample_with
/// [the provided constructor]: UnevenCore::new
#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
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
#[error("Could not construct an UnevenCore")]
pub enum UnevenCoreError {
    /// Not enough samples were provided.
    #[error(
        "Need at least two unique samples to create an UnevenCore, but {samples} were provided"
    )]
    NotEnoughSamples {
        /// The number of samples that were provided.
        samples: usize,
    },
}

#[cfg(feature = "alloc")]
impl<T> UnevenCore<T> {
    /// Create a new [`UnevenCore`]. The given samples are filtered to finite times and
    /// sorted internally; if there are not at least 2 valid timed samples, an error will be
    /// returned.
    pub fn new(timed_samples: impl IntoIterator<Item = (f32, T)>) -> Result<Self, UnevenCoreError> {
        // Filter out non-finite sample times first so they don't interfere with sorting/deduplication.
        let mut timed_samples = timed_samples
            .into_iter()
            .filter(|(t, _)| t.is_finite())
            .collect_vec();
        timed_samples
            // Using `total_cmp` is fine because no NANs remain and because deduplication uses
            // `PartialEq` anyway (so -0.0 and 0.0 will be considered equal later regardless).
            .sort_by(|(t0, _), (t1, _)| t0.total_cmp(t1));
        timed_samples.dedup_by_key(|(t, _)| *t);

        if timed_samples.len() < 2 {
            return Err(UnevenCoreError::NotEnoughSamples {
                samples: timed_samples.len(),
            });
        }

        let (times, samples): (Vec<f32>, Vec<T>) = timed_samples.into_iter().unzip();
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
        match uneven_interp(&self.times, t) {
            InterpolationDatum::Exact(idx)
            | InterpolationDatum::LeftTail(idx)
            | InterpolationDatum::RightTail(idx) => self.samples[idx].clone(),
            InterpolationDatum::Between(lower_idx, upper_idx, s) => {
                interpolation(&self.samples[lower_idx], &self.samples[upper_idx], s)
            }
        }
    }

    /// Given a time `t`, obtain a [`InterpolationDatum`] which governs how interpolation might recover
    /// a sample at time `t`. For example, when a [`Between`] value is returned, its contents can
    /// be used to interpolate between the two contained values with the given parameter. The other
    /// variants give additional context about where the value is relative to the family of samples.
    ///
    /// [`Between`]: `InterpolationDatum::Between`
    pub fn sample_interp(&self, t: f32) -> InterpolationDatum<&T> {
        uneven_interp(&self.times, t).map(|idx| &self.samples[idx])
    }

    /// Like [`sample_interp`], but the returned values include the sample times. This can be
    /// useful when sample interpolation is not scale-invariant.
    ///
    /// [`sample_interp`]: UnevenCore::sample_interp
    pub fn sample_interp_timed(&self, t: f32) -> InterpolationDatum<(f32, &T)> {
        uneven_interp(&self.times, t).map(|idx| (self.times[idx], &self.samples[idx]))
    }

    /// This core, but with the sample times moved by the map `f`.
    /// In principle, when `f` is monotone, this is equivalent to [`CurveExt::reparametrize`],
    /// but the function inputs to each are inverses of one another.
    ///
    /// The samples are re-sorted by time after mapping and deduplicated by output time, so
    /// the function `f` should generally be injective over the set of sample times, otherwise
    /// data will be deleted.
    ///
    /// [`CurveExt::reparametrize`]: crate::curve::CurveExt::reparametrize
    #[must_use]
    pub fn map_sample_times(mut self, f: impl Fn(f32) -> f32) -> UnevenCore<T> {
        let mut timed_samples = self
            .times
            .into_iter()
            .map(f)
            .zip(self.samples)
            .collect_vec();
        timed_samples.sort_by(|(t1, _), (t2, _)| t1.total_cmp(t2));
        timed_samples.dedup_by_key(|(t, _)| *t);
        (self.times, self.samples) = timed_samples.into_iter().unzip();
        self
    }
}

/// The data core of a curve using uneven samples (i.e. keyframes), where each sample time
/// yields some fixed number of values — the [sampling width]. This may serve as storage for
/// curves that yield vectors or iterators, and in some cases, it may be useful for cache locality
/// if the sample type can effectively be encoded as a fixed-length slice of values.
///
/// [sampling width]: ChunkedUnevenCore::width
#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct ChunkedUnevenCore<T> {
    /// The times, one for each sample.
    ///
    /// # Invariants
    /// This must always have a length of at least 2, be sorted, and have no duplicated or
    /// non-finite times.
    pub times: Vec<f32>,

    /// The values that are used in sampling. Each width-worth of these correspond to a single sample.
    ///
    /// # Invariants
    /// The length of this vector must always be some fixed integer multiple of that of `times`.
    pub values: Vec<T>,
}

/// An error that indicates that a [`ChunkedUnevenCore`] could not be formed.
#[derive(Debug, Error)]
#[error("Could not create a ChunkedUnevenCore")]
pub enum ChunkedUnevenCoreError {
    /// The width of a `ChunkedUnevenCore` cannot be zero.
    #[error("Chunk width must be at least 1")]
    ZeroWidth,

    /// At least two sample times are necessary to interpolate in `ChunkedUnevenCore`.
    #[error(
        "Need at least two unique samples to create a ChunkedUnevenCore, but {samples} were provided"
    )]
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

    /// Tried to infer the width, but the ratio of lengths wasn't an integer, so no such length exists.
    #[error("The length of the list of values ({values_len}) was not divisible by that of the list of times ({times_len})")]
    NonDivisibleLengths {
        /// The length of the value buffer.
        values_len: usize,
        /// The length of the time buffer.
        times_len: usize,
    },
}

#[cfg(feature = "alloc")]
impl<T> ChunkedUnevenCore<T> {
    /// Create a new [`ChunkedUnevenCore`]. The given `times` are sorted, filtered to finite times,
    /// and deduplicated. See the [type-level documentation] for more information about this type.
    ///
    /// Produces an error in any of the following circumstances:
    /// - `width` is zero.
    /// - `times` has less than `2` unique valid entries.
    /// - `values` has the incorrect length relative to `times`.
    ///
    /// [type-level documentation]: ChunkedUnevenCore
    pub fn new(
        times: impl IntoIterator<Item = f32>,
        values: impl IntoIterator<Item = T>,
        width: usize,
    ) -> Result<Self, ChunkedUnevenCoreError> {
        let times = times.into_iter().collect_vec();
        let values = values.into_iter().collect_vec();

        if width == 0 {
            return Err(ChunkedUnevenCoreError::ZeroWidth);
        }

        let times = filter_sort_dedup_times(times);

        if times.len() < 2 {
            return Err(ChunkedUnevenCoreError::NotEnoughSamples {
                samples: times.len(),
            });
        }

        if values.len() != times.len() * width {
            return Err(ChunkedUnevenCoreError::MismatchedLengths {
                expected: times.len() * width,
                actual: values.len(),
            });
        }

        Ok(Self { times, values })
    }

    /// Create a new [`ChunkedUnevenCore`], inferring the width from the sizes of the inputs.
    /// The given `times` are sorted, filtered to finite times, and deduplicated. See the
    /// [type-level documentation] for more information about this type. Prefer using [`new`]
    /// if possible, since that constructor has richer error checking.
    ///
    /// Produces an error in any of the following circumstances:
    /// - `values` has length zero.
    /// - `times` has less than `2` unique valid entries.
    /// - The length of `values` is not divisible by that of `times` (once sorted, filtered,
    ///   and deduplicated).
    ///
    /// The [width] is implicitly taken to be the length of `values` divided by that of `times`
    /// (once sorted, filtered, and deduplicated).
    ///
    /// [type-level documentation]: ChunkedUnevenCore
    /// [`new`]: ChunkedUnevenCore::new
    /// [width]: ChunkedUnevenCore::width
    pub fn new_width_inferred(
        times: impl IntoIterator<Item = f32>,
        values: impl IntoIterator<Item = T>,
    ) -> Result<Self, ChunkedUnevenCoreError> {
        let times = times.into_iter().collect_vec();
        let values = values.into_iter().collect_vec();

        let times = filter_sort_dedup_times(times);

        if times.len() < 2 {
            return Err(ChunkedUnevenCoreError::NotEnoughSamples {
                samples: times.len(),
            });
        }

        if values.len() % times.len() != 0 {
            return Err(ChunkedUnevenCoreError::NonDivisibleLengths {
                values_len: values.len(),
                times_len: times.len(),
            });
        }

        if values.is_empty() {
            return Err(ChunkedUnevenCoreError::ZeroWidth);
        }

        Ok(Self { times, values })
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

    /// The sample width: the number of values that are contained in each sample.
    #[inline]
    pub fn width(&self) -> usize {
        self.values.len() / self.times.len()
    }

    /// Given a time `t`, obtain a [`InterpolationDatum`] which governs how interpolation might recover
    /// a sample at time `t`. For example, when a [`Between`] value is returned, its contents can
    /// be used to interpolate between the two contained values with the given parameter. The other
    /// variants give additional context about where the value is relative to the family of samples.
    ///
    /// [`Between`]: `InterpolationDatum::Between`
    #[inline]
    pub fn sample_interp(&self, t: f32) -> InterpolationDatum<&[T]> {
        uneven_interp(&self.times, t).map(|idx| self.time_index_to_slice(idx))
    }

    /// Like [`sample_interp`], but the returned values include the sample times. This can be
    /// useful when sample interpolation is not scale-invariant.
    ///
    /// [`sample_interp`]: ChunkedUnevenCore::sample_interp
    pub fn sample_interp_timed(&self, t: f32) -> InterpolationDatum<(f32, &[T])> {
        uneven_interp(&self.times, t).map(|idx| (self.times[idx], self.time_index_to_slice(idx)))
    }

    /// Given an index in [times], returns the slice of [values] that correspond to the sample at
    /// that time.
    ///
    /// [times]: ChunkedUnevenCore::times
    /// [values]: ChunkedUnevenCore::values
    #[inline]
    fn time_index_to_slice(&self, idx: usize) -> &[T] {
        let width = self.width();
        let lower_idx = width * idx;
        let upper_idx = lower_idx + width;
        &self.values[lower_idx..upper_idx]
    }
}

/// Sort the given times, deduplicate them, and filter them to only finite times.
#[cfg(feature = "alloc")]
fn filter_sort_dedup_times(times: impl IntoIterator<Item = f32>) -> Vec<f32> {
    // Filter before sorting/deduplication so that NAN doesn't interfere with them.
    let mut times = times.into_iter().filter(|t| t.is_finite()).collect_vec();
    times.sort_by(f32::total_cmp);
    times.dedup();
    times
}

/// Given a list of `times` and a target value, get the interpolation relationship for the
/// target value in terms of the indices of the starting list. In a sense, this encapsulates the
/// heart of uneven/keyframe sampling.
///
/// `times` is assumed to be sorted, deduplicated, and consisting only of finite values. It is also
/// assumed to contain at least two values.
///
/// # Panics
/// This function will panic if `times` contains NAN.
pub fn uneven_interp(times: &[f32], t: f32) -> InterpolationDatum<usize> {
    match times.binary_search_by(|pt| pt.partial_cmp(&t).unwrap()) {
        Ok(index) => InterpolationDatum::Exact(index),
        Err(index) => {
            if index == 0 {
                // This is before the first keyframe.
                InterpolationDatum::LeftTail(0)
            } else if index >= times.len() {
                // This is after the last keyframe.
                InterpolationDatum::RightTail(times.len() - 1)
            } else {
                // This is actually in the middle somewhere.
                let t_lower = times[index - 1];
                let t_upper = times[index];
                let s = (t - t_lower) / (t_upper - t_lower);
                InterpolationDatum::Between(index - 1, index, s)
            }
        }
    }
}

#[cfg(all(test, feature = "alloc"))]
mod tests {
    use super::{ChunkedUnevenCore, EvenCore, UnevenCore};
    use crate::curve::{cores::InterpolationDatum, interval};
    use alloc::vec;
    use approx::{assert_abs_diff_eq, AbsDiffEq};

    fn approx_between<T>(datum: InterpolationDatum<T>, start: T, end: T, p: f32) -> bool
    where
        T: PartialEq,
    {
        if let InterpolationDatum::Between(m_start, m_end, m_p) = datum {
            m_start == start && m_end == end && m_p.abs_diff_eq(&p, 1e-6)
        } else {
            false
        }
    }

    fn is_left_tail<T>(datum: InterpolationDatum<T>) -> bool {
        matches!(datum, InterpolationDatum::LeftTail(_))
    }

    fn is_right_tail<T>(datum: InterpolationDatum<T>) -> bool {
        matches!(datum, InterpolationDatum::RightTail(_))
    }

    fn is_exact<T>(datum: InterpolationDatum<T>, target: T) -> bool
    where
        T: PartialEq,
    {
        if let InterpolationDatum::Exact(v) = datum {
            v == target
        } else {
            false
        }
    }

    #[test]
    fn even_sample_interp() {
        let even_core = EvenCore::<f32>::new(
            interval(0.0, 1.0).unwrap(),
            // 11 entries -> 10 segments
            vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0],
        )
        .expect("Failed to construct test core");

        let datum = even_core.sample_interp(-1.0);
        assert!(is_left_tail(datum));
        let datum = even_core.sample_interp(0.0);
        assert!(is_left_tail(datum));
        let datum = even_core.sample_interp(1.0);
        assert!(is_right_tail(datum));
        let datum = even_core.sample_interp(2.0);
        assert!(is_right_tail(datum));

        let datum = even_core.sample_interp(0.05);
        let InterpolationDatum::Between(0.0, 1.0, p) = datum else {
            panic!("Sample did not lie in the correct subinterval")
        };
        assert_abs_diff_eq!(p, 0.5);

        let datum = even_core.sample_interp(0.05);
        assert!(approx_between(datum, &0.0, &1.0, 0.5));
        let datum = even_core.sample_interp(0.33);
        assert!(approx_between(datum, &3.0, &4.0, 0.3));
        let datum = even_core.sample_interp(0.78);
        assert!(approx_between(datum, &7.0, &8.0, 0.8));

        let datum = even_core.sample_interp(0.5);
        assert!(approx_between(datum, &4.0, &5.0, 1.0) || approx_between(datum, &5.0, &6.0, 0.0));
        let datum = even_core.sample_interp(0.7);
        assert!(approx_between(datum, &6.0, &7.0, 1.0) || approx_between(datum, &7.0, &8.0, 0.0));
    }

    #[test]
    fn uneven_sample_interp() {
        let uneven_core = UnevenCore::<f32>::new(vec![
            (0.0, 0.0),
            (1.0, 3.0),
            (2.0, 9.0),
            (4.0, 10.0),
            (8.0, -5.0),
        ])
        .expect("Failed to construct test core");

        let datum = uneven_core.sample_interp(-1.0);
        assert!(is_left_tail(datum));
        let datum = uneven_core.sample_interp(0.0);
        assert!(is_exact(datum, &0.0));
        let datum = uneven_core.sample_interp(8.0);
        assert!(is_exact(datum, &(-5.0)));
        let datum = uneven_core.sample_interp(9.0);
        assert!(is_right_tail(datum));

        let datum = uneven_core.sample_interp(0.5);
        assert!(approx_between(datum, &0.0, &3.0, 0.5));
        let datum = uneven_core.sample_interp(2.5);
        assert!(approx_between(datum, &9.0, &10.0, 0.25));
        let datum = uneven_core.sample_interp(7.0);
        assert!(approx_between(datum, &10.0, &(-5.0), 0.75));

        let datum = uneven_core.sample_interp(2.0);
        assert!(is_exact(datum, &9.0));
        let datum = uneven_core.sample_interp(4.0);
        assert!(is_exact(datum, &10.0));
    }

    #[test]
    fn chunked_uneven_sample_interp() {
        let core =
            ChunkedUnevenCore::new(vec![0.0, 2.0, 8.0], vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0], 2)
                .expect("Failed to construct test core");

        let datum = core.sample_interp(-1.0);
        assert!(is_left_tail(datum));
        let datum = core.sample_interp(0.0);
        assert!(is_exact(datum, &[0.0, 1.0]));
        let datum = core.sample_interp(8.0);
        assert!(is_exact(datum, &[4.0, 5.0]));
        let datum = core.sample_interp(10.0);
        assert!(is_right_tail(datum));

        let datum = core.sample_interp(1.0);
        assert!(approx_between(datum, &[0.0, 1.0], &[2.0, 3.0], 0.5));
        let datum = core.sample_interp(3.0);
        assert!(approx_between(datum, &[2.0, 3.0], &[4.0, 5.0], 1.0 / 6.0));

        let datum = core.sample_interp(2.0);
        assert!(is_exact(datum, &[2.0, 3.0]));
    }
}
