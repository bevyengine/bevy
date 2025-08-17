//! Module containing different easing functions.
//!
//! An easing function is a [`Curve`] that's used to transition between two
//! values. It takes a time parameter, where a time of zero means the start of
//! the transition and a time of one means the end.
//!
//! Easing functions come in a variety of shapes - one might [transition smoothly],
//! while another might have a [bouncing motion].
//!
//! There are several ways to use easing functions. The simplest option is a
//! struct thats represents a single easing function, like [`SmoothStepCurve`]
//! and [`StepsCurve`]. These structs can only transition from a value of zero
//! to a value of one.
//!
//! ```
//! # use bevy_math::prelude::*;
//! # let time = 0.0;
//! let smoothed_value = SmoothStepCurve.sample(time);
//! ```
//!
//! ```
//! # use bevy_math::prelude::*;
//! # let time = 0.0;
//! let stepped_value = StepsCurve(5, JumpAt::Start).sample(time);
//! ```
//!
//! Another option is [`EaseFunction`]. Unlike the single function structs,
//! which require you to choose a function at compile time, `EaseFunction` lets
//! you choose at runtime. It can also be serialized.
//!
//! ```
//! # use bevy_math::prelude::*;
//! # let time = 0.0;
//! # let make_it_smooth = false;
//! let mut curve = EaseFunction::Linear;
//!
//! if make_it_smooth {
//!     curve = EaseFunction::SmoothStep;
//! }
//!
//! let value = curve.sample(time);
//! ```
//!
//! The final option is [`EasingCurve`]. This lets you transition between any
//! two values - not just zero to one. `EasingCurve` can use any value that
//! implements the [`Ease`] trait, including vectors and directions.
//!
//! ```
//! # use bevy_math::prelude::*;
//! # let time = 0.0;
//! // Make a curve that smoothly transitions between two positions.
//! let start_position = vec2(1.0, 2.0);
//! let end_position = vec2(5.0, 10.0);
//! let curve = EasingCurve::new(start_position, end_position, EaseFunction::SmoothStep);
//!
//! let smoothed_position = curve.sample(time);
//! ```
//!
//! Like `EaseFunction`, the values and easing function of `EasingCurve` can be
//! chosen at runtime and serialized.
//!
//! [transition smoothly]: `SmoothStepCurve`
//! [bouncing motion]: `BounceInCurve`
//! [`sample`]: `Curve::sample`
//! [`sample_clamped`]: `Curve::sample_clamped`
//! [`sample_unchecked`]: `Curve::sample_unchecked`
//!

use crate::{
    curve::{Curve, CurveExt, FunctionCurve, Interval},
    Dir2, Dir3, Dir3A, Isometry2d, Isometry3d, Quat, Rot2, VectorSpace,
};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::std_traits::ReflectDefault;

use variadics_please::all_tuples_enumerated;

// TODO: Think about merging `Ease` with `StableInterpolate`

/// A type whose values can be eased between.
///
/// This requires the construction of an interpolation curve that actually extends
/// beyond the curve segment that connects two values, because an easing curve may
/// extrapolate before the starting value and after the ending value. This is
/// especially common in easing functions that mimic elastic or springlike behavior.
pub trait Ease: Sized {
    /// Given `start` and `end` values, produce a curve with [unlimited domain]
    /// that:
    /// - takes a value equivalent to `start` at `t = 0`
    /// - takes a value equivalent to `end` at `t = 1`
    /// - has constant speed everywhere, including outside of `[0, 1]`
    ///
    /// [unlimited domain]: Interval::EVERYWHERE
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self>;
}

impl<V: VectorSpace<Scalar = f32>> Ease for V {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::EVERYWHERE, move |t| V::lerp(start, end, t))
    }
}

impl Ease for Rot2 {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::EVERYWHERE, move |t| Rot2::slerp(start, end, t))
    }
}

impl Ease for Quat {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        let dot = start.dot(end);
        let end_adjusted = if dot < 0.0 { -end } else { end };
        let difference = end_adjusted * start.inverse();
        let (axis, angle) = difference.to_axis_angle();
        FunctionCurve::new(Interval::EVERYWHERE, move |s| {
            Quat::from_axis_angle(axis, angle * s) * start
        })
    }
}

impl Ease for Dir2 {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::EVERYWHERE, move |t| Dir2::slerp(start, end, t))
    }
}

impl Ease for Dir3 {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        let difference_quat = Quat::from_rotation_arc(start.as_vec3(), end.as_vec3());
        Quat::interpolating_curve_unbounded(Quat::IDENTITY, difference_quat).map(move |q| q * start)
    }
}

impl Ease for Dir3A {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        let difference_quat =
            Quat::from_rotation_arc(start.as_vec3a().into(), end.as_vec3a().into());
        Quat::interpolating_curve_unbounded(Quat::IDENTITY, difference_quat).map(move |q| q * start)
    }
}

impl Ease for Isometry3d {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::EVERYWHERE, move |t| {
            // we can use sample_unchecked here, since both interpolating_curve_unbounded impls
            // used are defined on the whole domain
            Isometry3d {
                rotation: Quat::interpolating_curve_unbounded(start.rotation, end.rotation)
                    .sample_unchecked(t),
                translation: crate::Vec3A::interpolating_curve_unbounded(
                    start.translation,
                    end.translation,
                )
                .sample_unchecked(t),
            }
        })
    }
}

impl Ease for Isometry2d {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::EVERYWHERE, move |t| {
            // we can use sample_unchecked here, since both interpolating_curve_unbounded impls
            // used are defined on the whole domain
            Isometry2d {
                rotation: Rot2::interpolating_curve_unbounded(start.rotation, end.rotation)
                    .sample_unchecked(t),
                translation: crate::Vec2::interpolating_curve_unbounded(
                    start.translation,
                    end.translation,
                )
                .sample_unchecked(t),
            }
        })
    }
}

