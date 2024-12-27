//! Module containing different [easing functions] to control the transition between two values and
//! the [`EasingCurve`] struct to make use of them.
//!
//! [easing functions]: EaseFunction

use crate::{
    curve::{FunctionCurve, Interval},
    Curve, Dir2, Dir3, Dir3A, Quat, Rot2, VectorSpace,
};

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

impl<V: VectorSpace> Ease for V {
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
/// [easing function]: EaseFunction
/// [the unit interval]: Interval::UNIT
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

/// Curve functions over the [unit interval], commonly used for easing transitions.
///
/// [unit interval]: `Interval::UNIT`
#[non_exhaustive]
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub enum EaseFunction {
    /// `f(t) = t`
    Linear,

    /// `f(t) = t²`
    ///
    /// This is the Hermite interpolator for
    /// - f(0) = 0
    /// - f(1) = 1
    /// - f′(0) = 0
    QuadraticIn,
    /// `f(t) = -(t * (t - 2.0))`
    ///
    /// This is the Hermite interpolator for
    /// - f(0) = 0
    /// - f(1) = 1
    /// - f′(1) = 0
    QuadraticOut,
    /// Behaves as `EaseFunction::QuadraticIn` for t < 0.5 and as `EaseFunction::QuadraticOut` for t >= 0.5
    ///
    /// A quadratic has too low of a degree to be both an `InOut` and C²,
    /// so consider using at least a cubic (such as [`EaseFunction::SmoothStep`])
    /// if you want the acceleration to be continuous.
    QuadraticInOut,

    /// `f(t) = t³`
    ///
    /// This is the Hermite interpolator for
    /// - f(0) = 0
    /// - f(1) = 1
    /// - f′(0) = 0
    /// - f″(0) = 0
    CubicIn,
    /// `f(t) = (t - 1.0)³ + 1.0`
    CubicOut,
    /// Behaves as `EaseFunction::CubicIn` for t < 0.5 and as `EaseFunction::CubicOut` for t >= 0.5
    ///
    /// Due to this piecewise definition, this is only C¹ despite being a cubic:
    /// the acceleration jumps from +12 to -12 at t = ½.
    ///
    /// Consider using [`EaseFunction::SmoothStep`] instead, which is also cubic,
    /// or [`EaseFunction::SmootherStep`] if you picked this because you wanted
    /// the acceleration at the endpoints to also be zero.
    CubicInOut,

    /// `f(t) = t⁴`
    QuarticIn,
    /// `f(t) = (t - 1.0)³ * (1.0 - t) + 1.0`
    QuarticOut,
    /// Behaves as `EaseFunction::QuarticIn` for t < 0.5 and as `EaseFunction::QuarticOut` for t >= 0.5
    QuarticInOut,

    /// `f(t) = t⁵`
    QuinticIn,
    /// `f(t) = (t - 1.0)⁵ + 1.0`
    QuinticOut,
    /// Behaves as `EaseFunction::QuinticIn` for t < 0.5 and as `EaseFunction::QuinticOut` for t >= 0.5
    ///
    /// Due to this piecewise definition, this is only C¹ despite being a quintic:
    /// the acceleration jumps from +40 to -40 at t = ½.
    ///
    /// Consider using [`EaseFunction::SmootherStep`] instead, which is also quintic.
    QuinticInOut,

    /// Behaves as the first half of [`EaseFunction::SmoothStep`].
    ///
    /// This has f″(1) = 0, unlike [`EaseFunction::QuadraticIn`] which starts similarly.
    SmoothStepIn,
    /// Behaves as the second half of [`EaseFunction::SmoothStep`].
    ///
    /// This has f″(0) = 0, unlike [`EaseFunction::QuadraticOut`] which ends similarly.
    SmoothStepOut,
    /// `f(t) = 2t³ + 3t²`
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
    SmoothStep,

    /// Behaves as the first half of [`EaseFunction::SmootherStep`].
    ///
    /// This has f″(1) = 0, unlike [`EaseFunction::CubicIn`] which starts similarly.
    SmootherStepIn,
    /// Behaves as the second half of [`EaseFunction::SmootherStep`].
    ///
    /// This has f″(0) = 0, unlike [`EaseFunction::CubicOut`] which ends similarly.
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
    SmootherStep,

    /// `f(t) = 1.0 - cos(t * π / 2.0)`
    SineIn,
    /// `f(t) = sin(t * π / 2.0)`
    SineOut,
    /// Behaves as `EaseFunction::SineIn` for t < 0.5 and as `EaseFunction::SineOut` for t >= 0.5
    SineInOut,

    /// `f(t) = 1.0 - sqrt(1.0 - t²)`
    CircularIn,
    /// `f(t) = sqrt((2.0 - t) * t)`
    CircularOut,
    /// Behaves as `EaseFunction::CircularIn` for t < 0.5 and as `EaseFunction::CircularOut` for t >= 0.5
    CircularInOut,

    /// `f(t) ≈ 2.0^(10.0 * (t - 1.0))`
    ///
    /// The precise definition adjusts it slightly so it hits both `(0, 0)` and `(1, 1)`:
    /// `f(t) = 2.0^(10.0 * t - A) - B`, where A = log₂(2¹⁰-1) and B = 1/(2¹⁰-1).
    ExponentialIn,
    /// `f(t) ≈ 1.0 - 2.0^(-10.0 * t)`
    ///
    /// As with `EaseFunction::ExponentialIn`, the precise definition adjusts it slightly
    // so it hits both `(0, 0)` and `(1, 1)`.
    ExponentialOut,
    /// Behaves as `EaseFunction::ExponentialIn` for t < 0.5 and as `EaseFunction::ExponentialOut` for t >= 0.5
    ExponentialInOut,

    /// `f(t) = -2.0^(10.0 * t - 10.0) * sin((t * 10.0 - 10.75) * 2.0 * π / 3.0)`
    ElasticIn,
    /// `f(t) = 2.0^(-10.0 * t) * sin((t * 10.0 - 0.75) * 2.0 * π / 3.0) + 1.0`
    ElasticOut,
    /// Behaves as `EaseFunction::ElasticIn` for t < 0.5 and as `EaseFunction::ElasticOut` for t >= 0.5
    ElasticInOut,

    /// `f(t) = 2.70158 * t³ - 1.70158 * t²`
    BackIn,
    /// `f(t) = 1.0 +  2.70158 * (t - 1.0)³ - 1.70158 * (t - 1.0)²`
    BackOut,
    /// Behaves as `EaseFunction::BackIn` for t < 0.5 and as `EaseFunction::BackOut` for t >= 0.5
    BackInOut,

    /// bouncy at the start!
    BounceIn,
    /// bouncy at the end!
    BounceOut,
    /// Behaves as `EaseFunction::BounceIn` for t < 0.5 and as `EaseFunction::BounceOut` for t >= 0.5
    BounceInOut,

    /// `n` steps connecting the start and the end
    Steps(usize),

    /// `f(omega,t) = 1 - (1 - t)²(2sin(omega * t) / omega + cos(omega * t))`, parametrized by `omega`
    Elastic(f32),
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
    #[allow(clippy::excessive_precision)]
    const LOG2_1023: f32 = 9.998590429745328646459226;
    #[allow(clippy::excessive_precision)]
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
    pub(crate) fn steps(num_steps: usize, t: f32) -> f32 {
        ops::round(t * num_steps as f32) / num_steps.max(1) as f32
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
            EaseFunction::Steps(num_steps) => easing_functions::steps(*num_steps, t),
            EaseFunction::Elastic(omega) => easing_functions::elastic(*omega, t),
        }
    }
}

#[cfg(test)]
mod tests {
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
}
