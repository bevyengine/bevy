//! Concrete curve structures used to load glTF curves into the animation system.

use bevy_curve::{cores::*, iterable::IterableCurve, *};
use bevy_math::{vec4, Quat, Vec4, VectorSpace};
use bevy_reflect::Reflect;
use either::Either;
use thiserror::Error;

/// A keyframe-defined curve that "interpolates" by stepping at `t = 1.0` to the next keyframe.
#[derive(Debug, Clone, Reflect)]
pub struct SteppedKeyframeCurve<T> {
    core: UnevenCore<T>,
}

impl<T> Curve<T> for SteppedKeyframeCurve<T>
where
    T: Clone,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.core.domain()
    }

    #[inline]
    fn sample_clamped(&self, t: f32) -> T {
        self.core
            .sample_with(t, |x, y, t| if t >= 1.0 { y.clone() } else { x.clone() })
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        self.sample_clamped(t)
    }
}

impl<T> SteppedKeyframeCurve<T> {
    /// Create a new [`SteppedKeyframeCurve`]. If the curve could not be constructed from the
    /// given data, an error is returned.
    #[inline]
    pub fn new(timed_samples: impl IntoIterator<Item = (f32, T)>) -> Result<Self, UnevenCoreError> {
        Ok(Self {
            core: UnevenCore::new(timed_samples)?,
        })
    }
}

/// A keyframe-defined curve that uses cubic spline interpolation, backed by a contiguous buffer.
#[derive(Debug, Clone, Reflect)]
pub struct CubicKeyframeCurve<T> {
    // Note: the sample width here should be 3.
    core: ChunkedUnevenCore<T>,
}

impl<V> Curve<V> for CubicKeyframeCurve<V>
where
    V: VectorSpace<Scalar = f32>,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.core.domain()
    }

    #[inline]
    fn sample_clamped(&self, t: f32) -> V {
        match self.core.sample_interp_timed(t) {
            // In all the cases where only one frame matters, defer to the position within it.
            InterpolationDatum::Exact((_, v))
            | InterpolationDatum::LeftTail((_, v))
            | InterpolationDatum::RightTail((_, v)) => v[1],

            InterpolationDatum::Between((t0, u), (t1, v), s) => {
                cubic_spline_interpolation(u[1], u[2], v[0], v[1], s, t1 - t0)
            }
        }
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> V {
        self.sample_clamped(t)
    }
}

impl<T> CubicKeyframeCurve<T> {
    /// Create a new [`CubicKeyframeCurve`] from keyframe `times` and their associated `values`.
    /// Because 3 values are needed to perform cubic interpolation, `values` must have triple the
    /// length of `times` — each consecutive triple `a_k, v_k, b_k` associated to time `t_k`
    /// consists of:
    /// - The in-tangent `a_k` for the sample at time `t_k`
    /// - The actual value `v_k` for the sample at time `t_k`
    /// - The out-tangent `b_k` for the sample at time `t_k`
    ///
    /// For example, for a curve built from two keyframes, the inputs would have the following form:
    /// - `times`: `[t_0, t_1]`
    /// - `values`: `[a_0, v_0, b_0, a_1, v_1, b_1]`
    #[inline]
    pub fn new(
        times: impl IntoIterator<Item = f32>,
        values: impl IntoIterator<Item = T>,
    ) -> Result<Self, ChunkedUnevenCoreError> {
        Ok(Self {
            core: ChunkedUnevenCore::new(times, values, 3)?,
        })
    }
}

// NOTE: We can probably delete `CubicRotationCurve` once we improve the `Reflect` implementations
// for the `Curve` API adaptors; this is basically a `CubicKeyframeCurve` composed with `map`.

/// A keyframe-defined curve that uses cubic spline interpolation, special-cased for quaternions
/// since it uses `Vec4` internally.
#[derive(Debug, Clone, Reflect)]
#[reflect(Clone)]
pub struct CubicRotationCurve {
    // Note: The sample width here should be 3.
    core: ChunkedUnevenCore<Vec4>,
}

impl Curve<Quat> for CubicRotationCurve {
    #[inline]
    fn domain(&self) -> Interval {
        self.core.domain()
    }