macro_rules! impl_ease_tuple {
    ($(#[$meta:meta])* $(($n:tt, $T:ident)),*) => {
        $(#[$meta])*
        impl<$($T: Ease),*> Ease for ($($T,)*) {
            fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
                let curve_tuple =
                (
                    $(
                        <$T as Ease>::interpolating_curve_unbounded(start.$n, end.$n),
                    )*
                );

                FunctionCurve::new(Interval::EVERYWHERE, move |t|
                    (
                        $(
                            curve_tuple.$n.sample_unchecked(t),
                        )*
                    )
                )
            }
        }
    };
}

all_tuples_enumerated!(
    #[doc(fake_variadic)]
    impl_ease_tuple,
    1,
    11,
    T
);

/// A [`Curve`] that is defined by
///
/// - an initial `start` sample value at `t = 0`
/// - a final `end` sample value at `t = 1`
/// - an [easing function] to interpolate between the two values.
///
/// The resulting curve's domain is always [the unit interval].
///
/// # Example
///
/// Create a linear curve that interpolates between `2.0` and `4.0`.
///
/// ```
/// # use bevy_math::prelude::*;
/// let c = EasingCurve::new(2.0, 4.0, EaseFunction::Linear);
/// ```
///
/// [`sample`] the curve at various points. This will return `None` if the parameter
/// is outside the unit interval.
///
/// ```
/// # use bevy_math::prelude::*;
/// # let c = EasingCurve::new(2.0, 4.0, EaseFunction::Linear);
/// assert_eq!(c.sample(-1.0), None);
/// assert_eq!(c.sample(0.0), Some(2.0));
/// assert_eq!(c.sample(0.5), Some(3.0));
/// assert_eq!(c.sample(1.0), Some(4.0));
/// assert_eq!(c.sample(2.0), None);
/// ```
///
/// [`sample_clamped`] will clamp the parameter to the unit interval, so it
/// always returns a value.
///
/// ```
/// # use bevy_math::prelude::*;
/// # let c = EasingCurve::new(2.0, 4.0, EaseFunction::Linear);
/// assert_eq!(c.sample_clamped(-1.0), 2.0);
/// assert_eq!(c.sample_clamped(0.0), 2.0);
/// assert_eq!(c.sample_clamped(0.5), 3.0);
/// assert_eq!(c.sample_clamped(1.0), 4.0);
/// assert_eq!(c.sample_clamped(2.0), 4.0);
/// ```
///
/// `EasingCurve` can be used with any type that implements the [`Ease`] trait.
/// This includes many math types, like vectors and rotations.
///
/// ```
/// # use bevy_math::prelude::*;
/// let c = EasingCurve::new(
///     Vec2::new(0.0, 4.0),
///     Vec2::new(2.0, 8.0),
///     EaseFunction::Linear,
/// );
///
/// assert_eq!(c.sample_clamped(0.5), Vec2::new(1.0, 6.0));
/// ```
///
/// ```
/// # use bevy_math::prelude::*;
/// # use approx::assert_abs_diff_eq;
/// let c = EasingCurve::new(
///     Rot2::degrees(10.0),
///     Rot2::degrees(20.0),
///     EaseFunction::Linear,
/// );
///
/// assert_abs_diff_eq!(c.sample_clamped(0.5), Rot2::degrees(15.0));
/// ```
///
/// As a shortcut, an `EasingCurve` between `0.0` and `1.0` can be replaced by
/// [`EaseFunction`].
///
/// ```
/// # use bevy_math::prelude::*;
/// # let t = 0.5;
/// let f = EaseFunction::SineIn;
/// let c = EasingCurve::new(0.0, 1.0, EaseFunction::SineIn);
///
/// assert_eq!(f.sample(t), c.sample(t));
/// ```
///
/// [easing function]: EaseFunction
/// [the unit interval]: Interval::UNIT
/// [`sample`]: EasingCurve::sample
/// [`sample_clamped`]: EasingCurve::sample_clamped
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct EasingCurve<T> {
    start: T,
    end: T,
    ease_fn: EaseFunction,
}

impl<T> EasingCurve<T> {
    /// Given a `start` and `end` value, create a curve parametrized over [the unit interval]
    /// that connects them, using the given [ease function] to determine the form of the
    /// curve in between.
    ///
    /// [the unit interval]: Interval::UNIT
    /// [ease function]: EaseFunction
    pub fn new(start: T, end: T, ease_fn: EaseFunction) -> Self {
        Self {
            start,
            end,
            ease_fn,
        }
    }
}

impl<T> Curve<T> for EasingCurve<T>
where
    T: Ease + Clone,
{
    #[inline]
    fn domain(&self) -> Interval {
        Interval::UNIT
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        let remapped_t = self.ease_fn.eval(t);
        T::interpolating_curve_unbounded(self.start.clone(), self.end.clone())
            .sample_unchecked(remapped_t)
    }
}

/// Configuration options for the [`EaseFunction::Steps`] curves. This closely replicates the
/// [CSS step function specification].
///
/// [CSS step function specification]: https://developer.mozilla.org/en-US/docs/Web/CSS/easing-function/steps#description
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(bevy_reflect::Reflect),
    reflect(Clone, Default, PartialEq)
)]
pub enum JumpAt {
    /// Indicates that the first step happens when the animation begins.
    ///
    #[doc = include_str!("../../images/easefunction/StartSteps.svg")]
    Start,
    /// Indicates that the last step happens when the animation ends.
    ///
    #[doc = include_str!("../../images/easefunction/EndSteps.svg")]
    #[default]
    End,
    /// Indicates neither early nor late jumps happen.
    ///
    #[doc = include_str!("../../images/easefunction/NoneSteps.svg")]
    None,
    /// Indicates both early and late jumps happen.
    ///
    #[doc = include_str!("../../images/easefunction/BothSteps.svg")]
    Both,
}

impl JumpAt {
    #[inline]
    pub(crate) fn eval(self, num_steps: usize, t: f32) -> f32 {
        use crate::ops;

        let (a, b) = match self {
            JumpAt::Start => (1.0, 0),
            JumpAt::End => (0.0, 0),
            JumpAt::None => (0.0, -1),
            JumpAt::Both => (1.0, 1),
        };

        let current_step = ops::floor(t * num_steps as f32) + a;
        let step_size = (num_steps as isize + b).max(1) as f32;

        (current_step / step_size).clamp(0.0, 1.0)
    }
}

/// Curve functions over the [unit interval], commonly used for easing transitions.
///
/// `EaseFunction` can be used on its own to interpolate between `0.0` and `1.0`.
/// It can also be combined with [`EasingCurve`] to interpolate between other
/// intervals and types, including vectors and rotations.
///
/// # Example
///
/// [`sample`] the smoothstep function at various points. This will return `None`
/// if the parameter is outside the unit interval.
///
/// ```
/// # use bevy_math::prelude::*;
/// let f = EaseFunction::SmoothStep;
///
/// assert_eq!(f.sample(-1.0), None);
/// assert_eq!(f.sample(0.0), Some(0.0));
/// assert_eq!(f.sample(0.5), Some(0.5));
/// assert_eq!(f.sample(1.0), Some(1.0));
/// assert_eq!(f.sample(2.0), None);
/// ```
///
/// [`sample_clamped`] will clamp the parameter to the unit interval, so it
/// always returns a value.
///
/// ```
/// # use bevy_math::prelude::*;
/// # let f = EaseFunction::SmoothStep;
/// assert_eq!(f.sample_clamped(-1.0), 0.0);
/// assert_eq!(f.sample_clamped(0.0), 0.0);
/// assert_eq!(f.sample_clamped(0.5), 0.5);
/// assert_eq!(f.sample_clamped(1.0), 1.0);
/// assert_eq!(f.sample_clamped(2.0), 1.0);
/// ```
///
/// [`sample`]: EaseFunction::sample
/// [`sample_clamped`]: EaseFunction::sample_clamped
/// [unit interval]: `Interval::UNIT`
#[non_exhaustive]
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(bevy_reflect::Reflect),
    reflect(Clone, PartialEq)
)]
// Note: Graphs are auto-generated via `tools/build-easefunction-graphs`.
pub enum EaseFunction {
    /// `f(t) = t`
    ///
    #[doc = include_str!("../../images/easefunction/Linear.svg")]
    Linear,

    /// `f(t) = t²`
    ///
    /// This is the Hermite interpolator for
    /// - f(0) = 0
    /// - f(1) = 1
    /// - f′(0) = 0
    ///
    #[doc = include_str!("../../images/easefunction/QuadraticIn.svg")]
    QuadraticIn,
    /// `f(t) = -(t * (t - 2.0))`
    ///
    /// This is the Hermite interpolator for
    /// - f(0) = 0
    /// - f(1) = 1
    /// - f′(1) = 0
    ///
    #[doc = include_str!("../../images/easefunction/QuadraticOut.svg")]
    QuadraticOut,
    /// Behaves as `EaseFunction::QuadraticIn` for t < 0.5 and as `EaseFunction::QuadraticOut` for t >= 0.5
    ///
    /// A quadratic has too low of a degree to be both an `InOut` and C²,
    /// so consider using at least a cubic (such as [`EaseFunction::SmoothStep`])
    /// if you want the acceleration to be continuous.
    ///
    #[doc = include_str!("../../images/easefunction/QuadraticInOut.svg")]
    QuadraticInOut,

    /// `f(t) = t³`
    ///
    /// This is the Hermite interpolator for
    /// - f(0) = 0
    /// - f(1) = 1
    /// - f′(0) = 0
    /// - f″(0) = 0
    ///
    #[doc = include_str!("../../images/easefunction/CubicIn.svg")]
    CubicIn,
    /// `f(t) = (t - 1.0)³ + 1.0`
    ///
    #[doc = include_str!("../../images/easefunction/CubicOut.svg")]
    CubicOut,
    /// Behaves as `EaseFunction::CubicIn` for t < 0.5 and as `EaseFunction::CubicOut` for t >= 0.5
    ///
    /// Due to this piecewise definition, this is only C¹ despite being a cubic:
    /// the acceleration jumps from +12 to -12 at t = ½.
    ///
    /// Consider using [`EaseFunction::SmoothStep`] instead, which is also cubic,
    /// or [`EaseFunction::SmootherStep`] if you picked this because you wanted
    /// the acceleration at the endpoints to also be zero.
    ///
    #[doc = include_str!("../../images/easefunction/CubicInOut.svg")]
    CubicInOut,

