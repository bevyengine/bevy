use std::marker::PhantomData;

use bevy_math::{
    cubic_splines::{CubicGenerator, CubicHermite},
    curve::{interval, ConstantCurve, Curve, Interpolable, Interval, MapCurve, UnevenSampleCurve},
    FloatExt, Quat, Vec3, Vec4, VectorSpace,
};

/// A wrapper struct that gives the enclosed type the property of being [`Interpolable`] with
/// naïve step interpolation. `self.interpolate(other, t)` is such that `self` is returned when
/// `t` is less than `1.0`, while `other` is returned for values `1.0` and greater.
#[derive(Clone, Copy, Default)]
pub struct Stepped<T: Clone>(pub T)
where
    T: Clone;

impl<T: Clone> Interpolable for Stepped<T> {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        if t < 1.0 {
            self.clone()
        } else {
            other.clone()
        }
    }
}

/// A struct that wraps a vector space type together with data needed for cubic spline (Hermite)
/// interpolation. The resulting type is [`Interpolable`], with the interior position and velocity
/// between adjacent points determined by the Hermite spline connecting them.
///
/// Note that outside of the interval `[0, 1]`, this uses global extrapolation based on the
/// out-tangent of the left-hand point and the in-tangent of the right-hand point.
#[derive(Clone, Copy, Default)]
pub struct TwoSidedHermite<V: VectorSpace> {
    /// The position of the datum in space.
    pub point: V,

    /// The incoming tangent vector used for interpolation.
    pub in_tangent: V,

    /// The outgoing tangent vector used for interpolation.
    pub out_tangent: V,
}

impl<V> Interpolable for TwoSidedHermite<V>
where
    V: VectorSpace,
{
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        let control_points = [self.point, other.point];
        let tangents = [self.out_tangent, other.in_tangent];
        // We should probably have a better way of constructing these...
        let curve_segment = CubicHermite::new(control_points, tangents)
            .to_curve()
            .segments()[0];
        // (For interior points, the in-tangents and out-tangents are just the tangent.)
        Self {
            point: curve_segment.position(t),
            in_tangent: curve_segment.velocity(t),
            out_tangent: curve_segment.velocity(t),
        }
    }
}

// Pie in the sky: `TranslationCurve` is basically the same thing as a `Box<dyn Curve<Vec3>>` etc.

/// A curve specifying the translation component of a [`Transform`] in animation. The variants are
/// broken down by interpolation mode (with the exception of `Constant`, which never interpolates).
///
/// This type is, itself, a `Curve<Vec3>`, and it internally uses the provided sampling modes;
/// however, linear interpolation is intrinsic to `Vec3` itself, so the interpolation metadata
/// itself will be lost if the curve is resampled. On the other hand, the variant curves each
/// properly know their own modes of interpolation.
pub enum TranslationCurve {
    /// A curve which takes a constant value over its domain. Notably, this is how animations with
    /// only a single keyframe are interpreted.
    Constant(ConstantCurve<Vec3>),

    /// A curve which interpolates linearly between keyframes.
    Linear(UnevenSampleCurve<Vec3>),

    /// A curve which interpolates between keyframes in steps.
    Step(UnevenSampleCurve<Stepped<Vec3>>),

    /// A curve which interpolates between keyframes by using auxiliary tangent data to join
    /// adjacent keyframes with a cubic Hermite spline, which is then sampled.
    CubicSpline(UnevenSampleCurve<TwoSidedHermite<Vec3>>),
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
            TranslationCurve::Step(c) => c.sample(t).0,
            TranslationCurve::CubicSpline(c) => c.map(|x| x.point).sample(t),
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
pub enum ScaleCurve {
    /// A curve which takes a constant value over its domain. Notably, this is how animations with
    /// only a single keyframe are interpreted.
    Constant(ConstantCurve<Vec3>),

    /// A curve which interpolates linearly between keyframes.
    Linear(UnevenSampleCurve<Vec3>),

    /// A curve which interpolates between keyframes in steps.
    Step(UnevenSampleCurve<Stepped<Vec3>>),

