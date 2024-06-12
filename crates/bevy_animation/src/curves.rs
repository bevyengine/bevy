//! Curve structures used by the animation system.

use bevy_math::{curve::cores::*, curve::*, Quat, Vec3, Vec4, VectorSpace};
use bevy_reflect::Reflect;

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
    fn sample(&self, t: f32) -> T {
        self.core
            .sample_with(t, |x, y, t| if t >= 1.0 { y.clone() } else { x.clone() })
    }
}

impl<T> SteppedKeyframeCurve<T> {
    /// Create a new [`SteppedKeyframeCurve`], bypassing any formatting. If you use this, you must
    /// uphold the invariants of [`UnevenCore`] yourself.
    #[inline]
    pub fn new_raw(times: impl Into<Vec<f32>>, samples: impl Into<Vec<T>>) -> Self {
        Self {
            core: UnevenCore {
                times: times.into(),
                samples: samples.into(),
            },
        }
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
    V: VectorSpace,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.core.domain()
    }

    #[inline]
    fn sample(&self, t: f32) -> V {
        match self.core.sample_betweenness_timed(t) {
            // In all the cases where only one frame matters, defer to the position within it.
            Betweenness::Exact((_, v))
            | Betweenness::LeftTail((_, v))
            | Betweenness::RightTail((_, v)) => v[1],

            Betweenness::Between((t0, u), (t1, v), s) => {
                cubic_spline_interpolation(u[1], u[2], v[0], v[1], s, t1 - t0)
            }
        }
    }
}

impl<T> CubicKeyframeCurve<T> {
    /// Create a new [`CubicKeyframeCurve`] from raw data, bypassing all checks. If you use this, you
    /// must uphold the invariants of [`ChunkedUnevenCore`] yourself.
    #[inline]
    pub fn new_raw(times: impl Into<Vec<f32>>, values: impl Into<Vec<T>>) -> Self {
        Self {
            core: ChunkedUnevenCore {
                times: times.into(),
                values: values.into(),
            },
        }
    }
}

// Pie in the sky: `TranslationCurve` is basically the same thing as a `Box<dyn Curve<Vec3>>` etc.
// The first couple variants can be taken "off the shelf" from the Curve library, while the others
// are built on top of the core abstractions.

/// A curve specifying the translation component of a [`Transform`] in animation. The variants are
/// broken down by interpolation mode (with the exception of `Constant`, which never interpolates).
///
/// This type is, itself, a `Curve<Vec3>`, and it internally uses the provided sampling modes; each
/// variant "knows" its own interpolation mode.
#[derive(Clone, Debug, Reflect)]
pub enum TranslationCurve {
    /// A curve which takes a constant value over its domain. Notably, this is how animations with
    /// only a single keyframe are interpreted.
    Constant(ConstantCurve<Vec3>),

    /// A curve which interpolates linearly between keyframes.
    Linear(UnevenSampleAutoCurve<Vec3>),

    /// A curve which interpolates between keyframes in steps.
    Step(SteppedKeyframeCurve<Vec3>),

    /// A curve which interpolates between keyframes by using auxiliary tangent data to join
    /// adjacent keyframes with a cubic Hermite spline, which is then sampled.
    CubicSpline(CubicKeyframeCurve<Vec3>),
}

impl Curve<Vec3> for TranslationCurve {
    #[inline]
    fn domain(&self) -> Interval {
        match self {
            TranslationCurve::Constant(c) => c.domain(),
            TranslationCurve::Linear(c) => c.domain(),
            TranslationCurve::Step(c) => c.domain(),
            TranslationCurve::CubicSpline(c) => c.domain(),
        }
    }

    #[inline]
    fn sample(&self, t: f32) -> Vec3 {
        match self {
            TranslationCurve::Constant(c) => c.sample(t),
            TranslationCurve::Linear(c) => c.sample(t),
            TranslationCurve::Step(c) => c.sample(t),
            TranslationCurve::CubicSpline(c) => c.sample(t),
        }
    }
}

impl TranslationCurve {
    /// The time of the last keyframe for this animation curve. If the curve is constant, None
    /// is returned instead.
    #[inline]
    pub fn duration(&self) -> Option<f32> {
        match self {
            TranslationCurve::Constant(_) => None,
            TranslationCurve::Linear(c) => Some(c.domain().end()),
            TranslationCurve::Step(c) => Some(c.domain().end()),
            TranslationCurve::CubicSpline(c) => Some(c.domain().end()),
        }
    }
}

/// A curve specifying the scale component of a [`Transform`] in animation. The variants are
/// broken down by interpolation mode (with the exception of `Constant`, which never interpolates).
///
/// This type is, itself, a `Curve<Vec3>`, and it internally uses the provided sampling modes; each
/// variant "knows" its own interpolation mode.
#[derive(Clone, Debug, Reflect)]
pub enum ScaleCurve {
    /// A curve which takes a constant value over its domain. Notably, this is how animations with
    /// only a single keyframe are interpreted.
    Constant(ConstantCurve<Vec3>),