    /// `f(t) = t⁴`
    ///
    #[doc = include_str!("../../images/easefunction/QuarticIn.svg")]
    QuarticIn,
    /// `f(t) = (t - 1.0)³ * (1.0 - t) + 1.0`
    ///
    #[doc = include_str!("../../images/easefunction/QuarticOut.svg")]
    QuarticOut,
    /// Behaves as `EaseFunction::QuarticIn` for t < 0.5 and as `EaseFunction::QuarticOut` for t >= 0.5
    ///
    #[doc = include_str!("../../images/easefunction/QuarticInOut.svg")]
    QuarticInOut,

    /// `f(t) = t⁵`
    ///
    #[doc = include_str!("../../images/easefunction/QuinticIn.svg")]
    QuinticIn,
    /// `f(t) = (t - 1.0)⁵ + 1.0`
    ///
    #[doc = include_str!("../../images/easefunction/QuinticOut.svg")]
    QuinticOut,
    /// Behaves as `EaseFunction::QuinticIn` for t < 0.5 and as `EaseFunction::QuinticOut` for t >= 0.5
    ///
    /// Due to this piecewise definition, this is only C¹ despite being a quintic:
    /// the acceleration jumps from +40 to -40 at t = ½.
    ///
    /// Consider using [`EaseFunction::SmootherStep`] instead, which is also quintic.
    ///
    #[doc = include_str!("../../images/easefunction/QuinticInOut.svg")]
    QuinticInOut,

    /// Behaves as the first half of [`EaseFunction::SmoothStep`].
    ///
    /// This has f″(1) = 0, unlike [`EaseFunction::QuadraticIn`] which starts similarly.
    ///
    #[doc = include_str!("../../images/easefunction/SmoothStepIn.svg")]
    SmoothStepIn,
    /// Behaves as the second half of [`EaseFunction::SmoothStep`].
    ///
    /// This has f″(0) = 0, unlike [`EaseFunction::QuadraticOut`] which ends similarly.
    ///
    #[doc = include_str!("../../images/easefunction/SmoothStepOut.svg")]
    SmoothStepOut,
    /// `f(t) = 3t² - 2t³`
    ///
    /// This is the Hermite interpolator for
    /// - f(0) = 0
    /// - f(1) = 1
    /// - f′(0) = 0
    /// - f′(1) = 0
    ///
    /// See also [`smoothstep` in GLSL][glss].
    ///
    /// [glss]: https://registry.khronos.org/OpenGL-Refpages/gl4/html/smoothstep.xhtml
    ///
    #[doc = include_str!("../../images/easefunction/SmoothStep.svg")]
    SmoothStep,

    /// Behaves as the first half of [`EaseFunction::SmootherStep`].
    ///
    /// This has f″(1) = 0, unlike [`EaseFunction::CubicIn`] which starts similarly.
    ///
    #[doc = include_str!("../../images/easefunction/SmootherStepIn.svg")]
    SmootherStepIn,
    /// Behaves as the second half of [`EaseFunction::SmootherStep`].
    ///
    /// This has f″(0) = 0, unlike [`EaseFunction::CubicOut`] which ends similarly.
    ///
    #[doc = include_str!("../../images/easefunction/SmootherStepOut.svg")]
    SmootherStepOut,
    /// `f(t) = 6t⁵ - 15t⁴ + 10t³`
    ///
    /// This is the Hermite interpolator for
    /// - f(0) = 0
    /// - f(1) = 1
    /// - f′(0) = 0
    /// - f′(1) = 0
    /// - f″(0) = 0
    /// - f″(1) = 0
    ///
    #[doc = include_str!("../../images/easefunction/SmootherStep.svg")]
    SmootherStep,

    /// `f(t) = 1.0 - cos(t * π / 2.0)`
    ///
    #[doc = include_str!("../../images/easefunction/SineIn.svg")]
    SineIn,
    /// `f(t) = sin(t * π / 2.0)`
    ///
    #[doc = include_str!("../../images/easefunction/SineOut.svg")]
    SineOut,
    /// Behaves as `EaseFunction::SineIn` for t < 0.5 and as `EaseFunction::SineOut` for t >= 0.5
    ///
    #[doc = include_str!("../../images/easefunction/SineInOut.svg")]
    SineInOut,

    /// `f(t) = 1.0 - sqrt(1.0 - t²)`
    ///
    #[doc = include_str!("../../images/easefunction/CircularIn.svg")]
    CircularIn,
    /// `f(t) = sqrt((2.0 - t) * t)`
    ///
    #[doc = include_str!("../../images/easefunction/CircularOut.svg")]
    CircularOut,
    /// Behaves as `EaseFunction::CircularIn` for t < 0.5 and as `EaseFunction::CircularOut` for t >= 0.5
    ///
    #[doc = include_str!("../../images/easefunction/CircularInOut.svg")]
    CircularInOut,

    /// `f(t) ≈ 2.0^(10.0 * (t - 1.0))`
    ///
    /// The precise definition adjusts it slightly so it hits both `(0, 0)` and `(1, 1)`:
    /// `f(t) = 2.0^(10.0 * t - A) - B`, where A = log₂(2¹⁰-1) and B = 1/(2¹⁰-1).
    ///
    #[doc = include_str!("../../images/easefunction/ExponentialIn.svg")]
    ExponentialIn,
    /// `f(t) ≈ 1.0 - 2.0^(-10.0 * t)`
    ///
    /// As with `EaseFunction::ExponentialIn`, the precise definition adjusts it slightly
    // so it hits both `(0, 0)` and `(1, 1)`.
    ///
    #[doc = include_str!("../../images/easefunction/ExponentialOut.svg")]
    ExponentialOut,
    /// Behaves as `EaseFunction::ExponentialIn` for t < 0.5 and as `EaseFunction::ExponentialOut` for t >= 0.5
    ///
    #[doc = include_str!("../../images/easefunction/ExponentialInOut.svg")]
    ExponentialInOut,

    /// `f(t) = -2.0^(10.0 * t - 10.0) * sin((t * 10.0 - 10.75) * 2.0 * π / 3.0)`
    ///
    #[doc = include_str!("../../images/easefunction/ElasticIn.svg")]
    ElasticIn,
    /// `f(t) = 2.0^(-10.0 * t) * sin((t * 10.0 - 0.75) * 2.0 * π / 3.0) + 1.0`
    ///
    #[doc = include_str!("../../images/easefunction/ElasticOut.svg")]
    ElasticOut,
    /// Behaves as `EaseFunction::ElasticIn` for t < 0.5 and as `EaseFunction::ElasticOut` for t >= 0.5
    ///
    #[doc = include_str!("../../images/easefunction/ElasticInOut.svg")]
    ElasticInOut,

    /// `f(t) = 2.70158 * t³ - 1.70158 * t²`
    ///
    #[doc = include_str!("../../images/easefunction/BackIn.svg")]
    BackIn,
    /// `f(t) = 1.0 +  2.70158 * (t - 1.0)³ - 1.70158 * (t - 1.0)²`
    ///
    #[doc = include_str!("../../images/easefunction/BackOut.svg")]
    BackOut,
    /// Behaves as `EaseFunction::BackIn` for t < 0.5 and as `EaseFunction::BackOut` for t >= 0.5
    ///
    #[doc = include_str!("../../images/easefunction/BackInOut.svg")]
    BackInOut,

    /// bouncy at the start!
    ///
    #[doc = include_str!("../../images/easefunction/BounceIn.svg")]
    BounceIn,
    /// bouncy at the end!
    ///
    #[doc = include_str!("../../images/easefunction/BounceOut.svg")]
    BounceOut,
    /// Behaves as `EaseFunction::BounceIn` for t < 0.5 and as `EaseFunction::BounceOut` for t >= 0.5
    ///
    #[doc = include_str!("../../images/easefunction/BounceInOut.svg")]
    BounceInOut,

    /// `n` steps connecting the start and the end. Jumping behavior is customizable via
    /// [`JumpAt`]. See [`JumpAt`] for all the options and visual examples.
    Steps(usize, JumpAt),

    /// `f(omega,t) = 1 - (1 - t)²(2sin(omega * t) / omega + cos(omega * t))`, parametrized by `omega`
    ///
    #[doc = include_str!("../../images/easefunction/Elastic.svg")]
    Elastic(f32),
}

/// `f(t) = t`
///
#[doc = include_str!("../../images/easefunction/Linear.svg")]
#[derive(Copy, Clone)]
pub struct LinearCurve;