    #[inline]
    fn sample_clamped(&self, t: f32) -> Quat {
        let vec = match self.core.sample_interp_timed(t) {
            // In all the cases where only one frame matters, defer to the position within it.
            InterpolationDatum::Exact((_, v))
            | InterpolationDatum::LeftTail((_, v))
            | InterpolationDatum::RightTail((_, v)) => v[1],

            InterpolationDatum::Between((t0, u), (t1, v), s) => {
                cubic_spline_interpolation(u[1], u[2], v[0], v[1], s, t1 - t0)
            }
        };
        Quat::from_vec4(vec.normalize())
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> Quat {
        self.sample_clamped(t)
    }
}

impl CubicRotationCurve {
    /// Create a new [`CubicRotationCurve`] from keyframe `times` and their associated `values`.
    /// Because 3 values are needed to perform cubic interpolation, `values` must have triple the
    /// length of `times` — each consecutive triple `a_k, v_k, b_k` associated to time `t_k`
    /// consists of:
    /// - The in-tangent `a_k` for the sample at time `t_k`
    /// - The actual value `v_k` for the sample at time `t_k`
    /// - The out-tangent `b_k` for the sample at time `t_k`
    ///
    /// For example, for a curve built from two keyframes, the inputs would have the following form:
    /// - `times`: `[t_0, t_1]`
    /// - `values`: `[a_0, v_0, b_0, a_1, v_1, b_1]`
    ///
    /// To sample quaternions from this curve, the resulting interpolated `Vec4` output is normalized
    /// and interpreted as a quaternion.
    pub fn new(
        times: impl IntoIterator<Item = f32>,
        values: impl IntoIterator<Item = Vec4>,
    ) -> Result<Self, ChunkedUnevenCoreError> {
        Ok(Self {
            core: ChunkedUnevenCore::new(times, values, 3)?,
        })
    }
}

/// A keyframe-defined curve that uses linear interpolation over many samples at once, backed
/// by a contiguous buffer.
#[derive(Debug, Clone, Reflect)]
pub struct WideLinearKeyframeCurve<T> {
    // Here the sample width is the number of things to simultaneously interpolate.
    core: ChunkedUnevenCore<T>,
}

impl<T> IterableCurve<T> for WideLinearKeyframeCurve<T>
where
    T: VectorSpace<Scalar = f32>,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.core.domain()
    }

    #[inline]
    fn sample_iter_clamped(&self, t: f32) -> impl Iterator<Item = T> {
        match self.core.sample_interp_timed(t) {
            InterpolationDatum::Exact((_, v))
            | InterpolationDatum::LeftTail((_, v))
            | InterpolationDatum::RightTail((_, v)) => {
                // Middle third = the actual values (not the tangents).
                let width = self.core.width() / 3;
                Either::Left(v[width..(width * 2)].iter().copied())
            }

            InterpolationDatum::Between((t0, u), (t1, v), s) => Either::Right(
                cubic_spline_interpolate_slices(self.core.width() / 3, u, v, s, t1 - t0),
            ),
        }
    }

    #[inline]
    fn sample_iter_unchecked(&self, t: f32) -> impl Iterator<Item = T> {
        self.sample_iter_clamped(t)
    }
}

impl<T> WideLinearKeyframeCurve<T> {
    /// Create a new [`WideLinearKeyframeCurve`]. An error will be returned if:
    /// - `values` has length zero.
    /// - `times` has less than `2` unique valid entries.
    /// - The length of `values` is not divisible by that of `times` (once sorted, filtered,
    ///   and deduplicated).
    #[inline]
    pub fn new(
        times: impl IntoIterator<Item = f32>,
        values: impl IntoIterator<Item = T>,
    ) -> Result<Self, WideKeyframeCurveError> {
        Ok(Self {
            core: ChunkedUnevenCore::new_width_inferred(times, values)?,
        })
    }
}

/// A keyframe-defined curve that uses stepped "interpolation" over many samples at once, backed
/// by a contiguous buffer.
#[derive(Debug, Clone, Reflect)]
pub struct WideSteppedKeyframeCurve<T> {
    // Here the sample width is the number of things to simultaneously interpolate.
    core: ChunkedUnevenCore<T>,
}