    /// A curve which interpolates linearly between keyframes.
    Linear(UnevenSampleAutoCurve<Vec3>),

    /// A curve which interpolates between keyframes in steps.
    Step(SteppedKeyframeCurve<Vec3>),

    /// A curve which interpolates between keyframes by using auxiliary tangent data to join
    /// adjacent keyframes with a cubic Hermite spline, which is then sampled.
    CubicSpline(CubicKeyframeCurve<Vec3>),
}

impl Curve<Vec3> for ScaleCurve {
    #[inline]
    fn domain(&self) -> Interval {
        match self {
            ScaleCurve::Constant(c) => c.domain(),
            ScaleCurve::Linear(c) => c.domain(),
            ScaleCurve::Step(c) => c.domain(),
            ScaleCurve::CubicSpline(c) => c.domain(),
        }
    }

    #[inline]
    fn sample(&self, t: f32) -> Vec3 {
        match self {
            ScaleCurve::Constant(c) => c.sample(t),
            ScaleCurve::Linear(c) => c.sample(t),
            ScaleCurve::Step(c) => c.sample(t),
            ScaleCurve::CubicSpline(c) => c.sample(t),
        }
    }
}

impl ScaleCurve {
    /// The time of the last keyframe for this animation curve. If the curve is constant, None
    /// is returned instead.
    #[inline]
    pub fn duration(&self) -> Option<f32> {
        match self {
            ScaleCurve::Constant(_) => None,
            ScaleCurve::Linear(c) => Some(c.domain().end()),
            ScaleCurve::Step(c) => Some(c.domain().end()),
            ScaleCurve::CubicSpline(c) => Some(c.domain().end()),
        }
    }
}

/// A curve specifying the scale component of a [`Transform`] in animation. The variants are
/// broken down by interpolation mode (with the exception of `Constant`, which never interpolates).
///
/// This type is, itself, a `Curve<Quat>`, and it internally uses the provided sampling modes; each
/// variant "knows" its own interpolation mode.
#[derive(Clone, Debug, Reflect)]
pub enum RotationCurve {
    /// A curve which takes a constant value over its domain. Notably, this is how animations with
    /// only a single keyframe are interpreted.
    Constant(ConstantCurve<Quat>),

    /// A curve which uses spherical linear interpolation between keyframes.
    SphericalLinear(UnevenSampleAutoCurve<Quat>),

    /// A curve which interpolates between keyframes in steps.
    Step(SteppedKeyframeCurve<Quat>),

    /// A curve which interpolates between keyframes by using auxiliary tangent data to join
    /// adjacent keyframes with a cubic Hermite spline. For quaternions, this means interpolating
    /// the underlying 4-vectors, sampling, and normalizing the result.
    CubicSpline(CubicKeyframeCurve<Vec4>),
}

impl Curve<Quat> for RotationCurve {
    #[inline]
    fn domain(&self) -> Interval {
        match self {
            RotationCurve::Constant(c) => c.domain(),
            RotationCurve::SphericalLinear(c) => c.domain(),
            RotationCurve::Step(c) => c.domain(),
            RotationCurve::CubicSpline(c) => c.domain(),
        }
    }

    #[inline]
    fn sample(&self, t: f32) -> Quat {
        match self {
            RotationCurve::Constant(c) => c.sample(t),
            RotationCurve::SphericalLinear(c) => c.sample(t),
            RotationCurve::Step(c) => c.sample(t),
            RotationCurve::CubicSpline(c) => c.map(|x| Quat::from_vec4(x).normalize()).sample(t),
        }
    }
}

impl RotationCurve {
    /// The time of the last keyframe for this animation curve. If the curve is constant, None
    /// is returned instead.
    #[inline]
    pub fn duration(&self) -> Option<f32> {
        match self {
            RotationCurve::Constant(_) => None,
            RotationCurve::SphericalLinear(c) => Some(c.domain().end()),
            RotationCurve::Step(c) => Some(c.domain().end()),
            RotationCurve::CubicSpline(c) => Some(c.domain().end()),
        }
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
    T: VectorSpace,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.core.domain()
    }

    #[inline]
    fn sample_iter<'a>(&self, t: f32) -> impl Iterator<Item = T>
    where
        Self: 'a,
    {
        match self.core.sample_betweenness(t) {
            Betweenness::Exact(v) | Betweenness::LeftTail(v) | Betweenness::RightTail(v) => {
                TwoIterators::Left(v.iter().copied())
            }

            Betweenness::Between(u, v, s) => {
                let interpolated = u.iter().zip(v.iter()).map(move |(x, y)| x.lerp(*y, s));
                TwoIterators::Right(interpolated)
            }
        }
    }
}

