//! Module containing different [`Easing`] curves to control the transition between two values and
//! the [`EasingCurve`] struct to make use of them.

use crate::{Dir2, Dir3, Dir3A, Quat, Rot2, VectorSpace};
use interpolation::Ease as IEase;

use super::{function_curve, Curve, Interval};

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
    fn interpolating_curve_unbounded(start: &Self, end: &Self) -> impl Curve<Self>;
}

impl<V: VectorSpace> Ease for V {
    fn interpolating_curve_unbounded(start: &Self, end: &Self) -> impl Curve<Self> {
        function_curve(Interval::EVERYWHERE, |t| V::lerp(*start, *end, t))
    }
}

impl Ease for Rot2 {
    fn interpolating_curve_unbounded(start: &Self, end: &Self) -> impl Curve<Self> {
        function_curve(Interval::EVERYWHERE, |t| Rot2::slerp(*start, *end, t))
    }
}

impl Ease for Quat {
    fn interpolating_curve_unbounded(start: &Self, end: &Self) -> impl Curve<Self> {
        // TODO: Check this actually extrapolates correctly.
        function_curve(Interval::EVERYWHERE, |t| Quat::slerp(*start, *end, t))
    }
}

impl Ease for Dir2 {
    fn interpolating_curve_unbounded(start: &Self, end: &Self) -> impl Curve<Self> {
        function_curve(Interval::EVERYWHERE, |t| Dir2::slerp(*start, *end, t))
    }
}

impl Ease for Dir3 {
    fn interpolating_curve_unbounded(start: &Self, end: &Self) -> impl Curve<Self> {
        // TODO: Check this actually extrapolates correctly.
        function_curve(Interval::EVERYWHERE, |t| Dir3::slerp(*start, *end, t))
    }
}

impl Ease for Dir3A {
    fn interpolating_curve_unbounded(start: &Self, end: &Self) -> impl Curve<Self> {
        // TODO: Check this actually extrapolates correctly.
        function_curve(Interval::EVERYWHERE, |t| Dir3A::slerp(*start, *end, t))
    }
}

/// Given a `start` and `end` value, create a curve parametrized over [the unit interval]
/// that connects them, using the given [ease function] to determine the form of the
/// curve in between.
///
/// [the unit interval]: Interval::UNIT
/// [ease function]: EaseFunction
pub fn easing_curve<T: Ease>(start: T, end: T, ease_fn: EaseFunction) -> EasingCurve<T> {
    EasingCurve {
        start,
        end,
        ease_fn,
    }
}

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
pub struct EasingCurve<T>
where
    T: Ease,
{
    start: T,
    end: T,
    ease_fn: EaseFunction,
}

impl<T> Curve<T> for EasingCurve<T>
where
    T: Ease,
{
    #[inline]
    fn domain(&self) -> Interval {
        Interval::UNIT
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        let remapped_t = self.ease_fn.eval(t);
        T::interpolating_curve_unbounded(&self.start, &self.end).sample_unchecked(remapped_t)
    }
}

/// Curve functions over the [unit interval], commonly used for easing transitions.
///
/// [unit interval]: `Interval::UNIT`
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub enum EaseFunction {
    /// `f(t) = t`
    Linear,

    /// `f(t) = t²`
    QuadraticIn,
    /// `f(t) = -(t * (t - 2.0))`
    QuadraticOut,
    /// Behaves as `EaseFunction::QuadraticIn` for t < 0.5 and as `EaseFunction::QuadraticOut` for t >= 0.5
    QuadraticInOut,

    /// `f(t) = t³`
    CubicIn,
    /// `f(t) = (t - 1.0)³ + 1.0`
    CubicOut,
    /// Behaves as `EaseFunction::CubicIn` for t < 0.5 and as `EaseFunction::CubicOut` for t >= 0.5
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
    QuinticInOut,

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

    /// `f(t) = 2.0^(10.0 * (t - 1.0))`
    ExponentialIn,
    /// `f(t) = 1.0 - 2.0^(-10.0 * t)`
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
    pub(crate) fn sine_in(t: f32) -> f32 {
        1.0 - ops::cos(t * FRAC_PI_2)
    }
    #[inline]
    pub(crate) fn sine_out(t: f32) -> f32 {
        ops::sin(t * FRAC_PI_2)
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
    pub(crate) fn steps(num_steps: usize, t: f32) -> f32 {
        (t * num_steps as f32).round() / num_steps.max(1) as f32
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
            EaseFunction::QuadraticIn => IEase::quadratic_in(t),
            EaseFunction::QuadraticOut => IEase::quadratic_out(t),
            EaseFunction::QuadraticInOut => IEase::quadratic_in_out(t),
            EaseFunction::CubicIn => IEase::cubic_in(t),
            EaseFunction::CubicOut => IEase::cubic_out(t),
            EaseFunction::CubicInOut => IEase::cubic_in_out(t),
            EaseFunction::QuarticIn => IEase::quartic_in(t),
            EaseFunction::QuarticOut => IEase::quartic_out(t),
            EaseFunction::QuarticInOut => IEase::quartic_in_out(t),
            EaseFunction::QuinticIn => IEase::quintic_in(t),
            EaseFunction::QuinticOut => IEase::quintic_out(t),
            EaseFunction::QuinticInOut => IEase::quintic_in_out(t),
            EaseFunction::SineIn => easing_functions::sine_in(t),
            EaseFunction::SineOut => easing_functions::sine_out(t),
            EaseFunction::SineInOut => IEase::sine_in_out(t),
            EaseFunction::CircularIn => IEase::circular_in(t),
            EaseFunction::CircularOut => IEase::circular_out(t),
            EaseFunction::CircularInOut => IEase::circular_in_out(t),
            EaseFunction::ExponentialIn => IEase::exponential_in(t),
            EaseFunction::ExponentialOut => IEase::exponential_out(t),
            EaseFunction::ExponentialInOut => IEase::exponential_in_out(t),
            EaseFunction::ElasticIn => easing_functions::elastic_in(t),
            EaseFunction::ElasticOut => easing_functions::elastic_out(t),
            EaseFunction::ElasticInOut => easing_functions::elastic_in_out(t),
            EaseFunction::BackIn => easing_functions::back_in(t),
            EaseFunction::BackOut => easing_functions::back_out(t),
            EaseFunction::BackInOut => easing_functions::back_in_out(t),
            EaseFunction::BounceIn => IEase::bounce_in(t),
            EaseFunction::BounceOut => IEase::bounce_out(t),
            EaseFunction::BounceInOut => IEase::bounce_in_out(t),
            EaseFunction::Steps(num_steps) => easing_functions::steps(*num_steps, t),
            EaseFunction::Elastic(omega) => easing_functions::elastic(*omega, t),
        }
    }
}