    /// A curve which interpolates between keyframes by using auxiliary tangent data to join
    /// adjacent keyframes with a cubic Hermite spline, which is then sampled.
    CubicSpline(UnevenSampleCurve<TwoSidedHermite<Vec3>>),
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
            ScaleCurve::Step(c) => c.map(|x| x.0).sample(t),
            ScaleCurve::CubicSpline(c) => c.map(|x| x.point).sample(t),
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
pub enum RotationCurve {
    /// A curve which takes a constant value over its domain. Notably, this is how animations with
    /// only a single keyframe are interpreted.
    Constant(ConstantCurve<Quat>),

    /// A curve which uses spherical linear interpolation between keyframes.
    SphericalLinear(UnevenSampleCurve<Quat>),

    /// A curve which interpolates between keyframes in steps.
    Step(UnevenSampleCurve<Stepped<Quat>>),

    /// A curve which interpolates between keyframes by using auxiliary tangent data to join
    /// adjacent keyframes with a cubic Hermite spline. For quaternions, this means interpolating
    /// the underlying 4-vectors, sampling, and normalizing the result.
    CubicSpline(UnevenSampleCurve<TwoSidedHermite<Vec4>>),
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
            RotationCurve::Step(c) => c.map(|x| x.0).sample(t),
            RotationCurve::CubicSpline(c) => {
                c.map(|x| Quat::from_vec4(x.point).normalize()).sample(t)
            }
        }
    }
}
/// A curve specifying the [`MorphWeights`] for a mesh in animation. The variants are broken
/// down by interpolation mode (with the exception of `Constant`, which never interpolates).
///
/// This type is, itself, a `Curve<Vec<f32>>`; however, in order to avoid allocation, it is
/// recommended to use its implementation of the [`MultiCurve`] subtrait, which allows dumping
/// cross-channel sample data into an external buffer, avoiding allocation.
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

impl Curve<Vec<f32>> for WeightsCurve {
    fn domain(&self) -> Interval {
        match self {
            WeightsCurve::Constant(c) => c.domain(),
            WeightsCurve::Linear(c) => c.domain(),
            WeightsCurve::Step(c) => c.domain(),
            WeightsCurve::CubicSpline(c) => c.domain(),
        }
    }

    fn sample(&self, t: f32) -> Vec<f32> {
        match self {
            WeightsCurve::Constant(c) => c.sample(t),
            WeightsCurve::Linear(c) => c.sample(t),
            WeightsCurve::Step(c) => c.map(|v| v.into_iter().map(|x| x.0).collect()).sample(t),
            WeightsCurve::CubicSpline(c) => c
                .map(|v| v.into_iter().map(|x| x.point).collect())
                .sample(t),
        }
    }
}

impl MultiCurve<f32> for WeightsCurve {
    fn map_sample_into<S>(&self, t: f32, buffer: &mut [S], f: &impl Fn(f32) -> S)
    where
        S: Interpolable,
    {
        match self {
            WeightsCurve::Constant(c) => todo!(),
            WeightsCurve::Linear(c) => todo!(),
            WeightsCurve::Step(c) => todo!(),
            WeightsCurve::CubicSpline(c) => todo!(),
        }
    }
}

/// A curve for animating either a the component of a [`Transform`] (translation, rotation, scale)
/// or the [`MorphWeights`] of morph targets for a mesh.
///
/// Each variant yields a [`Curve`] over the data that it parametrizes.
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
// EXPERIMENTAL //
//--------------//

/// A trait for a curve that takes many interpolable values simultaneously, providing a function
/// to place those values into a buffer rather than allocating while sampling.
pub trait MultiCurve<T>: Curve<Vec<T>>
where
    T: Interpolable,
{
    /// Sample a number of simultaneous values from this curve into a buffer.
    fn sample_into(&self, t: f32, buffer: &mut [T]) {
        self.map_sample_into(t, buffer, &|x| x)
    }

    /// Map the collection of samples by `f` before putting them into the given buffer.
    fn map_sample_into<S>(&self, t: f32, buffer: &mut [S], f: &impl Fn(T) -> S)
    where
        S: Interpolable;
}

pub struct MultiMapCurve<S, T, C, F>
where
    S: Interpolable,
    T: Interpolable,
    C: MultiCurve<S>,
    F: Fn(S) -> T,
{
    preimage: C,
    f: F,
    _phantom: PhantomData<(S, T)>,
}