/// `f(t) = t²`
///
/// This is the Hermite interpolator for
/// - f(0) = 0
/// - f(1) = 1
/// - f′(0) = 0
///
#[doc = include_str!("../../images/easefunction/QuadraticIn.svg")]
#[derive(Copy, Clone)]
pub struct QuadraticInCurve;

/// `f(t) = -(t * (t - 2.0))`
///
/// This is the Hermite interpolator for
/// - f(0) = 0
/// - f(1) = 1
/// - f′(1) = 0
///
#[doc = include_str!("../../images/easefunction/QuadraticOut.svg")]
#[derive(Copy, Clone)]
pub struct QuadraticOutCurve;

/// Behaves as `QuadraticIn` for t < 0.5 and as `QuadraticOut` for t >= 0.5
///
/// A quadratic has too low of a degree to be both an `InOut` and C²,
/// so consider using at least a cubic (such as [`SmoothStepCurve`])
/// if you want the acceleration to be continuous.
///
#[doc = include_str!("../../images/easefunction/QuadraticInOut.svg")]
#[derive(Copy, Clone)]
pub struct QuadraticInOutCurve;

/// `f(t) = t³`
///
/// This is the Hermite interpolator for
/// - f(0) = 0
/// - f(1) = 1
/// - f′(0) = 0
/// - f″(0) = 0
///
#[doc = include_str!("../../images/easefunction/CubicIn.svg")]
#[derive(Copy, Clone)]
pub struct CubicInCurve;

/// `f(t) = (t - 1.0)³ + 1.0`
///
#[doc = include_str!("../../images/easefunction/CubicOut.svg")]
#[derive(Copy, Clone)]
pub struct CubicOutCurve;

/// Behaves as `CubicIn` for t < 0.5 and as `CubicOut` for t >= 0.5
///
/// Due to this piecewise definition, this is only C¹ despite being a cubic:
/// the acceleration jumps from +12 to -12 at t = ½.
///
/// Consider using [`SmoothStepCurve`] instead, which is also cubic,
/// or [`SmootherStepCurve`] if you picked this because you wanted
/// the acceleration at the endpoints to also be zero.
///
#[doc = include_str!("../../images/easefunction/CubicInOut.svg")]
#[derive(Copy, Clone)]
pub struct CubicInOutCurve;

/// `f(t) = t⁴`
///
#[doc = include_str!("../../images/easefunction/QuarticIn.svg")]
#[derive(Copy, Clone)]
pub struct QuarticInCurve;

/// `f(t) = (t - 1.0)³ * (1.0 - t) + 1.0`
///
#[doc = include_str!("../../images/easefunction/QuarticOut.svg")]
#[derive(Copy, Clone)]
pub struct QuarticOutCurve;

/// Behaves as `QuarticIn` for t < 0.5 and as `QuarticOut` for t >= 0.5
///
#[doc = include_str!("../../images/easefunction/QuarticInOut.svg")]
#[derive(Copy, Clone)]
pub struct QuarticInOutCurve;

/// `f(t) = t⁵`
///
#[doc = include_str!("../../images/easefunction/QuinticIn.svg")]
#[derive(Copy, Clone)]
pub struct QuinticInCurve;

/// `f(t) = (t - 1.0)⁵ + 1.0`
///
#[doc = include_str!("../../images/easefunction/QuinticOut.svg")]
#[derive(Copy, Clone)]
pub struct QuinticOutCurve;

/// Behaves as `QuinticIn` for t < 0.5 and as `QuinticOut` for t >= 0.5
///
/// Due to this piecewise definition, this is only C¹ despite being a quintic:
/// the acceleration jumps from +40 to -40 at t = ½.
///
/// Consider using [`SmootherStepCurve`] instead, which is also quintic.
///
#[doc = include_str!("../../images/easefunction/QuinticInOut.svg")]
#[derive(Copy, Clone)]
pub struct QuinticInOutCurve;

/// Behaves as the first half of [`SmoothStepCurve`].
///
/// This has f″(1) = 0, unlike [`QuadraticInCurve`] which starts similarly.
///
#[doc = include_str!("../../images/easefunction/SmoothStepIn.svg")]
#[derive(Copy, Clone)]
pub struct SmoothStepInCurve;

/// Behaves as the second half of [`SmoothStepCurve`].
///
/// This has f″(0) = 0, unlike [`QuadraticOutCurve`] which ends similarly.
///
#[doc = include_str!("../../images/easefunction/SmoothStepOut.svg")]
#[derive(Copy, Clone)]
pub struct SmoothStepOutCurve;

/// `f(t) = 3t² - 2t³`
///
/// This is the Hermite interpolator for
/// - f(0) = 0
/// - f(1) = 1
/// - f′(0) = 0
/// - f′(1) = 0
///
/// See also [`smoothstep` in GLSL][glss].
///
/// [glss]: https://registry.khronos.org/OpenGL-Refpages/gl4/html/smoothstep.xhtml
///
#[doc = include_str!("../../images/easefunction/SmoothStep.svg")]
#[derive(Copy, Clone)]
pub struct SmoothStepCurve;

/// Behaves as the first half of [`SmootherStepCurve`].
///
/// This has f″(1) = 0, unlike [`CubicInCurve`] which starts similarly.
///
#[doc = include_str!("../../images/easefunction/SmootherStepIn.svg")]
#[derive(Copy, Clone)]
pub struct SmootherStepInCurve;

/// Behaves as the second half of [`SmootherStepCurve`].
///
/// This has f″(0) = 0, unlike [`CubicOutCurve`] which ends similarly.
///
#[doc = include_str!("../../images/easefunction/SmootherStepOut.svg")]
#[derive(Copy, Clone)]
pub struct SmootherStepOutCurve;

/// `f(t) = 6t⁵ - 15t⁴ + 10t³`
///
/// This is the Hermite interpolator for
/// - f(0) = 0
/// - f(1) = 1
/// - f′(0) = 0
/// - f′(1) = 0
/// - f″(0) = 0
/// - f″(1) = 0
///
#[doc = include_str!("../../images/easefunction/SmootherStep.svg")]
#[derive(Copy, Clone)]
pub struct SmootherStepCurve;

/// `f(t) = 1.0 - cos(t * π / 2.0)`
///
#[doc = include_str!("../../images/easefunction/SineIn.svg")]
#[derive(Copy, Clone)]
pub struct SineInCurve;

/// `f(t) = sin(t * π / 2.0)`
///
#[doc = include_str!("../../images/easefunction/SineOut.svg")]
#[derive(Copy, Clone)]
pub struct SineOutCurve;

/// Behaves as `SineIn` for t < 0.5 and as `SineOut` for t >= 0.5
///
#[doc = include_str!("../../images/easefunction/SineInOut.svg")]
#[derive(Copy, Clone)]
pub struct SineInOutCurve;

/// `f(t) = 1.0 - sqrt(1.0 - t²)`
///
#[doc = include_str!("../../images/easefunction/CircularIn.svg")]
#[derive(Copy, Clone)]
pub struct CircularInCurve;

/// `f(t) = sqrt((2.0 - t) * t)`
///
#[doc = include_str!("../../images/easefunction/CircularOut.svg")]
#[derive(Copy, Clone)]
pub struct CircularOutCurve;

/// Behaves as `CircularIn` for t < 0.5 and as `CircularOut` for t >= 0.5
///
#[doc = include_str!("../../images/easefunction/CircularInOut.svg")]
#[derive(Copy, Clone)]
pub struct CircularInOutCurve;

/// `f(t) ≈ 2.0^(10.0 * (t - 1.0))`
///
/// The precise definition adjusts it slightly so it hits both `(0, 0)` and `(1, 1)`:
/// `f(t) = 2.0^(10.0 * t - A) - B`, where A = log₂(2¹⁰-1) and B = 1/(2¹⁰-1).
///
#[doc = include_str!("../../images/easefunction/ExponentialIn.svg")]
#[derive(Copy, Clone)]
pub struct ExponentialInCurve;

/// `f(t) ≈ 1.0 - 2.0^(-10.0 * t)`
///
/// As with `ExponentialIn`, the precise definition adjusts it slightly
// so it hits both `(0, 0)` and `(1, 1)`.
///
#[doc = include_str!("../../images/easefunction/ExponentialOut.svg")]
#[derive(Copy, Clone)]
pub struct ExponentialOutCurve;

