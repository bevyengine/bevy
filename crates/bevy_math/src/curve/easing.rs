//! Module containing different [`Easing`] curves to control the transition between two values and
//! the [`EasingCurve`] struct to make use of them.

use crate::{
    ops::{self, FloatPow},
    VectorSpace,
};
use interpolation::Ease;

use super::{Curve, FunctionCurve, Interval};

/// A trait for [`Curves`] that map the [unit interval] to some other values. These kinds of curves
/// are used to create a transition between two values. Easing curves are most commonly known from
/// [CSS animations] but are also widely used in other fields.
///
/// [unit interval]: `Interval::UNIT`
/// [`Curves`]: `Curve`
/// [CSS animations]: https://developer.mozilla.org/en-US/docs/Web/CSS/easing-function
pub trait Easing<T>: Curve<T> {}
impl<T: VectorSpace, C: Curve<f32>> Easing<T> for EasingCurve<T, C> {}
impl<T: VectorSpace> Easing<T> for LinearCurve<T> {}
impl Easing<f32> for StepCurve {}
impl Easing<f32> for ElasticCurve {}

/// A [`Curve`] that is defined by
///
/// - an initial `start` sample value at `t = 0`
/// - a final `end` sample value at `t = 1`
/// - an [`EasingCurve`] to interpolate between the two values within the [unit interval].
///
/// [unit interval]: `Interval::UNIT`
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct EasingCurve<T, E>
where
    T: VectorSpace,
    E: Curve<f32>,
{
    start: T,
    end: T,
    easing: E,
}

impl<T, E> Curve<T> for EasingCurve<T, E>
where
    T: VectorSpace,
    E: Curve<f32>,
{
    #[inline]
    fn domain(&self) -> Interval {
        Interval::UNIT
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        let domain = self.easing.domain();
        let t = domain.start().lerp(domain.end(), t);
        self.start.lerp(self.end, self.easing.sample_unchecked(t))
    }
}

impl<T, E> EasingCurve<T, E>
where
    T: VectorSpace,
    E: Curve<f32>,
{
    /// Create a new [`EasingCurve`] over the [unit interval] which transitions between a `start`
    /// and an `end` value based on the provided [`Curve<f32>`] curve.
    ///
    /// If the input curve's domain is not the unit interval, then the [`EasingCurve`] will ensure
    /// that this invariant is guaranteed by internally [reparametrizing] the curve to the unit
    /// interval.
    ///
    /// [`Curve<f32>`]: `Curve`
    /// [unit interval]: `Interval::UNIT`
    /// [reparametrizing]: `Curve::reparametrize_linear`
    pub fn new(start: T, end: T, easing: E) -> Result<Self, EasingCurveError> {
        easing
            .domain()
            .is_bounded()
            .then_some(Self { start, end, easing })
            .ok_or(EasingCurveError)
    }
}

impl EasingCurve<f32, FunctionCurve<f32, fn(f32) -> f32>> {
    /// A [`Curve`] mapping the [unit interval] to itself.
    ///
    /// Quadratic easing functions can have exactly one critical point. This is a point on the function
    /// such that `f′(t) = 0`. This means that there won't be any sudden jumps at this point leading to
    /// smooth transitions. A common choice is to place that point at `t = 0` or [`t = 1`].
    ///
    /// It uses the function `f(t) = t²`
    ///
    /// [unit interval]: `Interval::UNIT`
    /// [`t = 1`]: `Self::quadratic_ease_out`
    pub fn quadratic_ease_in() -> Self {
        Self {
            start: 0.0,
            end: 1.0,
            easing: FunctionCurve::new(Interval::UNIT, FloatPow::squared),
        }
    }

    /// A [`Curve`] mapping the [unit interval] to itself.
    ///
    /// Quadratic easing functions can have exactly one critical point. This is a point on the function
    /// such that `f′(t) = 0`. This means that there won't be any sudden jumps at this point leading to
    /// smooth transitions. A common choice is to place that point at [`t = 0`] or`t = 1`.
    ///
    /// It uses the function `f(t) = 1 - (1 - t)²`
    ///
    /// [unit interval]: `Interval::UNIT`
    /// [`t = 0`]: `Self::quadratic_ease_in`
    pub fn quadratic_ease_out() -> Self {
        fn f(t: f32) -> f32 {
            1.0 - (1.0 - t).squared()
        }
        Self {
            start: 0.0,
            end: 1.0,
            easing: FunctionCurve::new(Interval::UNIT, f),
        }
    }

    /// A [`Curve`] mapping the [unit interval] to itself.
    ///
    /// Cubic easing functions can have up to two critical points. These are points on the function
    /// such that `f′(t) = 0`. This means that there won't be any sudden jumps at these points leading to
    /// smooth transitions. For this curve they are placed at `t = 0` and `t = 1` respectively and the
    /// result is a well-known kind of [sigmoid function] called a [smoothstep function].
    ///
    /// It uses the function `f(t) = t² * (3 - 2t)`
    ///
    /// [unit interval]: `Interval::UNIT`
    /// [sigmoid function]: https://en.wikipedia.org/wiki/Sigmoid_function
    /// [smoothstep function]: https://en.wikipedia.org/wiki/Smoothstep
    pub fn smoothstep() -> Self {
        fn f(t: f32) -> f32 {
            t.squared() * (3.0 - 2.0 * t)
        }
        Self {
            start: 0.0,
            end: 1.0,
            easing: FunctionCurve::new(Interval::UNIT, f),
        }
    }

    /// A [`Curve`] mapping the [unit interval] to itself.
    ///
    /// It uses the function `f(t) = t`
    ///
    /// [unit interval]: `Interval::UNIT`
    pub fn identity() -> Self {
        Self {
            start: 0.0,
            end: 1.0,
            easing: FunctionCurve::new(Interval::UNIT, core::convert::identity),
        }
    }
}

