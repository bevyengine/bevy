use crate::cubic_spline_interpolation;
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

/// A keyframe-defined curve that uses linear interpolation for [`VectorSpace`] types.
#[derive(Debug, Clone, Reflect)]
pub struct LinearKeyframeCurve<T> {
    core: UnevenCore<T>,
}

impl<V> Curve<V> for LinearKeyframeCurve<V>
where
    V: VectorSpace,
{
    fn domain(&self) -> Interval {
        self.core.domain()
    }

    fn sample(&self, t: f32) -> V {
        self.core.sample_with(t, |x, y, t| x.lerp(*y, t))
    }
}

/// A keyframe-defined curve that uses cubic spline interpolation, backed by a contiguous buffer.
#[derive(Debug, Clone, Reflect)]
pub struct CubicSplineKeyframeCurve<T> {
    // Note: the sample width here should be 3.
    core: ChunkedUnevenCore<T>,
}

impl<V> Curve<V> for CubicSplineKeyframeCurve<V>
where
    V: VectorSpace,
{
    fn domain(&self) -> Interval {
        self.core.domain()
    }

    fn sample(&self, t: f32) -> V {
        match self.core.sample_betweenness(t) {
            // In all the cases where only one frame matters, defer to the position within it.
            Betweenness::Exact(v) => v[1],
            Betweenness::LeftTail(v) => v[1],
            Betweenness::RightTail(v) => v[1],
            Betweenness::Between(u, v, s) => {
                cubic_spline_interpolation(u[1], u[2], v[0], v[1], s, todo!())
            }
        }
    }
}

// Pie in the sky: `TranslationCurve` is basically the same thing as a `Box<dyn Curve<Vec3>>` etc.

/// A curve specifying the translation component of a [`Transform`] in animation. The variants are
/// broken down by interpolation mode (with the exception of `Constant`, which never interpolates).
///
/// This type is, itself, a `Curve<Vec3>`, and it internally uses the provided sampling modes; each
/// variant "knows" its own sampling type
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
    CubicSpline(CubicSplineKeyframeCurve<Vec3>),
}

impl Curve<Vec3> for TranslationCurve {
    fn domain(&self) -> Interval {
        match self {
            TranslationCurve::Constant(c) => c.domain(),
            TranslationCurve::Linear(c) => c.domain(),
            TranslationCurve::Step(c) => c.domain(),
            TranslationCurve::CubicSpline(c) => c.domain(),
        }
    }

    fn sample(&self, t: f32) -> Vec3 {
        match self {
            TranslationCurve::Constant(c) => c.sample(t),
            TranslationCurve::Linear(c) => c.sample(t),
            TranslationCurve::Step(c) => c.sample(t),
            TranslationCurve::CubicSpline(c) => c.sample(t),
        }
    }
}

/// A curve specifying the scale component of a [`Transform`] in animation. The variants are
/// broken down by interpolation mode (with the exception of `Constant`, which never interpolates).
///
/// This type is, itself, a `Curve<Vec3>`, and it internally uses the provided sampling modes;
/// however, linear interpolation is intrinsic to `Vec3` itself, so the interpolation metadata
/// itself will be lost if the curve is resampled. On the other hand, the variant curves each
/// properly know their own modes of interpolation.
#[derive(Clone, Debug, Reflect)]
pub enum ScaleCurve {
    /// A curve which takes a constant value over its domain. Notably, this is how animations with
    /// only a single keyframe are interpreted.
    Constant(ConstantCurve<Vec3>),

    /// A curve which interpolates linearly between keyframes.
    Linear(LinearKeyframeCurve<Vec3>),

    /// A curve which interpolates between keyframes in steps.
    Step(SteppedKeyframeCurve<Vec3>),

    /// A curve which interpolates between keyframes by using auxiliary tangent data to join
    /// adjacent keyframes with a cubic Hermite spline, which is then sampled.
    CubicSpline(CubicSplineKeyframeCurve<Vec3>),
}

impl Curve<Vec3> for ScaleCurve {
    fn domain(&self) -> Interval {
        match self {
            ScaleCurve::Constant(c) => c.domain(),
            ScaleCurve::Linear(c) => c.domain(),
            ScaleCurve::Step(c) => c.domain(),
            ScaleCurve::CubicSpline(c) => c.domain(),
        }
    }

    fn sample(&self, t: f32) -> Vec3 {
        match self {
            ScaleCurve::Constant(c) => c.sample(t),
            ScaleCurve::Linear(c) => c.sample(t),
            ScaleCurve::Step(c) => c.sample(t),
            ScaleCurve::CubicSpline(c) => c.sample(t),
        }
    }
}

