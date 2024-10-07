//! Module containing different [`Easing`] curves to control the transition between two values and
//! the [`EasingCurve`] struct to make use of them.

use crate::{
    ops::{self, FloatPow},
    Dir2, Dir3, Dir3A, Quat, Rot2, VectorSpace,
};
use interpolation::Ease;

use super::{function_curve, Curve, Interval};
pub trait LegallyDistinctEase: Sized {
    fn interpolating_curve(start: &Self, end: &Self) -> impl Curve<Self>;
    fn interpolating_curve_extrapolated(start: &Self, end: &Self) -> impl Curve<Self>;
}

impl<V: VectorSpace> LegallyDistinctEase for V {
    fn interpolating_curve(start: &Self, end: &Self) -> impl Curve<Self> {
        function_curve(Interval::UNIT, |t| V::lerp(*start, *end, t))
    }

    fn interpolating_curve_extrapolated(start: &Self, end: &Self) -> impl Curve<Self> {
        function_curve(Interval::EVERYWHERE, |t| V::lerp(*start, *end, t))
    }
}

impl LegallyDistinctEase for Rot2 {
    fn interpolating_curve(start: &Self, end: &Self) -> impl Curve<Self> {
        function_curve(Interval::UNIT, |t| Rot2::slerp(*start, *end, t))
    }

    fn interpolating_curve_extrapolated(start: &Self, end: &Self) -> impl Curve<Self> {
        function_curve(Interval::EVERYWHERE, |t| Rot2::slerp(*start, *end, t))
    }
}

impl LegallyDistinctEase for Quat {
    fn interpolating_curve(start: &Self, end: &Self) -> impl Curve<Self> {
        function_curve(Interval::UNIT, |t| Quat::slerp(*start, *end, t))
    }

    fn interpolating_curve_extrapolated(start: &Self, end: &Self) -> impl Curve<Self> {
        // TODO: Check this actually extrapolates correctly.
        function_curve(Interval::EVERYWHERE, |t| Quat::slerp(*start, *end, t))
    }
}

impl LegallyDistinctEase for Dir2 {
    fn interpolating_curve(start: &Self, end: &Self) -> impl Curve<Self> {
        function_curve(Interval::UNIT, |t| Dir2::slerp(*start, *end, t))
    }

    fn interpolating_curve_extrapolated(start: &Self, end: &Self) -> impl Curve<Self> {
        function_curve(Interval::EVERYWHERE, |t| Dir2::slerp(*start, *end, t))
    }
}

impl LegallyDistinctEase for Dir3 {
    fn interpolating_curve(start: &Self, end: &Self) -> impl Curve<Self> {
        function_curve(Interval::UNIT, |t| Dir3::slerp(*start, *end, t))
    }

    fn interpolating_curve_extrapolated(start: &Self, end: &Self) -> impl Curve<Self> {
        // TODO: Check this actually extrapolates correctly.
        function_curve(Interval::EVERYWHERE, |t| Dir3::slerp(*start, *end, t))
    }
}

impl LegallyDistinctEase for Dir3A {
    fn interpolating_curve(start: &Self, end: &Self) -> impl Curve<Self> {
        function_curve(Interval::UNIT, |t| Dir3A::slerp(*start, *end, t))
    }