impl<T> IterableCurve<T> for WideSteppedKeyframeCurve<T>
where
    T: Clone,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.core.domain()
    }

    #[inline]
    fn sample_iter_clamped(&self, t: f32) -> impl Iterator<Item = T> {
        match self.core.sample_interp(t) {
            InterpolationDatum::Exact(v)
            | InterpolationDatum::LeftTail(v)
            | InterpolationDatum::RightTail(v) => Either::Left(v.iter().cloned()),

            InterpolationDatum::Between(u, v, s) => {
                let interpolated =
                    u.iter()
                        .zip(v.iter())
                        .map(move |(x, y)| if s >= 1.0 { y.clone() } else { x.clone() });
                Either::Right(interpolated)
            }
        }
    }

    #[inline]
    fn sample_iter_unchecked(&self, t: f32) -> impl Iterator<Item = T> {
        self.sample_iter_clamped(t)
    }
}

impl<T> WideSteppedKeyframeCurve<T> {
    /// Create a new [`WideSteppedKeyframeCurve`]. An error will be returned if:
    /// - `values` has length zero.
    /// - `times` has less than `2` unique valid entries.
    /// - The length of `values` is not divisible by that of `times` (once sorted, filtered,
    ///   and deduplicated).
    #[inline]
    pub fn new(
        times: impl IntoIterator<Item = f32>,
        values: impl IntoIterator<Item = T>,
    ) -> Result<Self, WideKeyframeCurveError> {
        Ok(Self {
            core: ChunkedUnevenCore::new_width_inferred(times, values)?,
        })
    }
}

/// A keyframe-defined curve that uses cubic interpolation over many samples at once, backed by a
/// contiguous buffer.
#[derive(Debug, Clone, Reflect)]
pub struct WideCubicKeyframeCurve<T> {
    core: ChunkedUnevenCore<T>,
}

impl<T> IterableCurve<T> for WideCubicKeyframeCurve<T>
where
    T: VectorSpace<Scalar = f32>,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.core.domain()
    }

    fn sample_iter_clamped(&self, t: f32) -> impl Iterator<Item = T> {
        match self.core.sample_interp_timed(t) {
            InterpolationDatum::Exact((_, v))
            | InterpolationDatum::LeftTail((_, v))
            | InterpolationDatum::RightTail((_, v)) => {
                // Pick out the part of this that actually represents the position (instead of tangents),
                // which is the middle third.
                let width = self.core.width() / 3;
                Either::Left(v[width..(width * 2)].iter().copied())
            }

            InterpolationDatum::Between((t0, u), (t1, v), s) => Either::Right(
                cubic_spline_interpolate_slices(self.core.width() / 3, u, v, s, t1 - t0),
            ),
        }
    }

    #[inline]
    fn sample_iter_unchecked(&self, t: f32) -> impl Iterator<Item = T> {
        self.sample_iter_clamped(t)
    }
}