/// Behaves as `ExponentialIn` for t < 0.5 and as `ExponentialOut` for t >= 0.5
///
#[doc = include_str!("../../images/easefunction/ExponentialInOut.svg")]
#[derive(Copy, Clone)]
pub struct ExponentialInOutCurve;

/// `f(t) = -2.0^(10.0 * t - 10.0) * sin((t * 10.0 - 10.75) * 2.0 * π / 3.0)`
///
#[doc = include_str!("../../images/easefunction/ElasticIn.svg")]
#[derive(Copy, Clone)]
pub struct ElasticInCurve;

/// `f(t) = 2.0^(-10.0 * t) * sin((t * 10.0 - 0.75) * 2.0 * π / 3.0) + 1.0`
///
#[doc = include_str!("../../images/easefunction/ElasticOut.svg")]
#[derive(Copy, Clone)]
pub struct ElasticOutCurve;

/// Behaves as `ElasticIn` for t < 0.5 and as `ElasticOut` for t >= 0.5
///
#[doc = include_str!("../../images/easefunction/ElasticInOut.svg")]
#[derive(Copy, Clone)]
pub struct ElasticInOutCurve;

/// `f(t) = 2.70158 * t³ - 1.70158 * t²`
///
#[doc = include_str!("../../images/easefunction/BackIn.svg")]
#[derive(Copy, Clone)]
pub struct BackInCurve;

/// `f(t) = 1.0 +  2.70158 * (t - 1.0)³ - 1.70158 * (t - 1.0)²`
///
#[doc = include_str!("../../images/easefunction/BackOut.svg")]
#[derive(Copy, Clone)]
pub struct BackOutCurve;

/// Behaves as `BackIn` for t < 0.5 and as `BackOut` for t >= 0.5
///
#[doc = include_str!("../../images/easefunction/BackInOut.svg")]
#[derive(Copy, Clone)]
pub struct BackInOutCurve;

/// bouncy at the start!
///
#[doc = include_str!("../../images/easefunction/BounceIn.svg")]
#[derive(Copy, Clone)]
pub struct BounceInCurve;

/// bouncy at the end!
///
#[doc = include_str!("../../images/easefunction/BounceOut.svg")]
#[derive(Copy, Clone)]
pub struct BounceOutCurve;

/// Behaves as `BounceIn` for t < 0.5 and as `BounceOut` for t >= 0.5
///
#[doc = include_str!("../../images/easefunction/BounceInOut.svg")]
#[derive(Copy, Clone)]
pub struct BounceInOutCurve;

/// `n` steps connecting the start and the end. Jumping behavior is customizable via
/// [`JumpAt`]. See [`JumpAt`] for all the options and visual examples.
#[derive(Copy, Clone)]
pub struct StepsCurve(pub usize, pub JumpAt);

/// `f(omega,t) = 1 - (1 - t)²(2sin(omega * t) / omega + cos(omega * t))`, parametrized by `omega`
///
#[doc = include_str!("../../images/easefunction/Elastic.svg")]
#[derive(Copy, Clone)]
pub struct ElasticCurve(pub f32);

/// Implements `Curve<f32>` for a unit struct using a function in `easing_functions`.
macro_rules! impl_ease_unit_struct {
    ($ty: ty, $fn: ident) => {
        impl Curve<f32> for $ty {
            #[inline]
            fn domain(&self) -> Interval {
                Interval::UNIT
            }

            #[inline]
            fn sample_unchecked(&self, t: f32) -> f32 {
                easing_functions::$fn(t)
            }
        }
    };
}

impl_ease_unit_struct!(LinearCurve, linear);
impl_ease_unit_struct!(QuadraticInCurve, quadratic_in);
impl_ease_unit_struct!(QuadraticOutCurve, quadratic_out);
impl_ease_unit_struct!(QuadraticInOutCurve, quadratic_in_out);
impl_ease_unit_struct!(CubicInCurve, cubic_in);
impl_ease_unit_struct!(CubicOutCurve, cubic_out);
impl_ease_unit_struct!(CubicInOutCurve, cubic_in_out);
impl_ease_unit_struct!(QuarticInCurve, quartic_in);
impl_ease_unit_struct!(QuarticOutCurve, quartic_out);
impl_ease_unit_struct!(QuarticInOutCurve, quartic_in_out);
impl_ease_unit_struct!(QuinticInCurve, quintic_in);
impl_ease_unit_struct!(QuinticOutCurve, quintic_out);
impl_ease_unit_struct!(QuinticInOutCurve, quintic_in_out);
impl_ease_unit_struct!(SmoothStepInCurve, smoothstep_in);
impl_ease_unit_struct!(SmoothStepOutCurve, smoothstep_out);
impl_ease_unit_struct!(SmoothStepCurve, smoothstep);
impl_ease_unit_struct!(SmootherStepInCurve, smootherstep_in);
impl_ease_unit_struct!(SmootherStepOutCurve, smootherstep_out);
impl_ease_unit_struct!(SmootherStepCurve, smootherstep);
impl_ease_unit_struct!(SineInCurve, sine_in);
impl_ease_unit_struct!(SineOutCurve, sine_out);
impl_ease_unit_struct!(SineInOutCurve, sine_in_out);
impl_ease_unit_struct!(CircularInCurve, circular_in);
impl_ease_unit_struct!(CircularOutCurve, circular_out);
impl_ease_unit_struct!(CircularInOutCurve, circular_in_out);
impl_ease_unit_struct!(ExponentialInCurve, exponential_in);
impl_ease_unit_struct!(ExponentialOutCurve, exponential_out);
impl_ease_unit_struct!(ExponentialInOutCurve, exponential_in_out);
impl_ease_unit_struct!(ElasticInCurve, elastic_in);
impl_ease_unit_struct!(ElasticOutCurve, elastic_out);
impl_ease_unit_struct!(ElasticInOutCurve, elastic_in_out);
impl_ease_unit_struct!(BackInCurve, back_in);
impl_ease_unit_struct!(BackOutCurve, back_out);
impl_ease_unit_struct!(BackInOutCurve, back_in_out);
impl_ease_unit_struct!(BounceInCurve, bounce_in);
impl_ease_unit_struct!(BounceOutCurve, bounce_out);
impl_ease_unit_struct!(BounceInOutCurve, bounce_in_out);

impl Curve<f32> for StepsCurve {
    #[inline]
    fn domain(&self) -> Interval {
        Interval::UNIT
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> f32 {
        easing_functions::steps(self.0, self.1, t)
    }
}

impl Curve<f32> for ElasticCurve {
    #[inline]
    fn domain(&self) -> Interval {
        Interval::UNIT
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> f32 {
        easing_functions::elastic(self.0, t)
    }
}

mod easing_functions {
    use core::f32::consts::{FRAC_PI_2, FRAC_PI_3, PI};

    use crate::{ops, FloatPow};

    #[inline]
    pub(crate) fn linear(t: f32) -> f32 {
        t
    }

    #[inline]
    pub(crate) fn quadratic_in(t: f32) -> f32 {
        t.squared()
    }
    #[inline]
    pub(crate) fn quadratic_out(t: f32) -> f32 {
        1.0 - (1.0 - t).squared()
    }
    #[inline]
    pub(crate) fn quadratic_in_out(t: f32) -> f32 {
        if t < 0.5 {
            2.0 * t.squared()
        } else {
            1.0 - (-2.0 * t + 2.0).squared() / 2.0
        }
    }

    #[inline]
    pub(crate) fn cubic_in(t: f32) -> f32 {
        t.cubed()
    }
    #[inline]
    pub(crate) fn cubic_out(t: f32) -> f32 {
        1.0 - (1.0 - t).cubed()
    }
    #[inline]
    pub(crate) fn cubic_in_out(t: f32) -> f32 {
        if t < 0.5 {
            4.0 * t.cubed()
        } else {
            1.0 - (-2.0 * t + 2.0).cubed() / 2.0
        }
    }

    #[inline]
    pub(crate) fn quartic_in(t: f32) -> f32 {
        t * t * t * t
    }
    #[inline]
    pub(crate) fn quartic_out(t: f32) -> f32 {
        1.0 - (1.0 - t) * (1.0 - t) * (1.0 - t) * (1.0 - t)
    }
    #[inline]
    pub(crate) fn quartic_in_out(t: f32) -> f32 {
        if t < 0.5 {
            8.0 * t * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0) * (-2.0 * t + 2.0) * (-2.0 * t + 2.0) * (-2.0 * t + 2.0) / 2.0
        }
    }