/// A curve specifying the scale component of a [`Transform`] in animation. The variants are
/// broken down by interpolation mode (with the exception of `Constant`, which never interpolates).
///
/// This type is, itself, a `Curve<Quat>`, and it internally uses the provided sampling modes;
/// however, spherical linear interpolation is intrinsic to `Vec3` itself, so the interpolation
/// metadata itself will be lost if the curve is resampled. On the other hand, the variant curves each
/// properly know their own modes of interpolation.
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
    CubicSpline(CubicSplineKeyframeCurve<Vec4>),
}

impl Curve<Quat> for RotationCurve {
    fn domain(&self) -> Interval {
        match self {
            RotationCurve::Constant(c) => c.domain(),
            RotationCurve::SphericalLinear(c) => c.domain(),
            RotationCurve::Step(c) => c.domain(),
            RotationCurve::CubicSpline(c) => c.domain(),
        }
    }

    fn sample(&self, t: f32) -> Quat {
        match self {
            RotationCurve::Constant(c) => c.sample(t),
            RotationCurve::SphericalLinear(c) => c.sample(t),
            RotationCurve::Step(c) => c.sample(t),
            RotationCurve::CubicSpline(c) => c.map(|x| Quat::from_vec4(x).normalize()).sample(t),
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

    /// A curve which interpolates linearly between keyframes.
    Linear(DynamicArrayCurve<f32>),

    /// A curve which interpolates between keyframes in steps.
    Step(DynamicArrayCurve<Stepped<f32>>),

    /// A curve which interpolates between keyframes by using auxiliary tangent data to join
    /// adjacent keyframes with a cubic Hermite spline, which is then sampled.
    CubicSpline(DynamicArrayCurve<TwoSidedHermite<f32>>),
}

impl IterableCurve<f32> for WeightsCurve {
    fn domain(&self) -> Interval {
        match self {
            WeightsCurve::Constant(c) => IterableCurve::domain(c),
            WeightsCurve::Linear(c) => c.domain(),
            WeightsCurve::Step(c) => c.domain(),
            WeightsCurve::CubicSpline(c) => c.domain(),
        }
    }

    fn sample_iter<'a>(&self, t: f32) -> impl Iterator<Item = f32>
    where
        Self: 'a,
    {
        match self {
            WeightsCurve::Constant(c) => QuaternaryIteratorDisjunction::First(c.sample_iter(t)),
            WeightsCurve::Linear(c) => QuaternaryIteratorDisjunction::Second(c.sample_iter(t)),
            WeightsCurve::Step(c) => {
                QuaternaryIteratorDisjunction::Third(c.sample_iter(t).map(|v| v.0))
            }
            WeightsCurve::CubicSpline(c) => {
                QuaternaryIteratorDisjunction::Fourth(c.sample_iter(t).map(|v| v.point))
            }
        }
    }
}

impl Curve<Vec<f32>> for WeightsCurve {
    fn domain(&self) -> Interval {
        IterableCurve::domain(self)
    }

    fn sample(&self, t: f32) -> Vec<f32> {
        self.sample_iter(t).collect()
    }
}

/// A curve for animating either a the component of a [`Transform`] (translation, rotation, scale)
/// or the [`MorphWeights`] of morph targets for a mesh.
///
/// Each variant yields a [`Curve`] over the data that it parametrizes.
//#[derive(Reflect, Clone, Debug)]
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

//--------------//
// HELPER STUFF //
//--------------//

enum IteratorDisjunction<A, B, T>
where
    A: Iterator<Item = T>,
    B: Iterator<Item = T>,
{
    Left(A),
    Right(B),
}

impl<A, B, T> Iterator for IteratorDisjunction<A, B, T>
where
    A: Iterator<Item = T>,
    B: Iterator<Item = T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            IteratorDisjunction::Left(a) => a.next(),
            IteratorDisjunction::Right(b) => b.next(),
        }
    }
}

enum QuaternaryIteratorDisjunction<A, B, C, D, T>
where
    A: Iterator<Item = T>,
    B: Iterator<Item = T>,
    C: Iterator<Item = T>,
    D: Iterator<Item = T>,
{
    First(A),
    Second(B),
    Third(C),
    Fourth(D),
}

impl<A, B, C, D, T> Iterator for QuaternaryIteratorDisjunction<A, B, C, D, T>
where
    A: Iterator<Item = T>,
    B: Iterator<Item = T>,
    C: Iterator<Item = T>,
    D: Iterator<Item = T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            QuaternaryIteratorDisjunction::First(a) => a.next(),
            QuaternaryIteratorDisjunction::Second(b) => b.next(),
            QuaternaryIteratorDisjunction::Third(c) => c.next(),
            QuaternaryIteratorDisjunction::Fourth(d) => d.next(),
        }
    }
}
