//! Core data structures to be used internally in Curve implementations.

use super::interval::Interval;
use thiserror::Error;

/// The data core of a curve derived from evenly-spaced samples. The intention is to use this
/// in addition to explicit or inferred interpolation information in user-space in order to
/// implement curves using [`domain`] and [`sample_with`]
///
/// The internals are made transparent to give curve authors freedom, but [the provided constructor]
/// enforces the required invariants.
///
/// [the provided constructor]: SampleCore::new
/// [`domain`]: SampleCore::domain
/// [`sample_with`]: SampleCore::sample_with
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
///     core: SampleCore<T>,
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
pub struct SampleCore<T> {
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

/// An error indicating that a [`SampleCore`] could not be constructed.
#[derive(Debug, Error)]
pub enum SampleCoreError {
    /// Not enough samples were provided.
    #[error("Need at least two samples to create a SampleCore, but {samples} were provided")]
    NotEnoughSamples {
        /// The number of samples that were provided.
        samples: usize,
    },

    /// Unbounded domains are not compatible with `SampleCore`.
    #[error("Cannot create a SampleCore over a domain with an infinite endpoint")]
    InfiniteDomain,
}

impl<T> SampleCore<T> {
    /// Create a new [`SampleCore`] from the specified `domain` and `samples`. An error is returned
    /// if there are not at least 2 samples or if the given domain is unbounded.
    #[inline]
    pub fn new(domain: Interval, samples: impl Into<Vec<T>>) -> Result<Self, SampleCoreError> {
        let samples: Vec<T> = samples.into();
        if samples.len() < 2 {
            return Err(SampleCoreError::NotEnoughSamples {
                samples: samples.len(),
            });
        }
        if !domain.is_finite() {
            return Err(SampleCoreError::InfiniteDomain);
        }

        Ok(SampleCore { domain, samples })
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
            interpolation(
                &self.samples[lower_index],
                &self.samples[upper_index],
                fract,
            )
        }
    }
}

/// The data core of a curve defined by unevenly-spaced samples or keyframes. The intention is to
/// use this in concert with implicitly or explicitly-defined interpolation in user-space in
/// order to implement the curve interface using [`domain`] and [`sample_with`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct UnevenSampleCore<T> {
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

/// An error indicating that an [`UnevenSampleCore`] could not be constructed.
#[derive(Debug, Error)]
pub enum UnevenSampleCoreError {
    /// Not enough samples were provided.
    #[error(
        "Need at least two samples to create an UnevenSampleCore, but {samples} were provided"
    )]
    NotEnoughSamples {
        /// The number of samples that were provided.
        samples: usize,
    },
}

impl<T> UnevenSampleCore<T> {
    /// Create a new [`UnevenSampleCore`] using the provided `interpolation` to interpolate
    /// between adjacent `timed_samples`. The given samples are filtered to finite times and
    /// sorted internally; if there are not at least 2 valid timed samples, an error will be
    /// returned.
    ///
    /// The interpolation takes two values by reference together with a scalar parameter and
    /// produces an owned value. The expectation is that `interpolation(&x, &y, 0.0)` and
    /// `interpolation(&x, &y, 1.0)` are equivalent to `x` and `y` respectively.
    pub fn new(timed_samples: impl Into<Vec<(f32, T)>>) -> Result<Self, UnevenSampleCoreError> {
        let mut timed_samples: Vec<(f32, T)> = timed_samples.into();
        // Use default Equal to not do anything in case NAN appears; it will get removed in the
        // next step anyway.
        timed_samples
            .sort_by(|(t0, _), (t1, _)| t0.partial_cmp(t1).unwrap_or(std::cmp::Ordering::Equal));
        let (times, samples): (Vec<f32>, Vec<T>) = timed_samples
            .into_iter()
            .filter(|(t, _)| t.is_finite())
            .unzip();
        if times.len() < 2 {
            return Err(UnevenSampleCoreError::NotEnoughSamples {
                samples: times.len(),
            });
        }
        Ok(UnevenSampleCore { times, samples })
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
                    interpolation(v_lower, v_upper, s)
                }
            }
        }
    }

    /// This core, but with the sample times moved by the map `f`.
    /// In principle, when `f` is monotone, this is equivalent to [`Curve::reparametrize`],
    /// but the function inputs to each are inverses of one another.
    ///
    /// The samples are re-sorted by time after mapping and deduplicated by output time, so
    /// the function `f` should generally be injective over the sample times of the curve.
    pub fn map_sample_times(mut self, f: impl Fn(f32) -> f32) -> UnevenSampleCore<T> {
        let mut timed_samples: Vec<(f32, T)> =
            self.times.into_iter().map(f).zip(self.samples).collect();
        timed_samples.dedup_by(|(t1, _), (t2, _)| (*t1).eq(t2));
        timed_samples.sort_by(|(t1, _), (t2, _)| t1.partial_cmp(t2).unwrap());
        self.times = timed_samples.iter().map(|(t, _)| t).copied().collect();
        self.samples = timed_samples.into_iter().map(|(_, x)| x).collect();
        self
    }
}

/// The data core of a curve using uneven samples taken more than one at a time.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct ChunkedUnevenSampleCore<T> {
    times: Vec<f32>,
    samples_serial: Vec<T>,
    chunk_width: usize,
}