impl<T> WideLinearKeyframeCurve<T> {
    /// Create a new [`WideLinearKeyframeCurve`] from raw data, bypassing all checks. If you use this, you
    /// must uphold the invariants of [`ChunkedUnevenCore`] yourself.
    #[inline]
    pub fn new_raw(times: impl Into<Vec<f32>>, values: impl Into<Vec<T>>) -> Self {
        Self {
            core: ChunkedUnevenCore {
                times: times.into(),
                values: values.into(),
            },
        }
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
    fn sample_iter<'a>(&self, t: f32) -> impl Iterator<Item = T>
    where
        Self: 'a,
    {
        match self.core.sample_betweenness(t) {
            Betweenness::Exact(v) | Betweenness::LeftTail(v) | Betweenness::RightTail(v) => {
                TwoIterators::Left(v.iter().cloned())
            }

            Betweenness::Between(u, v, s) => {
                let interpolated =
                    u.iter()
                        .zip(v.iter())
                        .map(move |(x, y)| if s >= 1.0 { y.clone() } else { x.clone() });
                TwoIterators::Right(interpolated)
            }
        }
    }
}

impl<T> WideSteppedKeyframeCurve<T> {
    /// Create a new [`WideSteppedKeyframeCurve`] from raw data, bypassing all checks. If you use this, you
    /// must uphold the invariants of [`ChunkedUnevenCore`] yourself.
    #[inline]
    pub fn new_raw(times: impl Into<Vec<f32>>, values: impl Into<Vec<T>>) -> Self {
        Self {
            core: ChunkedUnevenCore {
                times: times.into(),
                values: values.into(),
            },
        }
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
    T: VectorSpace,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.core.domain()
    }

    #[inline]
    fn sample_iter<'a>(&self, t: f32) -> impl Iterator<Item = T>
    where
        Self: 'a,
    {
        match self.core.sample_betweenness_timed(t) {
            Betweenness::Exact((_, v))
            | Betweenness::LeftTail((_, v))
            | Betweenness::RightTail((_, v)) => {
                // Pick out the part of this that actually represents the position (instead of tangents),
                // which is the middle third.
                let width = self.core.width();
                TwoIterators::Left(v[width..(width * 2)].iter().copied())
            }

            Betweenness::Between((t0, u), (t1, v), s) => TwoIterators::Right(
                cubic_spline_interpolate_slices(self.core.width() / 3, u, v, s, t1 - t0),
            ),
        }
    }
}

impl<T> WideCubicKeyframeCurve<T> {
    /// Create a new [`WideCubicKeyframeCurve`] from raw data, bypassing all checks. If you use this, you
    /// must uphold the invariants of [`ChunkedUnevenCore`] yourself.
    #[inline]
    pub fn new_raw(times: impl Into<Vec<f32>>, values: impl Into<Vec<T>>) -> Self {
        Self {
            core: ChunkedUnevenCore {
                times: times.into(),
                values: values.into(),
            },
        }
    }
}

/// A curve specifying the [`MorphWeights`] for a mesh in animation. The variants are broken
/// down by interpolation mode (with the exception of `Constant`, which never interpolates).
///
/// This type is, itself, a `Curve<Vec<f32>>`; however, in order to avoid allocation, it is
/// recommended to use its implementation of the [`IterableCurve`] trait, which allows iterating
/// directly over information derived from the curve without allocating.
#[derive(Debug, Clone, Reflect)]
pub enum WeightsCurve {
    /// A curve which takes a constant value over its domain. Notably, this is how animations with
    /// only a single keyframe are interpreted.
    Constant(ConstantCurve<Vec<f32>>),

    /// A curve which interpolates weights linearly between keyframes.
    Linear(WideLinearKeyframeCurve<f32>),

    /// A curve which interpolates weights between keyframes in steps.
    Step(WideSteppedKeyframeCurve<f32>),

    /// A curve which interpolates between keyframes by using auxiliary tangent data to join
    /// adjacent keyframes with a cubic Hermite spline, which is then sampled.
    CubicSpline(WideCubicKeyframeCurve<f32>),
}

impl IterableCurve<f32> for WeightsCurve {
    #[inline]
    fn domain(&self) -> Interval {
        match self {
            WeightsCurve::Constant(c) => IterableCurve::domain(c),
            WeightsCurve::Linear(c) => c.domain(),
            WeightsCurve::Step(c) => c.domain(),
            WeightsCurve::CubicSpline(c) => c.domain(),
        }
    }