    fn interpolating_curve_extrapolated(start: &Self, end: &Self) -> impl Curve<Self> {
        // TODO: Check this actually extrapolates correctly.
        function_curve(Interval::EVERYWHERE, |t| Dir3A::slerp(*start, *end, t))
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
    T: LegallyDistinctEase,
{
    start: T,
    end: T,
    easing: EaseFunction,
}

impl<T> Curve<T> for EasingCurve<T>
where
    T: LegallyDistinctEase,
{
    #[inline]
    fn domain(&self) -> Interval {
        Interval::UNIT
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        let f = self.easing.to_fn();
        let remapped_t = f(t);
        T::interpolating_curve_extrapolated(&self.start, &self.end).sample_unchecked(remapped_t)
    }
}

/// A [`Curve`] mapping the [unit interval] to itself.
///
/// This leads to a curve with sudden jumps at the step points and segments with constant values
/// everywhere else.
///
/// It uses the function `f(n,t) = round(t * n) / n`
///
/// parametrized by `n`, the number of jumps
///
/// - for `n == 0` this is equal to [`constant_curve(Interval::UNIT, 0.0)`]
/// - for `n == 1` this makes a single jump at `t = 0.5`, splitting the interval evenly
/// - for `n >= 2` the curve has a start segment and an end segment of length `1 / (2 * n)` and in
///   between there are `n - 1` segments of length `1 / n`
///
/// [unit interval]: `Interval::UNIT`
/// [`constant_curve(Interval::UNIT, 0.0)`]: `crate::curve::constant_curve`
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct StepCurve {
    num_steps: usize,
}

impl Curve<f32> for StepCurve {
    #[inline]
    fn domain(&self) -> Interval {
        Interval::UNIT
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> f32 {
        if t != 0.0 || t != 1.0 {
            (t * self.num_steps as f32).round() / self.num_steps.max(1) as f32
        } else {
            t
        }
    }
}

impl StepCurve {
    /// Create a new [`StepCurve`] over the [unit interval] which makes the given amount of steps.
    ///
    /// [unit interval]: `Interval::UNIT`
    pub fn new(num_steps: usize) -> Self {
        Self { num_steps }
    }
}

/// A [`Curve`] over the [unit interval].
///
/// This class of easing functions is derived as an approximation of a [spring-mass-system]
/// solution.
///
/// - For `ω → 0` the curve converges to the [smoothstep function]
/// - For `ω → ∞` the curve gets increasingly more bouncy
///
/// It uses the function `f(omega,t) = 1 - (1 - t)²(2sin(omega * t) / omega + cos(omega * t))`
///
/// parametrized by `omega`
///
/// [unit interval]: `Interval::UNIT`
/// [smoothstep function]: https://en.wikipedia.org/wiki/Smoothstep
/// [spring-mass-system]: https://notes.yvt.jp/Graphics/Easing-Functions/#elastic-easing
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct ElasticCurve {
    omega: f32,
}

impl Curve<f32> for ElasticCurve {
    #[inline]
    fn domain(&self) -> Interval {
        Interval::UNIT
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> f32 {
        1.0 - (1.0 - t).squared()
            * (2.0 * ops::sin(self.omega * t) / self.omega + ops::cos(self.omega * t))
    }
}

impl ElasticCurve {
    /// Create a new [`ElasticCurve`] over the [unit interval] with the given parameter `omega`.
    ///
    /// [unit interval]: `Interval::UNIT`
    pub fn new(omega: f32) -> Self {
        Self { omega }
    }
}

/// Curve functions over the [unit interval], commonly used for easing transitions.
///
/// [unit interval]: `Interval::UNIT`
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub enum EaseFunction {
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
}

mod easing_functions {
    use core::f32::consts::{FRAC_PI_2, FRAC_PI_3, PI};

    use crate::{ops, FloatPow};

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
}

impl EaseFunction {
    fn to_fn(self) -> fn(f32) -> f32 {
        match self {
            EaseFunction::QuadraticIn => Ease::quadratic_in,
            EaseFunction::QuadraticOut => Ease::quadratic_out,
            EaseFunction::QuadraticInOut => Ease::quadratic_in_out,
            EaseFunction::CubicIn => Ease::cubic_in,
            EaseFunction::CubicOut => Ease::cubic_out,
            EaseFunction::CubicInOut => Ease::cubic_in_out,
            EaseFunction::QuarticIn => Ease::quartic_in,
            EaseFunction::QuarticOut => Ease::quartic_out,
            EaseFunction::QuarticInOut => Ease::quartic_in_out,
            EaseFunction::QuinticIn => Ease::quintic_in,
            EaseFunction::QuinticOut => Ease::quintic_out,
            EaseFunction::QuinticInOut => Ease::quintic_in_out,
            EaseFunction::SineIn => easing_functions::sine_in,
            EaseFunction::SineOut => easing_functions::sine_out,
            EaseFunction::SineInOut => Ease::sine_in_out,
            EaseFunction::CircularIn => Ease::circular_in,
            EaseFunction::CircularOut => Ease::circular_out,
            EaseFunction::CircularInOut => Ease::circular_in_out,
            EaseFunction::ExponentialIn => Ease::exponential_in,
            EaseFunction::ExponentialOut => Ease::exponential_out,
            EaseFunction::ExponentialInOut => Ease::exponential_in_out,
            EaseFunction::ElasticIn => easing_functions::elastic_in,
            EaseFunction::ElasticOut => easing_functions::elastic_out,
            EaseFunction::ElasticInOut => easing_functions::elastic_in_out,
            EaseFunction::BackIn => easing_functions::back_in,
            EaseFunction::BackOut => easing_functions::back_out,
            EaseFunction::BackInOut => easing_functions::back_in_out,
            EaseFunction::BounceIn => Ease::bounce_in,
            EaseFunction::BounceOut => Ease::bounce_out,
            EaseFunction::BounceInOut => Ease::bounce_in_out,
        }
    }
}