/// An error indicating that a multisampling keyframe curve could not be constructed.
#[derive(Debug, Error)]
#[error("unable to construct a curve using this data")]
pub enum WideKeyframeCurveError {
    /// The number of given values was not divisible by a multiple of the number of keyframes.
    #[error("number of values ({values_given}) is not divisible by {divisor}")]
    LengthMismatch {
        /// The number of values given.
        values_given: usize,
        /// The number that `values_given` was supposed to be divisible by.
        divisor: usize,
    },
    /// An error was returned by the internal core constructor.
    #[error(transparent)]
    CoreError(#[from] ChunkedUnevenCoreError),
}

impl<T> WideCubicKeyframeCurve<T> {
    /// Create a new [`WideCubicKeyframeCurve`].
    ///
    /// An error will be returned if:
    /// - `values` has length zero.
    /// - `times` has less than `2` unique valid entries.
    /// - The length of `values` is not divisible by three times that of `times` (once sorted,
    ///   filtered, and deduplicated).
    #[inline]
    pub fn new(
        times: impl IntoIterator<Item = f32>,
        values: impl IntoIterator<Item = T>,
    ) -> Result<Self, WideKeyframeCurveError> {
        let times: Vec<f32> = times.into_iter().collect();
        let values: Vec<T> = values.into_iter().collect();
        let divisor = times.len() * 3;

        if !values.len().is_multiple_of(divisor) {
            return Err(WideKeyframeCurveError::LengthMismatch {
                values_given: values.len(),
                divisor,
            });
        }

        Ok(Self {
            core: ChunkedUnevenCore::new_width_inferred(times, values)?,
        })
    }
}

//---------//
// HELPERS //
//---------//

/// Helper function for cubic spline interpolation.
fn cubic_spline_interpolation<T>(
    value_start: T,
    tangent_out_start: T,
    tangent_in_end: T,
    value_end: T,
    lerp: f32,
    step_duration: f32,
) -> T
where
    T: VectorSpace<Scalar = f32>,
{
    let coeffs = (vec4(2.0, 1.0, -2.0, 1.0) * lerp + vec4(-3.0, -2.0, 3.0, -1.0)) * lerp;
    value_start * (coeffs.x * lerp + 1.0)
        + tangent_out_start * step_duration * lerp * (coeffs.y + 1.0)
        + value_end * lerp * coeffs.z
        + tangent_in_end * step_duration * lerp * coeffs.w
}

fn cubic_spline_interpolate_slices<'a, T: VectorSpace<Scalar = f32>>(
    width: usize,
    first: &'a [T],
    second: &'a [T],
    s: f32,
    step_between: f32,
) -> impl Iterator<Item = T> + 'a {
    (0..width).map(move |idx| {
        cubic_spline_interpolation(
            first[idx + width],
            first[idx + (width * 2)],
            second[idx],
            second[idx + width],
            s,
            step_between,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hermite(p0: f32, m0: f32, m1: f32, p1: f32, s: f32, dt: f32) -> f32 {
        let s2 = s * s;
        let s3 = s2 * s;
        (2.0 * s3 - 3.0 * s2 + 1.0) * p0
            + (s3 - 2.0 * s2 + s) * dt * m0
            + (-2.0 * s3 + 3.0 * s2) * p1
            + (s3 - s2) * dt * m1
    }

    #[test]
    fn wide_cubic_keyframe_curve_interpolates_multiple_channels() {
        let times = [0.0, 2.0];
        let mut values = Vec::new();
        // Keyframe 0 at t = 0.0: 2 channels (in, value, out).
        values.extend_from_slice(&[0.2, -0.4]);
        values.extend_from_slice(&[0.0, 1.0]);
        values.extend_from_slice(&[0.5, -0.25]);
        // Keyframe 1 at t = 2.0: 2 channels (in, value, out).
        values.extend_from_slice(&[-0.3, 0.6]);
        values.extend_from_slice(&[2.0, -1.0]);
        values.extend_from_slice(&[0.1, 0.4]);

        let curve = WideCubicKeyframeCurve::new(times, values).unwrap();

        let t = 0.75;
        let s = 0.375;
        let dt = 2.0;
        let sampled: Vec<f32> = curve.sample_iter_clamped(t).collect();
        let expected = [
            hermite(0.0, 0.5, -0.3, 2.0, s, dt),
            hermite(1.0, -0.25, 0.6, -1.0, s, dt),
        ];

        assert_eq!(sampled.len(), 2);
        assert!((sampled[0] - expected[0]).abs() <= 1e-5);
        assert!((sampled[1] - expected[1]).abs() <= 1e-5);
    }

    #[test]
    fn wide_cubic_keyframe_curve_clamps_and_hits_exact_keys() {
        let times = [0.0, 1.0];
        let mut values = Vec::new();
        values.extend_from_slice(&[9.0, 9.0]);
        values.extend_from_slice(&[1.0, 2.0]);
        values.extend_from_slice(&[8.0, 8.0]);
        values.extend_from_slice(&[7.0, 7.0]);
        values.extend_from_slice(&[3.0, 4.0]);
        values.extend_from_slice(&[6.0, 6.0]);

        let curve = WideCubicKeyframeCurve::new(times, values).unwrap();

        assert_eq!(
            curve.sample_iter_clamped(-0.5).collect::<Vec<_>>(),
            vec![1.0, 2.0]
        );
        assert_eq!(
            curve.sample_iter_clamped(0.0).collect::<Vec<_>>(),
            vec![1.0, 2.0]
        );
        assert_eq!(
            curve.sample_iter_clamped(1.0).collect::<Vec<_>>(),
            vec![3.0, 4.0]
        );
        assert_eq!(
            curve.sample_iter_clamped(1.5).collect::<Vec<_>>(),
            vec![3.0, 4.0]
        );
    }
}