    #[inline]
    pub(crate) fn quintic_in(t: f32) -> f32 {
        t * t * t * t * t
    }
    #[inline]
    pub(crate) fn quintic_out(t: f32) -> f32 {
        1.0 - (1.0 - t) * (1.0 - t) * (1.0 - t) * (1.0 - t) * (1.0 - t)
    }
    #[inline]
    pub(crate) fn quintic_in_out(t: f32) -> f32 {
        if t < 0.5 {
            16.0 * t * t * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0)
                * (-2.0 * t + 2.0)
                * (-2.0 * t + 2.0)
                * (-2.0 * t + 2.0)
                * (-2.0 * t + 2.0)
                / 2.0
        }
    }

    #[inline]
    pub(crate) fn smoothstep_in(t: f32) -> f32 {
        ((1.5 - 0.5 * t) * t) * t
    }

    #[inline]
    pub(crate) fn smoothstep_out(t: f32) -> f32 {
        (1.5 + (-0.5 * t) * t) * t
    }

    #[inline]
    pub(crate) fn smoothstep(t: f32) -> f32 {
        ((3.0 - 2.0 * t) * t) * t
    }

    #[inline]
    pub(crate) fn smootherstep_in(t: f32) -> f32 {
        (((2.5 + (-1.875 + 0.375 * t) * t) * t) * t) * t
    }

    #[inline]
    pub(crate) fn smootherstep_out(t: f32) -> f32 {
        (1.875 + ((-1.25 + (0.375 * t) * t) * t) * t) * t
    }

    #[inline]
    pub(crate) fn smootherstep(t: f32) -> f32 {
        (((10.0 + (-15.0 + 6.0 * t) * t) * t) * t) * t
    }

    #[inline]
    pub(crate) fn sine_in(t: f32) -> f32 {
        1.0 - ops::cos(t * FRAC_PI_2)
    }
    #[inline]
    pub(crate) fn sine_out(t: f32) -> f32 {
        ops::sin(t * FRAC_PI_2)
    }
    #[inline]
    pub(crate) fn sine_in_out(t: f32) -> f32 {
        -(ops::cos(PI * t) - 1.0) / 2.0
    }

    #[inline]
    pub(crate) fn circular_in(t: f32) -> f32 {
        1.0 - ops::sqrt(1.0 - t.squared())
    }
    #[inline]
    pub(crate) fn circular_out(t: f32) -> f32 {
        ops::sqrt(1.0 - (t - 1.0).squared())
    }
    #[inline]
    pub(crate) fn circular_in_out(t: f32) -> f32 {
        if t < 0.5 {
            (1.0 - ops::sqrt(1.0 - (2.0 * t).squared())) / 2.0
        } else {
            (ops::sqrt(1.0 - (-2.0 * t + 2.0).squared()) + 1.0) / 2.0
        }
    }

    // These are copied from a high precision calculator; I'd rather show them
    // with blatantly more digits than needed (since rust will round them to the
    // nearest representable value anyway) rather than make it seem like the
    // truncated value is somehow carefully chosen.
    #[expect(
        clippy::excessive_precision,
        reason = "This is deliberately more precise than an f32 will allow, as truncating the value might imply that the value is carefully chosen."
    )]
    const LOG2_1023: f32 = 9.998590429745328646459226;
    #[expect(
        clippy::excessive_precision,
        reason = "This is deliberately more precise than an f32 will allow, as truncating the value might imply that the value is carefully chosen."
    )]
    const FRAC_1_1023: f32 = 0.00097751710654936461388074291;
    #[inline]
    pub(crate) fn exponential_in(t: f32) -> f32 {
        // Derived from a rescaled exponential formula `(2^(10*t) - 1) / (2^10 - 1)`
        // See <https://www.wolframalpha.com/input?i=solve+over+the+reals%3A+pow%282%2C+10-A%29+-+pow%282%2C+-A%29%3D+1>
        ops::exp2(10.0 * t - LOG2_1023) - FRAC_1_1023
    }
    #[inline]
    pub(crate) fn exponential_out(t: f32) -> f32 {
        (FRAC_1_1023 + 1.0) - ops::exp2(-10.0 * t - (LOG2_1023 - 10.0))
    }
    #[inline]
    pub(crate) fn exponential_in_out(t: f32) -> f32 {
        if t < 0.5 {
            ops::exp2(20.0 * t - (LOG2_1023 + 1.0)) - (FRAC_1_1023 / 2.0)
        } else {
            (FRAC_1_1023 / 2.0 + 1.0) - ops::exp2(-20.0 * t - (LOG2_1023 - 19.0))
        }
    }

    #[inline]
    pub(crate) fn back_in(t: f32) -> f32 {
        let c = 1.70158;

        (c + 1.0) * t.cubed() - c * t.squared()
    }
    #[inline]
    pub(crate) fn back_out(t: f32) -> f32 {
        let c = 1.70158;

        1.0 + (c + 1.0) * (t - 1.0).cubed() + c * (t - 1.0).squared()
    }
    #[inline]
    pub(crate) fn back_in_out(t: f32) -> f32 {
        let c1 = 1.70158;
        let c2 = c1 + 1.525;

        if t < 0.5 {
            (2.0 * t).squared() * ((c2 + 1.0) * 2.0 * t - c2) / 2.0
        } else {
            ((2.0 * t - 2.0).squared() * ((c2 + 1.0) * (2.0 * t - 2.0) + c2) + 2.0) / 2.0
        }
    }

    #[inline]
    pub(crate) fn elastic_in(t: f32) -> f32 {
        -ops::powf(2.0, 10.0 * t - 10.0) * ops::sin((t * 10.0 - 10.75) * 2.0 * FRAC_PI_3)
    }
    #[inline]
    pub(crate) fn elastic_out(t: f32) -> f32 {
        ops::powf(2.0, -10.0 * t) * ops::sin((t * 10.0 - 0.75) * 2.0 * FRAC_PI_3) + 1.0
    }
    #[inline]
    pub(crate) fn elastic_in_out(t: f32) -> f32 {
        let c = (2.0 * PI) / 4.5;

        if t < 0.5 {
            -ops::powf(2.0, 20.0 * t - 10.0) * ops::sin((t * 20.0 - 11.125) * c) / 2.0
        } else {
            ops::powf(2.0, -20.0 * t + 10.0) * ops::sin((t * 20.0 - 11.125) * c) / 2.0 + 1.0
        }
    }

    #[inline]
    pub(crate) fn bounce_in(t: f32) -> f32 {
        1.0 - bounce_out(1.0 - t)
    }
    #[inline]
    pub(crate) fn bounce_out(t: f32) -> f32 {
        if t < 4.0 / 11.0 {
            (121.0 * t.squared()) / 16.0
        } else if t < 8.0 / 11.0 {
            (363.0 / 40.0 * t.squared()) - (99.0 / 10.0 * t) + 17.0 / 5.0
        } else if t < 9.0 / 10.0 {
            (4356.0 / 361.0 * t.squared()) - (35442.0 / 1805.0 * t) + 16061.0 / 1805.0
        } else {
            (54.0 / 5.0 * t.squared()) - (513.0 / 25.0 * t) + 268.0 / 25.0
        }
    }
    #[inline]
    pub(crate) fn bounce_in_out(t: f32) -> f32 {
        if t < 0.5 {
            (1.0 - bounce_out(1.0 - 2.0 * t)) / 2.0
        } else {
            (1.0 + bounce_out(2.0 * t - 1.0)) / 2.0
        }
    }

    #[inline]
    pub(crate) fn steps(num_steps: usize, jump_at: super::JumpAt, t: f32) -> f32 {
        jump_at.eval(num_steps, t)
    }

    #[inline]
    pub(crate) fn elastic(omega: f32, t: f32) -> f32 {
        1.0 - (1.0 - t).squared() * (2.0 * ops::sin(omega * t) / omega + ops::cos(omega * t))
    }
}