    #[inline]
    fn sample_iter<'a>(&self, t: f32) -> impl Iterator<Item = f32>
    where
        Self: 'a,
    {
        match self {
            WeightsCurve::Constant(c) => FourIterators::First(c.sample_iter(t)),
            WeightsCurve::Linear(c) => FourIterators::Second(c.sample_iter(t)),
            WeightsCurve::Step(c) => FourIterators::Third(c.sample_iter(t)),
            WeightsCurve::CubicSpline(c) => FourIterators::Fourth(c.sample_iter(t)),
        }
    }
}

impl WeightsCurve {
    /// The time of the last keyframe for this animation curve. If the curve is constant, None
    /// is returned instead.
    #[inline]
    pub fn duration(&self) -> Option<f32> {
        match self {
            WeightsCurve::Constant(_) => None,
            WeightsCurve::Linear(c) => Some(c.domain().end()),
            WeightsCurve::Step(c) => Some(c.domain().end()),
            WeightsCurve::CubicSpline(c) => Some(c.domain().end()),
        }
    }
}

/// A curve for animating either a the component of a [`Transform`] (translation, rotation, scale)
/// or the [`MorphWeights`] of morph targets for a mesh.
///
/// Each variant yields a [`Curve`] over the data that it parametrizes.
///
/// This follows the [glTF design].
/// [glTF design]: <https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#animations/>
#[derive(Debug, Clone, Reflect)]
pub enum VariableCurve {
    /// A [`TranslationCurve`] for animating the `translation` component of a [`Transform`].
    Translation(TranslationCurve),

    /// A [`RotationCurve`] for animating the `rotation` component of a [`Transform`].
    Rotation(RotationCurve),

    /// A [`ScaleCurve`] for animating the `scale` component of a [`Transform`].
    Scale(ScaleCurve),

    /// A [`WeightsCurve`] for animating [`MorphWeights`] of a mesh.
    Weights(WeightsCurve),
}

impl VariableCurve {
    /// The domain of this curve as an interval.
    #[inline]
    pub fn domain(&self) -> Interval {
        match self {
            VariableCurve::Translation(c) => c.domain(),
            VariableCurve::Rotation(c) => c.domain(),
            VariableCurve::Scale(c) => c.domain(),
            VariableCurve::Weights(c) => c.domain(),
        }
    }

    /// The time of the last keyframe for this animation curve. If the curve is constant, None
    /// is returned instead.
    #[inline]
    pub fn duration(&self) -> Option<f32> {
        match self {
            VariableCurve::Translation(c) => c.duration(),
            VariableCurve::Rotation(c) => c.duration(),
            VariableCurve::Scale(c) => c.duration(),
            VariableCurve::Weights(c) => c.duration(),
        }
    }
}

impl From<TranslationCurve> for VariableCurve {
    fn from(curve: TranslationCurve) -> Self {
        Self::Translation(curve)
    }
}

impl From<RotationCurve> for VariableCurve {
    fn from(curve: RotationCurve) -> Self {
        Self::Rotation(curve)
    }
}

impl From<ScaleCurve> for VariableCurve {
    fn from(curve: ScaleCurve) -> Self {
        Self::Scale(curve)
    }
}

impl From<WeightsCurve> for VariableCurve {
    fn from(curve: WeightsCurve) -> Self {
        Self::Weights(curve)
    }
}

//---------//
// HELPERS //
//---------//

enum TwoIterators<A, B> {
    Left(A),
    Right(B),
}

impl<A, B, T> Iterator for TwoIterators<A, B>
where
    A: Iterator<Item = T>,
    B: Iterator<Item = T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            TwoIterators::Left(a) => a.next(),
            TwoIterators::Right(b) => b.next(),
        }
    }
}

enum FourIterators<A, B, C, D> {
    First(A),
    Second(B),
    Third(C),
    Fourth(D),
}

impl<A, B, C, D, T> Iterator for FourIterators<A, B, C, D>
where
    A: Iterator<Item = T>,
    B: Iterator<Item = T>,
    C: Iterator<Item = T>,
    D: Iterator<Item = T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            FourIterators::First(a) => a.next(),
            FourIterators::Second(b) => b.next(),
            FourIterators::Third(c) => c.next(),
            FourIterators::Fourth(d) => d.next(),
        }
    }
}

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
    T: VectorSpace,
{
    value_start * (2.0 * lerp.powi(3) - 3.0 * lerp.powi(2) + 1.0)
        + tangent_out_start * (step_duration) * (lerp.powi(3) - 2.0 * lerp.powi(2) + lerp)
        + value_end * (-2.0 * lerp.powi(3) + 3.0 * lerp.powi(2))
        + tangent_in_end * step_duration * (lerp.powi(3) - lerp.powi(2))
}

fn cubic_spline_interpolate_slices<'a, T: VectorSpace>(
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
            second[idx + width],
            second[idx],
            s,
            step_between,
        )
    })
}