/// An error that occurs if the construction of [`EasingCurve`] fails
#[derive(Debug, thiserror::Error)]
#[error("Easing curves can only be constructed from curves with bounded domain")]
pub struct EasingCurveError;

/// A [`Curve`] that is defined by a `start` and an `end` point, together with linear interpolation
/// between the values over the [unit interval]. It's basically an [`EasingCurve`] with the
/// identity as an easing function.
///
/// [unit interval]: `Interval::UNIT`
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct LinearCurve<T: VectorSpace> {
    start: T,
    end: T,
}

impl<T> Curve<T> for LinearCurve<T>
where
    T: VectorSpace,
{
    #[inline]
    fn domain(&self) -> Interval {
        Interval::UNIT
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        self.start.lerp(self.end, t)
    }
}

impl<T> LinearCurve<T>
where
    T: VectorSpace,
{
    /// Create a new [`LinearCurve`] over the [unit interval] from `start` to `end`.
    ///
    /// [unit interval]: `Interval::UNIT`
    pub fn new(start: T, end: T) -> Self {
        Self { start, end }
    }
}

/// A [`Curve`] mapping the [unit interval] to itself.
///
/// This leads to a cruve with sudden jumps at the step points and segments with constant values
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

    /// `f(t) = sin((t - 1.0) * π / 2.0) + 1.0`
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

    /// `f(t) = 2.0.powf(10.0 * (t - 1.0))`
    ExponentialIn,
    /// `f(t) = 1.0 - 2.0.powf(-10.0 * t)`
    ExponentialOut,
    /// Behaves as `EaseFunction::ExponentialIn` for t < 0.5 and as `EaseFunction::ExponentialOut` for t >= 0.5
    ExponentialInOut,

    /// `f(t) = sin(13.0 * π / 2.0 * t) * 2.0.powf(10.0 * (t - 1.0))`
    ElasticIn,
    /// `f(t) = sin(-13.0 * π / 2.0 * (t + 1.0)) * 2.0.powf(-10.0 * t) + 1.0`
    ElasticOut,
    /// Behaves as `EaseFunction::ElasticIn` for t < 0.5 and as `EaseFunction::ElasticOut` for t >= 0.5
    ElasticInOut,

    /// `f(t) = t³ - t * sin(t * π)`
    BackIn,
    /// `f(t) = 1.0 - (1.0 - t)³ - t * sin((1.0 - t) * π))`
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

impl Easing<f32> for EaseFunction {}
impl Curve<f32> for EaseFunction {
    fn domain(&self) -> Interval {
        Interval::UNIT
    }

    fn sample_unchecked(&self, t: f32) -> f32 {
        match self {
            EaseFunction::QuadraticIn => t.calc(interpolation::EaseFunction::QuadraticIn),
            EaseFunction::QuadraticOut => t.calc(interpolation::EaseFunction::QuadraticOut),
            EaseFunction::QuadraticInOut => t.calc(interpolation::EaseFunction::QuadraticInOut),
            EaseFunction::CubicIn => t.calc(interpolation::EaseFunction::CubicIn),
            EaseFunction::CubicOut => t.calc(interpolation::EaseFunction::CubicOut),
            EaseFunction::CubicInOut => t.calc(interpolation::EaseFunction::CubicInOut),
            EaseFunction::QuarticIn => t.calc(interpolation::EaseFunction::QuarticIn),
            EaseFunction::QuarticOut => t.calc(interpolation::EaseFunction::QuarticOut),
            EaseFunction::QuarticInOut => t.calc(interpolation::EaseFunction::QuarticInOut),
            EaseFunction::QuinticIn => t.calc(interpolation::EaseFunction::QuinticIn),
            EaseFunction::QuinticOut => t.calc(interpolation::EaseFunction::QuinticOut),
            EaseFunction::QuinticInOut => t.calc(interpolation::EaseFunction::QuinticInOut),
            EaseFunction::SineIn => t.calc(interpolation::EaseFunction::SineIn),
            EaseFunction::SineOut => t.calc(interpolation::EaseFunction::SineOut),
            EaseFunction::SineInOut => t.calc(interpolation::EaseFunction::SineInOut),
            EaseFunction::CircularIn => t.calc(interpolation::EaseFunction::CircularIn),
            EaseFunction::CircularOut => t.calc(interpolation::EaseFunction::CircularOut),
            EaseFunction::CircularInOut => t.calc(interpolation::EaseFunction::CircularInOut),
            EaseFunction::ExponentialIn => t.calc(interpolation::EaseFunction::ExponentialIn),
            EaseFunction::ExponentialOut => t.calc(interpolation::EaseFunction::ExponentialOut),
            EaseFunction::ExponentialInOut => t.calc(interpolation::EaseFunction::ExponentialInOut),
            EaseFunction::ElasticIn => t.calc(interpolation::EaseFunction::ElasticIn),
            EaseFunction::ElasticOut => t.calc(interpolation::EaseFunction::ElasticOut),
            EaseFunction::ElasticInOut => t.calc(interpolation::EaseFunction::ElasticInOut),
            EaseFunction::BackIn => t.calc(interpolation::EaseFunction::BackIn),
            EaseFunction::BackOut => t.calc(interpolation::EaseFunction::BackOut),
            EaseFunction::BackInOut => t.calc(interpolation::EaseFunction::BackInOut),
            EaseFunction::BounceIn => t.calc(interpolation::EaseFunction::BounceIn),
            EaseFunction::BounceOut => t.calc(interpolation::EaseFunction::BounceOut),
            EaseFunction::BounceInOut => t.calc(interpolation::EaseFunction::BounceInOut),
        }
    }
}