impl EaseFunction {
    fn eval(&self, t: f32) -> f32 {
        match self {
            EaseFunction::Linear => easing_functions::linear(t),
            EaseFunction::QuadraticIn => easing_functions::quadratic_in(t),
            EaseFunction::QuadraticOut => easing_functions::quadratic_out(t),
            EaseFunction::QuadraticInOut => easing_functions::quadratic_in_out(t),
            EaseFunction::CubicIn => easing_functions::cubic_in(t),
            EaseFunction::CubicOut => easing_functions::cubic_out(t),
            EaseFunction::CubicInOut => easing_functions::cubic_in_out(t),
            EaseFunction::QuarticIn => easing_functions::quartic_in(t),
            EaseFunction::QuarticOut => easing_functions::quartic_out(t),
            EaseFunction::QuarticInOut => easing_functions::quartic_in_out(t),
            EaseFunction::QuinticIn => easing_functions::quintic_in(t),
            EaseFunction::QuinticOut => easing_functions::quintic_out(t),
            EaseFunction::QuinticInOut => easing_functions::quintic_in_out(t),
            EaseFunction::SmoothStepIn => easing_functions::smoothstep_in(t),
            EaseFunction::SmoothStepOut => easing_functions::smoothstep_out(t),
            EaseFunction::SmoothStep => easing_functions::smoothstep(t),
            EaseFunction::SmootherStepIn => easing_functions::smootherstep_in(t),
            EaseFunction::SmootherStepOut => easing_functions::smootherstep_out(t),
            EaseFunction::SmootherStep => easing_functions::smootherstep(t),
            EaseFunction::SineIn => easing_functions::sine_in(t),
            EaseFunction::SineOut => easing_functions::sine_out(t),
            EaseFunction::SineInOut => easing_functions::sine_in_out(t),
            EaseFunction::CircularIn => easing_functions::circular_in(t),
            EaseFunction::CircularOut => easing_functions::circular_out(t),
            EaseFunction::CircularInOut => easing_functions::circular_in_out(t),
            EaseFunction::ExponentialIn => easing_functions::exponential_in(t),
            EaseFunction::ExponentialOut => easing_functions::exponential_out(t),
            EaseFunction::ExponentialInOut => easing_functions::exponential_in_out(t),
            EaseFunction::ElasticIn => easing_functions::elastic_in(t),
            EaseFunction::ElasticOut => easing_functions::elastic_out(t),
            EaseFunction::ElasticInOut => easing_functions::elastic_in_out(t),
            EaseFunction::BackIn => easing_functions::back_in(t),
            EaseFunction::BackOut => easing_functions::back_out(t),
            EaseFunction::BackInOut => easing_functions::back_in_out(t),
            EaseFunction::BounceIn => easing_functions::bounce_in(t),
            EaseFunction::BounceOut => easing_functions::bounce_out(t),
            EaseFunction::BounceInOut => easing_functions::bounce_in_out(t),
            EaseFunction::Steps(num_steps, jump_at) => {
                easing_functions::steps(*num_steps, *jump_at, t)
            }
            EaseFunction::Elastic(omega) => easing_functions::elastic(*omega, t),
        }
    }
}

impl Curve<f32> for EaseFunction {
    #[inline]
    fn domain(&self) -> Interval {
        Interval::UNIT
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> f32 {
        self.eval(t)
    }
}

#[cfg(test)]
#[cfg(feature = "approx")]
mod tests {

    use crate::{Vec2, Vec3, Vec3A};
    use approx::assert_abs_diff_eq;

    use super::*;
    const MONOTONIC_IN_OUT_INOUT: &[[EaseFunction; 3]] = {
        use EaseFunction::*;
        &[
            [QuadraticIn, QuadraticOut, QuadraticInOut],
            [CubicIn, CubicOut, CubicInOut],
            [QuarticIn, QuarticOut, QuarticInOut],
            [QuinticIn, QuinticOut, QuinticInOut],
            [SmoothStepIn, SmoothStepOut, SmoothStep],
            [SmootherStepIn, SmootherStepOut, SmootherStep],
            [SineIn, SineOut, SineInOut],
            [CircularIn, CircularOut, CircularInOut],
            [ExponentialIn, ExponentialOut, ExponentialInOut],
        ]
    };

    // For easing function we don't care if eval(0) is super-tiny like 2.0e-28,
    // so add the same amount of error on both ends of the unit interval.
    const TOLERANCE: f32 = 1.0e-6;
    const _: () = const {
        assert!(1.0 - TOLERANCE != 1.0);
    };

    #[test]
    fn ease_functions_zero_to_one() {
        for ef in MONOTONIC_IN_OUT_INOUT.iter().flatten() {
            let start = ef.eval(0.0);
            assert!(
                (0.0..=TOLERANCE).contains(&start),
                "EaseFunction.{ef:?}(0) was {start:?}",
            );

            let finish = ef.eval(1.0);
            assert!(
                (1.0 - TOLERANCE..=1.0).contains(&finish),
                "EaseFunction.{ef:?}(1) was {start:?}",
            );
        }
    }

    #[test]
    fn ease_function_inout_deciles() {
        // convexity gives the comparisons against the input built-in tolerances
        for [ef_in, ef_out, ef_inout] in MONOTONIC_IN_OUT_INOUT {
            for x in [0.1, 0.2, 0.3, 0.4] {
                let y = ef_inout.eval(x);
                assert!(y < x, "EaseFunction.{ef_inout:?}({x:?}) was {y:?}");

                let iny = ef_in.eval(2.0 * x) / 2.0;
                assert!(
                    (y - TOLERANCE..y + TOLERANCE).contains(&iny),
                    "EaseFunction.{ef_inout:?}({x:?}) was {y:?}, but \
                    EaseFunction.{ef_in:?}(2 * {x:?}) / 2 was {iny:?}",
                );
            }

            for x in [0.6, 0.7, 0.8, 0.9] {
                let y = ef_inout.eval(x);
                assert!(y > x, "EaseFunction.{ef_inout:?}({x:?}) was {y:?}");

                let outy = ef_out.eval(2.0 * x - 1.0) / 2.0 + 0.5;
                assert!(
                    (y - TOLERANCE..y + TOLERANCE).contains(&outy),
                    "EaseFunction.{ef_inout:?}({x:?}) was {y:?}, but \
                    EaseFunction.{ef_out:?}(2 * {x:?} - 1) / 2 + ½ was {outy:?}",
                );
            }
        }
    }

    #[test]
    fn ease_function_midpoints() {
        for [ef_in, ef_out, ef_inout] in MONOTONIC_IN_OUT_INOUT {
            let mid = ef_in.eval(0.5);
            assert!(
                mid < 0.5 - TOLERANCE,
                "EaseFunction.{ef_in:?}(½) was {mid:?}",
            );

            let mid = ef_out.eval(0.5);
            assert!(
                mid > 0.5 + TOLERANCE,
                "EaseFunction.{ef_out:?}(½) was {mid:?}",
            );

            let mid = ef_inout.eval(0.5);
            assert!(
                (0.5 - TOLERANCE..=0.5 + TOLERANCE).contains(&mid),
                "EaseFunction.{ef_inout:?}(½) was {mid:?}",
            );
        }
    }

    #[test]
    fn ease_quats() {
        let quat_start = Quat::from_axis_angle(Vec3::Z, 0.0);
        let quat_end = Quat::from_axis_angle(Vec3::Z, 90.0_f32.to_radians());

        let quat_curve = Quat::interpolating_curve_unbounded(quat_start, quat_end);

        assert_abs_diff_eq!(
            quat_curve.sample(0.0).unwrap(),
            Quat::from_axis_angle(Vec3::Z, 0.0)
        );
        {
            let (before_mid_axis, before_mid_angle) =
                quat_curve.sample(0.25).unwrap().to_axis_angle();
            assert_abs_diff_eq!(before_mid_axis, Vec3::Z);
            assert_abs_diff_eq!(before_mid_angle, 22.5_f32.to_radians());
        }
        {
            let (mid_axis, mid_angle) = quat_curve.sample(0.5).unwrap().to_axis_angle();
            assert_abs_diff_eq!(mid_axis, Vec3::Z);
            assert_abs_diff_eq!(mid_angle, 45.0_f32.to_radians());
        }
        {
            let (after_mid_axis, after_mid_angle) =
                quat_curve.sample(0.75).unwrap().to_axis_angle();
            assert_abs_diff_eq!(after_mid_axis, Vec3::Z);
            assert_abs_diff_eq!(after_mid_angle, 67.5_f32.to_radians());
        }
        assert_abs_diff_eq!(
            quat_curve.sample(1.0).unwrap(),
            Quat::from_axis_angle(Vec3::Z, 90.0_f32.to_radians())
        );
    }