impl<S, T, C, F> Curve<Vec<T>> for MultiMapCurve<S, T, C, F>
where
    S: Interpolable,
    T: Interpolable,
    C: MultiCurve<S>,
    F: Fn(S) -> T,
{
    fn domain(&self) -> Interval {
        self.preimage.domain()
    }

    fn sample(&self, t: f32) -> Vec<T> {
        self.preimage
            .sample(t)
            .into_iter()
            .map(|x| (self.f)(x))
            .collect()
    }
}

impl<S, T, C, F> MultiCurve<T> for MultiMapCurve<S, T, C, F>
where
    S: Interpolable,
    T: Interpolable,
    C: MultiCurve<S>,
    F: Fn(S) -> T,
{
    fn sample_into(&self, t: f32, buffer: &mut [T]) {
        self.preimage.map_sample_into(t, buffer, &self.f);
    }

    fn map_sample_into<R>(&self, t: f32, buffer: &mut [R], g: &impl Fn(T) -> R)
    where
        R: Interpolable,
    {
        let gf = |x| g((self.f)(x));
        self.preimage.map_sample_into(t, buffer, &gf);
    }
}

/// A curve data structure which holds data for a list of keyframes in a number of distinct
/// "channels" equal to its `width`. This is sampled through `sample_into`, which places the data
/// into an external buffer. If `T: Default`, this may also be used as a `Curve` directly, but a new
/// `Vec<T>` will be allocated for each call, which may be undesirable.
pub struct DynamicArrayCurve<T>
where
    T: Interpolable,
{
    /// The times at which the keyframes are placed. There must be at least two of these, and they
    /// must always be sorted in increasing order.
    times: Vec<f32>,

    /// The output values. These are stored in multiples of the `width`, with each `width` of
    /// indices corresponding to the outputs for a single keyframe. For instance, the indices
    /// `0..self.width` in `values` correspond to the different channels for keyframe `0`.
    ///
    /// The length of this vector must always be `width` times that of `times`.
    values: Vec<T>,

    /// The number of channels that this data structure maintains, and therefore the factor between
    /// the length of `times` and that of `values`. Must be at least `1`.
    width: usize,
}

/// An error that indicates that a [`DynamicArrayCurve`] could not be formed.
#[derive(Debug, Clone, Copy)]
pub struct DynamicArrayError;

impl<T> DynamicArrayCurve<T>
where
    T: Interpolable,
{
    /// Create a new [`DynamicArrayCurve`]. Produces an error in any of the following circumstances:
    /// * `width` is zero.
    /// * `times` has a length less than `2`.
    /// * `values` has the incorrect length relative to `times`.
    pub fn new(
        times: impl Into<Vec<f32>>,
        values: impl Into<Vec<T>>,
        width: usize,
    ) -> Result<Self, DynamicArrayError> {
        let times: Vec<f32> = times.into();
        let values: Vec<T> = values.into();

        if width == 0 {
            return Err(DynamicArrayError);
        }
        if times.len() < 2 {
            return Err(DynamicArrayError);
        }
        if values.len() != times.len() * width {
            return Err(DynamicArrayError);
        }

        Ok(Self {
            times,
            values,
            width,
        })
    }

    fn find_keyframe(&self, t: f32) -> Option<usize> {
        match self
            .times
            .binary_search_by(|pt| pt.partial_cmp(&t).unwrap())
        {
            Ok(index) => {
                if index >= self.times.len() - 1 {
                    // This is the index of the last keyframe
                    None
                } else {
                    // Exact match that is not the last keyframe
                    Some(index)
                }
            }
            Err(index) => {
                if index == 0 {
                    // This is before the first keyframe
                    None
                } else if index >= self.times.len() {
                    // This is after the last keyframe
                    None
                } else {
                    // This is actually in the middle somewhere; we subtract 1 to return the index
                    // of the lower of the two keyframes
                    Some(index - 1)
                }
            }
        }
    }

    /// The width for this curve; i.e., the number of distinct channels sampled for each keyframe.
    pub fn width(&self) -> usize {
        self.width
    }

    /// The interval which spans between the first and last sample times.
    fn domain(&self) -> Interval {
        let start = self.times.first().unwrap();
        let end = self.times.last().unwrap();
        interval(*start, *end).unwrap()
    }

    /// Sample the interpolated data at time `t` into a given `buffer`.
    ///
    /// # Panics
    /// Panics if the provided buffer is not at least as large as `width`.
    pub fn sample_into(&self, t: f32, buffer: &mut [T]) {
        assert!(buffer.len() >= self.width);

        let t = self.domain().clamp(t);

        let Some(lower_index) = self.find_keyframe(t) else {
            // After clamping, `find_keyframe` will only return None if we landed on the
            // last keyframe.
            let index = self.times.len() - 1;

            // Jump to where the values for the last keyframe are:
            let morph_index = index * self.width;

            // Copy the values for the last keyframe into the buffer:
            for offset in 0..self.width {
                buffer[offset] = self.values[morph_index + offset].clone();
            }

            return;
        };

        // Get the adjacent timestamps and the lerp parameter of `t` between them:
        let upper_index = lower_index + 1;
        let lower_timestamp = self.times[lower_index];
        let upper_timestamp = self.times[upper_index];
        let lerp_param = f32::inverse_lerp(lower_timestamp, upper_timestamp, t);

        // The indices in `self.values` where the values actually start:
        let lower_morph_index = lower_index * self.width;
        let upper_morph_index = upper_index * self.width;

        // Interpolate and dump the results into the given buffer:
        for offset in 0..self.width {
            let lower_value = &self.values[lower_morph_index + offset];
            let upper_value = &self.values[upper_morph_index + offset];
            buffer[offset] = lower_value.interpolate(upper_value, lerp_param);
        }
    }

    /// Sample the interpolated data at time `t` into a given `buffer` after mapping it through
    /// a function `f`.
    ///
    /// # Panics
    /// Panics if the provided buffer is not at least as large as `width`.
    pub fn map_sample_into<S>(&self, t: f32, buffer: &mut [S], f: &impl Fn(T) -> S)
    where
        S: Interpolable,
    {
        assert!(buffer.len() >= self.width);

        let t = self.domain().clamp(t);

        let Some(lower_index) = self.find_keyframe(t) else {
            // After clamping, `find_keyframe` will only return None if we landed on the
            // last keyframe.
            let index = self.times.len() - 1;

            // Jump to where the values for the last keyframe are:
            let morph_index = index * self.width;

            // Copy the values for the last keyframe into the buffer:
            for offset in 0..self.width {
                buffer[offset] = f(self.values[morph_index + offset].clone());
            }

            return;
        };

        // Get the adjacent timestamps and the lerp parameter of `t` between them:
        let upper_index = lower_index + 1;
        let lower_timestamp = self.times[lower_index];
        let upper_timestamp = self.times[upper_index];
        let lerp_param = f32::inverse_lerp(lower_timestamp, upper_timestamp, t);

        // The indices in `self.values` where the values actually start:
        let lower_morph_index = lower_index * self.width;
        let upper_morph_index = upper_index * self.width;

        // Interpolate and dump the results into the given buffer:
        for offset in 0..self.width {
            let lower_value = &self.values[lower_morph_index + offset];
            let upper_value = &self.values[upper_morph_index + offset];
            buffer[offset] = f(lower_value.interpolate(upper_value, lerp_param));
        }
    }
}

// Note that the `sample` function always allocates its output, whereas `sample_into` can dump
// the sample data into an external buffer, bypassing the need to allocate.

impl<T> Curve<Vec<T>> for DynamicArrayCurve<T>
where
    T: Interpolable + Default,
{
    fn domain(&self) -> Interval {
        self.domain()
    }

    fn sample(&self, t: f32) -> Vec<T> {
        let mut output: Vec<T> = vec![<T as Default>::default(); self.width];
        self.sample_into(t, output.as_mut_slice());
        output
    }
}

impl<T> MultiCurve<T> for DynamicArrayCurve<T>
where
    T: Interpolable + Default,
{
    fn sample_into(&self, t: f32, buffer: &mut [T]) {
        self.sample_into(t, buffer);
    }

    fn map_sample_into<S>(&self, t: f32, buffer: &mut [S], f: &impl Fn(T) -> S)
    where
        S: Interpolable,
    {
        self.map_sample_into(t, buffer, f)
    }
}