    #[test]
    fn ease_isometries_2d() {
        let angle = 90.0;
        let iso_2d_start = Isometry2d::new(Vec2::ZERO, Rot2::degrees(0.0));
        let iso_2d_end = Isometry2d::new(Vec2::ONE, Rot2::degrees(angle));

        let iso_2d_curve = Isometry2d::interpolating_curve_unbounded(iso_2d_start, iso_2d_end);

        [-1.0, 0.0, 0.5, 1.0, 2.0].into_iter().for_each(|t| {
            assert_abs_diff_eq!(
                iso_2d_curve.sample(t).unwrap(),
                Isometry2d::new(Vec2::ONE * t, Rot2::degrees(angle * t))
            );
        });
    }

    #[test]
    fn ease_isometries_3d() {
        let angle = 90.0_f32.to_radians();
        let iso_3d_start = Isometry3d::new(Vec3A::ZERO, Quat::from_axis_angle(Vec3::Z, 0.0));
        let iso_3d_end = Isometry3d::new(Vec3A::ONE, Quat::from_axis_angle(Vec3::Z, angle));

        let iso_3d_curve = Isometry3d::interpolating_curve_unbounded(iso_3d_start, iso_3d_end);

        [-1.0, 0.0, 0.5, 1.0, 2.0].into_iter().for_each(|t| {
            assert_abs_diff_eq!(
                iso_3d_curve.sample(t).unwrap(),
                Isometry3d::new(Vec3A::ONE * t, Quat::from_axis_angle(Vec3::Z, angle * t))
            );
        });
    }

    #[test]
    fn jump_at_start() {
        let jump_at = JumpAt::Start;
        let num_steps = 4;

        [
            (0.0, 0.25),
            (0.249, 0.25),
            (0.25, 0.5),
            (0.499, 0.5),
            (0.5, 0.75),
            (0.749, 0.75),
            (0.75, 1.0),
            (1.0, 1.0),
        ]
        .into_iter()
        .for_each(|(t, expected)| {
            assert_abs_diff_eq!(jump_at.eval(num_steps, t), expected);
        });
    }

    #[test]
    fn jump_at_end() {
        let jump_at = JumpAt::End;
        let num_steps = 4;

        [
            (0.0, 0.0),
            (0.249, 0.0),
            (0.25, 0.25),
            (0.499, 0.25),
            (0.5, 0.5),
            (0.749, 0.5),
            (0.75, 0.75),
            (0.999, 0.75),
            (1.0, 1.0),
        ]
        .into_iter()
        .for_each(|(t, expected)| {
            assert_abs_diff_eq!(jump_at.eval(num_steps, t), expected);
        });
    }

    #[test]
    fn jump_at_none() {
        let jump_at = JumpAt::None;
        let num_steps = 5;

        [
            (0.0, 0.0),
            (0.199, 0.0),
            (0.2, 0.25),
            (0.399, 0.25),
            (0.4, 0.5),
            (0.599, 0.5),
            (0.6, 0.75),
            (0.799, 0.75),
            (0.8, 1.0),
            (0.999, 1.0),
            (1.0, 1.0),
        ]
        .into_iter()
        .for_each(|(t, expected)| {
            assert_abs_diff_eq!(jump_at.eval(num_steps, t), expected);
        });
    }

    #[test]
    fn jump_at_both() {
        let jump_at = JumpAt::Both;
        let num_steps = 4;

        [
            (0.0, 0.2),
            (0.249, 0.2),
            (0.25, 0.4),
            (0.499, 0.4),
            (0.5, 0.6),
            (0.749, 0.6),
            (0.75, 0.8),
            (0.999, 0.8),
            (1.0, 1.0),
        ]
        .into_iter()
        .for_each(|(t, expected)| {
            assert_abs_diff_eq!(jump_at.eval(num_steps, t), expected);
        });
    }

    #[test]
    fn ease_function_curve() {
        // Test that the various ways to build an ease function are all
        // equivalent.

        let f0 = SmoothStepCurve;
        let f1 = EaseFunction::SmoothStep;
        let f2 = EasingCurve::new(0.0, 1.0, EaseFunction::SmoothStep);

        assert_eq!(f0.domain(), f1.domain());
        assert_eq!(f0.domain(), f2.domain());

        [
            -1.0,
            -f32::MIN_POSITIVE,
            0.0,
            0.5,
            1.0,
            1.0 + f32::EPSILON,
            2.0,
        ]
        .into_iter()
        .for_each(|t| {
            assert_eq!(f0.sample(t), f1.sample(t));
            assert_eq!(f0.sample(t), f2.sample(t));

            assert_eq!(f0.sample_clamped(t), f1.sample_clamped(t));
            assert_eq!(f0.sample_clamped(t), f2.sample_clamped(t));
        });
    }

    #[test]
    fn unit_structs_match_function() {
        // Test that the unit structs and `EaseFunction` match each other and
        // implement `Curve<f32>`.

        fn test(f1: impl Curve<f32>, f2: impl Curve<f32>, t: f32) {
            assert_eq!(f1.sample(t), f2.sample(t));
        }

        for t in [-1.0, 0.0, 0.25, 0.5, 0.75, 1.0, 2.0] {
            test(LinearCurve, EaseFunction::Linear, t);
            test(QuadraticInCurve, EaseFunction::QuadraticIn, t);
            test(QuadraticOutCurve, EaseFunction::QuadraticOut, t);
            test(QuadraticInOutCurve, EaseFunction::QuadraticInOut, t);
            test(CubicInCurve, EaseFunction::CubicIn, t);
            test(CubicOutCurve, EaseFunction::CubicOut, t);
            test(CubicInOutCurve, EaseFunction::CubicInOut, t);
            test(QuarticInCurve, EaseFunction::QuarticIn, t);
            test(QuarticOutCurve, EaseFunction::QuarticOut, t);
            test(QuarticInOutCurve, EaseFunction::QuarticInOut, t);
            test(QuinticInCurve, EaseFunction::QuinticIn, t);
            test(QuinticOutCurve, EaseFunction::QuinticOut, t);
            test(QuinticInOutCurve, EaseFunction::QuinticInOut, t);
            test(SmoothStepInCurve, EaseFunction::SmoothStepIn, t);
            test(SmoothStepOutCurve, EaseFunction::SmoothStepOut, t);
            test(SmoothStepCurve, EaseFunction::SmoothStep, t);
            test(SmootherStepInCurve, EaseFunction::SmootherStepIn, t);
            test(SmootherStepOutCurve, EaseFunction::SmootherStepOut, t);
            test(SmootherStepCurve, EaseFunction::SmootherStep, t);
            test(SineInCurve, EaseFunction::SineIn, t);
            test(SineOutCurve, EaseFunction::SineOut, t);
            test(SineInOutCurve, EaseFunction::SineInOut, t);
            test(CircularInCurve, EaseFunction::CircularIn, t);
            test(CircularOutCurve, EaseFunction::CircularOut, t);
            test(CircularInOutCurve, EaseFunction::CircularInOut, t);
            test(ExponentialInCurve, EaseFunction::ExponentialIn, t);
            test(ExponentialOutCurve, EaseFunction::ExponentialOut, t);
            test(ExponentialInOutCurve, EaseFunction::ExponentialInOut, t);
            test(ElasticInCurve, EaseFunction::ElasticIn, t);
            test(ElasticOutCurve, EaseFunction::ElasticOut, t);
            test(ElasticInOutCurve, EaseFunction::ElasticInOut, t);
            test(BackInCurve, EaseFunction::BackIn, t);
            test(BackOutCurve, EaseFunction::BackOut, t);
            test(BackInOutCurve, EaseFunction::BackInOut, t);
            test(BounceInCurve, EaseFunction::BounceIn, t);
            test(BounceOutCurve, EaseFunction::BounceOut, t);
            test(BounceInOutCurve, EaseFunction::BounceInOut, t);

            test(
                StepsCurve(4, JumpAt::Start),
                EaseFunction::Steps(4, JumpAt::Start),
                t,
            );

            test(ElasticCurve(50.0), EaseFunction::Elastic(50.0), t);
        }
    }
}
