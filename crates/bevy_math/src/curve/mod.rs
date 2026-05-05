//! The [`Curve`] trait, providing a domain-agnostic description of curves.
//!
//! ## Overview
//!
//! At a high level, [`Curve`] is a trait that abstracts away the implementation details of curves,
//! which comprise any kind of data parametrized by a single continuous variable. For example, that
//! variable could represent time, in which case a curve would represent a value that changes over
//! time, as in animation; on the other hand, it could represent something like displacement or
//! distance, as in graphs, gradients, and curves in space.
//!
//! The trait itself has two fundamental components: a curve must have a [domain], which is a nonempty
//! range of `f32` values, and it must be able to be [sampled] on every one of those values, producing
//! output of some fixed type.
//!
//! A primary goal of the trait is to allow interfaces to simply accept `impl Curve<T>` as input
//! rather than requiring for input curves to be defined in data in any particular way. This is
//! supported by a number of interface methods which allow [changing parametrizations], [mapping output],
//! and [rasterization].
//!
//! ## Analogy with `Iterator`
//!
//! The `Curve` API behaves, in many ways, like a continuous counterpart to [`Iterator`]. The analogy
//! looks something like this with some of the common methods:
//!
//! |    Iterators     |     Curves      |
//! | :--------------- | :-------------- |
//! | `map`            | `map`           |
//! | `skip`/`step_by` | `reparametrize` |
//! | `enumerate`      | `graph`         |
//! | `chain`          | `chain`         |
//! | `zip`            | `zip`           |
//! | `rev`            | `reverse`       |
//! | `by_ref`         | `by_ref`        |
//!
//! Of course, there are very important differences, as well. For instance, the continuous nature of
//! curves means that many iterator methods make little sense in the context of curves, or at least
//! require numerical techniques. For example, the analogue of `sum` would be an integral, approximated
//! by something like Riemann summation.
//!
//! Furthermore, the two also differ greatly in their orientation to borrowing and mutation:
//! iterators are mutated by being iterated, and by contrast, all curve methods are immutable. More
//! information on the implications of this can be found [below](self#Ownership-and-borrowing).
//!
//! ## Defining curves
//!
//! Curves may be defined in a number of ways. The following are common:
//! - using [functions];
//! - using [sample interpolation];
//! - using [splines];
//! - using [easings].
//!
//! Among these, the first is the most versatile[^footnote]: the domain and the sampling output are just
//! specified directly in the construction. For this reason, function curves are a reliable go-to for
//! simple one-off constructions and procedural uses, where flexibility is desirable. For example:
//! ```rust
//! # use bevy_math::vec3;
//! # use bevy_math::curve::*;
//! // A sinusoid:
//! let sine_curve = FunctionCurve::new(Interval::EVERYWHERE, f32::sin);
//!
//! // A sawtooth wave:
//! let sawtooth_curve = FunctionCurve::new(Interval::EVERYWHERE, |t| t % 1.0);
//!
//! // A helix:
//! let helix_curve = FunctionCurve::new(Interval::EVERYWHERE, |theta| vec3(theta.sin(), theta, theta.cos()));
//! ```
//!
//! Sample-interpolated curves commonly arises in both rasterization and in animation, and this library
//! has support for producing them in both fashions. See [below](self#Resampling-and-rasterization) for
//! more information about rasterization. Here is what an explicit sample-interpolated curve might look like:
//! ```rust
//! # use bevy_math::prelude::*;
//! # use std::f32::consts::FRAC_PI_2;
//! // A list of angles that we want to traverse:
//! let angles = [
//!     0.0,
//!     -FRAC_PI_2,
//!     0.0,
//!     FRAC_PI_2,
//!     0.0
//! ];
//!
//! // Make each angle into a rotation by that angle:
//! let rotations = angles.map(|angle| Rot2::radians(angle));
//!
//! // Interpolate these rotations with a `Rot2`-valued curve:
//! let rotation_curve = SampleAutoCurve::new(interval(0.0, 4.0).unwrap(), rotations).unwrap();
//! ```
//!
//! For more information on [spline curves] and [easing curves], see their respective modules.
//!
//! And, of course, you are also free to define curve types yourself, implementing the trait directly.
//! For custom sample-interpolated curves, the [`cores`] submodule provides machinery to avoid having to
//! reimplement interpolation logic yourself. In many other cases, implementing the trait directly is
//! often quite straightforward:
//! ```rust
//! # use bevy_math::prelude::*;
//! struct ExponentialCurve {
//!     exponent: f32,
//! }
//!
//! impl Curve<f32> for ExponentialCurve {
//!     fn domain(&self) -> Interval {
//!         Interval::EVERYWHERE
//!     }
//!
//!     fn sample_unchecked(&self, t: f32) -> f32 {
//!         f32::exp(self.exponent * t)
//!     }
//!
//!     // All other trait methods can be inferred from these.
//! }
//! ```
//!
//! ## Transforming curves
//!
//! The API provides a few key ways of transforming one curve into another. These are often useful when
//! you would like to make use of an interface that requires a curve that bears some logical relationship
//! to one that you already have access to, but with different requirements or expectations. For example,
//! the output type of the curves may differ, or the domain may be expected to be different. The `map`
//! and `reparametrize` methods can help address this.
//!
//! As a simple example of the kind of thing that arises in practice, let's imagine that we have a
//! `Curve<Vec2>` that we want to use to describe the motion of some object over time, but the interface
//! for animation expects a `Curve<Vec3>`, since the object will move in three dimensions:
//! ```rust
//! # use bevy_math::{vec2, prelude::*};
//! # use std::f32::consts::TAU;
//! // Our original curve, which may look something like this:
//! let ellipse_curve = FunctionCurve::new(
//!     interval(0.0, TAU).unwrap(),
//!     |t| vec2(t.cos(), t.sin() * 2.0)
//! );
//!
//! // Use `map` to situate this in 3D as a Curve<Vec3>; in this case, it will be in the xy-plane:
//! let ellipse_motion_curve = ellipse_curve.map(|pos| pos.extend(0.0));
//! ```
//!
//! We might imagine further still that the interface expects the curve to have domain `[0, 1]`. The
//! `reparametrize` methods can address this:
//! ```rust
//! # use bevy_math::{vec2, prelude::*};
//! # use std::f32::consts::TAU;
//! # let ellipse_curve = FunctionCurve::new(interval(0.0, TAU).unwrap(), |t| vec2(t.cos(), t.sin() * 2.0));
//! # let ellipse_motion_curve = ellipse_curve.map(|pos| pos.extend(0.0));
//! // Change the domain to `[0, 1]` instead of `[0, TAU]`:
//! let final_curve = ellipse_motion_curve.reparametrize_linear(Interval::UNIT).unwrap();
//! ```
//!
//! Of course, there are many other ways of using these methods. In general, `map` is used for transforming
//! the output and using it to drive something else, while `reparametrize` preserves the curve's shape but
//! changes the speed and direction in which it is traversed. For instance:
//! ```rust
//! # use bevy_math::{vec2, prelude::*};
//! // A line segment curve connecting two points in the plane:
//! let start = vec2(-1.0, 1.0);
//! let end = vec2(1.0, 1.0);
//! let segment = FunctionCurve::new(Interval::UNIT, |t| start.lerp(end, t));
//!
//! // Let's make a curve that goes back and forth along this line segment forever.
//! //
//! // Start by stretching the line segment in parameter space so that it travels along its length
//! // from `-1` to `1` instead of `0` to `1`:
//! let stretched_segment = segment.reparametrize_linear(interval(-1.0, 1.0).unwrap()).unwrap();
//!
//! // Now, the *output* of `f32::sin` in `[-1, 1]` corresponds to the *input* interval of
//! // `stretched_segment`; the sinusoid output is mapped to the input parameter and controls how
//! // far along the segment we are:
//! let back_and_forth_curve = stretched_segment.reparametrize(Interval::EVERYWHERE, f32::sin);
//! ```
//!
//! ## Combining curves
//!
//! Curves become more expressive when used together. For example, maybe you want to combine two
//! curves end-to-end:
//! ```rust
//! # use bevy_math::{vec2, prelude::*};
//! # use std::f32::consts::PI;
//! // A line segment connecting `(-1, 0)` to `(0, 0)`:
//! let line_curve = FunctionCurve::new(
//!     Interval::UNIT,
//!     |t| vec2(-1.0, 0.0).lerp(vec2(0.0, 0.0), t)
//! );
//!
//! // A half-circle curve starting at `(0, 0)`:
//! let half_circle_curve = FunctionCurve::new(
//!     interval(0.0, PI).unwrap(),
//!     |t| vec2(t.cos() * -1.0 + 1.0, t.sin())
//! );
//!
//! // A curve that traverses `line_curve` and then `half_circle_curve` over the interval
//! // from `0` to `PI + 1`:
//! let combined_curve = line_curve.chain(half_circle_curve).unwrap();
//! ```
//!
//! Or, instead, maybe you want to combine two curves the *other* way, producing a single curve
//! that combines their output in a tuple:
//! ```rust
//! # use bevy_math::{vec2, prelude::*};
//! // Some entity's position in 2D:
//! let position_curve = FunctionCurve::new(Interval::UNIT, |t| vec2(t.cos(), t.sin()));
//!
//! // The same entity's orientation, described as a rotation. (In this case it will be spinning.)
//! let orientation_curve = FunctionCurve::new(Interval::UNIT, |t| Rot2::radians(5.0 * t));
//!
//! // Both in one curve with `(Vec2, Rot2)` output:
//! let position_and_orientation = position_curve.zip(orientation_curve).unwrap();
//! ```
//!
//! See the documentation on [`chain`] and [`zip`] for more details on how these methods work.
//!
//! ## <a name="Resampling-and-rasterization"></a>Resampling and rasterization
//!
//! Sometimes, for reasons of portability, performance, or otherwise, it can be useful to ensure that
//! curves of various provenance all actually share the same concrete type. This is the purpose of the
//! [`resample`] family of functions: they allow a curve to be replaced by an approximate version of
//! itself defined by interpolation over samples from the original curve.
//!
//! In effect, this allows very different curves to be rasterized and treated uniformly. For example:
//! ```rust
//! # use bevy_math::{vec2, prelude::*};
//! // A curve that is not easily transported because it relies on evaluating a function:
//! let interesting_curve = FunctionCurve::new(Interval::UNIT, |t| vec2(t * 3.0, t.exp()));
//!
//! // A rasterized form of the preceding curve which is just a `SampleAutoCurve`. Inside, this
//! // just stores an `Interval` along with a buffer of sample data, so it's easy to serialize
//! // and deserialize:
//! let resampled_curve = interesting_curve.resample_auto(100).unwrap();
//!
//! // The rasterized form can be seamlessly used as a curve itself:
//! let some_value = resampled_curve.sample(0.5).unwrap();
//! ```
//!
//! ## <a name="Ownership-and-borrowing"></a>Ownership and borrowing
//!
//! It can be easy to get tripped up by how curves specifically interact with Rust's ownership semantics.
//! First of all, it's worth noting that the API never uses `&mut self` — every method either takes
//! ownership of the original curve or uses a shared reference.
//!
//! Because of the methods that take ownership, it is useful to be aware of the following:
//! - If `curve` is a curve, then `&curve` is also a curve with the same output. For convenience,
//!   `&curve` can be written as `curve.by_ref()` for use in method chaining.
//! - However, `&curve` cannot outlive `curve`. In general, it is not `'static`.
//!
//! In other words, `&curve` can be used to perform temporary operations without consuming `curve` (for
//! example, to effectively pass `curve` into an API which expects an `impl Curve<T>`), but it *cannot*
//! be used in situations where persistence is necessary (e.g. when the curve itself must be stored
//! for later use).
//!
//! Here is a demonstration:
//! ```rust
//! # use bevy_math::prelude::*;
//! # let some_magic_constructor = || EasingCurve::new(0.0, 1.0, EaseFunction::ElasticInOut).graph();
//! //`my_curve` is obtained somehow. It is a `Curve<(f32, f32)>`.
//! let my_curve = some_magic_constructor();
//!
//! // Now, we want to sample a mapped version of `my_curve`.
//!
//! // let samples: Vec<f32> = my_curve.map(|(x, y)| y).samples(50).unwrap().collect();
//! // ^ This would work, but it would also invalidate `my_curve`, since `map` takes ownership.
//!
//! // Instead, we pass a borrowed version of `my_curve` to `map`. It lives long enough that we
//! // can extract samples:
//! let samples: Vec<f32> = my_curve.by_ref().map(|(x, y)| y).samples(50).unwrap().collect();
//!
//! // This way, we retain the ability to use `my_curve` later:
//! let new_curve = my_curve.map(|(x,y)| x + y);
//! ```
//!
//! [domain]: Curve::domain
//! [sampled]: Curve::sample
//! [changing parametrizations]: CurveExt::reparametrize
//! [mapping output]: CurveExt::map
//! [rasterization]: CurveResampleExt::resample
//! [functions]: FunctionCurve
//! [sample interpolation]: SampleCurve
//! [splines]: crate::cubic_splines
//! [easings]: easing
//! [spline curves]: crate::cubic_splines
//! [easing curves]: easing
//! [`chain`]: CurveExt::chain
//! [`zip`]: CurveExt::zip
//! [`resample`]: CurveResampleExt::resample
//!
//! [^footnote]: In fact, universal as well, in some sense: if `curve` is any curve, then `FunctionCurve::new
//! (curve.domain(), |t| curve.sample_unchecked(t))` is an equivalent function curve.

pub mod adaptors;
pub mod cores;
pub mod derivatives;
pub mod easing;
pub mod interval;
pub mod iterable;

#[cfg(feature = "alloc")]
pub mod sample_curves;

// bevy_math::curve re-exports all commonly-needed curve-related items.
pub use adaptors::*;
pub use easing::*;
pub use interval::{interval, Interval};

#[cfg(feature = "alloc")]
pub use {
    cores::{EvenCore, UnevenCore},
    sample_curves::*,
};

use crate::VectorSpace;
use core::{marker::PhantomData, ops::Deref};
use interval::InvalidIntervalError;
use thiserror::Error;

#[cfg(feature = "alloc")]
use {crate::StableInterpolate, itertools::Itertools};

/// A trait for a type that can represent values of type `T` parametrized over a fixed interval.
///
/// Typical examples of this are actual geometric curves where `T: VectorSpace`, but other kinds
/// of output data can be represented as well. See the [module-level documentation] for details.
///
/// [module-level documentation]: self
pub trait Curve<T> {
    /// The interval over which this curve is parametrized.
    ///
    /// This is the range of values of `t` where we can sample the curve and receive valid output.
    fn domain(&self) -> Interval;

    /// Sample a point on this curve at the parameter value `t`, extracting the associated value.
    /// This is the unchecked version of sampling, which should only be used if the sample time `t`
    /// is already known to lie within the curve's domain.
    ///
    /// Values sampled from outside of a curve's domain are generally considered invalid; data which
    /// is nonsensical or otherwise useless may be returned in such a circumstance, and extrapolation
    /// beyond a curve's domain should not be relied upon.
    fn sample_unchecked(&self, t: f32) -> T;

    /// Sample a point on this curve at the parameter value `t`, returning `None` if the point is
    /// outside of the curve's domain.
    fn sample(&self, t: f32) -> Option<T> {
        match self.domain().contains(t) {
            true => Some(self.sample_unchecked(t)),
            false => None,
        }
    }

    /// Sample a point on this curve at the parameter value `t`, clamping `t` to lie inside the
    /// domain of the curve.
    fn sample_clamped(&self, t: f32) -> T {
        let t = self.domain().clamp(t);
        self.sample_unchecked(t)
    }
}

impl<T, C, D> Curve<T> for D
where
    C: Curve<T> + ?Sized,
    D: Deref<Target = C>,
{
    fn domain(&self) -> Interval {
        <C as Curve<T>>::domain(self)
    }

    fn sample_unchecked(&self, t: f32) -> T {
        <C as Curve<T>>::sample_unchecked(self, t)
    }
}

/// Extension trait implemented by [curves], allowing access to a number of adaptors and
/// convenience methods.
///
/// This trait is automatically implemented for all curves that are `Sized`. In particular,
/// it is implemented for types like `Box<dyn Curve<T>>`. `CurveExt` is not dyn-compatible
/// itself.
///
/// For more information, see the [module-level documentation].
///
/// [curves]: Curve
/// [module-level documentation]: self
pub trait CurveExt<T>: Curve<T> + Sized {
    /// Sample a collection of `n >= 0` points on this curve at the parameter values `t_n`,
    /// returning `None` if the point is outside of the curve's domain.
    ///
    /// The samples are returned in the same order as the parameter values `t_n` were provided and
    /// will include all results. This leaves the responsibility for things like filtering and
    /// sorting to the user for maximum flexibility.
    fn sample_iter(&self, iter: impl IntoIterator<Item = f32>) -> impl Iterator<Item = Option<T>> {
        iter.into_iter().map(|t| self.sample(t))
    }

    /// Sample a collection of `n >= 0` points on this curve at the parameter values `t_n`,
    /// extracting the associated values. This is the unchecked version of sampling, which should
    /// only be used if the sample times `t_n` are already known to lie within the curve's domain.
    ///
    /// Values sampled from outside of a curve's domain are generally considered invalid; data
    /// which is nonsensical or otherwise useless may be returned in such a circumstance, and
    /// extrapolation beyond a curve's domain should not be relied upon.
    ///
    /// The samples are returned in the same order as the parameter values `t_n` were provided and
    /// will include all results. This leaves the responsibility for things like filtering and
    /// sorting to the user for maximum flexibility.
    fn sample_iter_unchecked(
        &self,
        iter: impl IntoIterator<Item = f32>,
    ) -> impl Iterator<Item = T> {
        iter.into_iter().map(|t| self.sample_unchecked(t))
    }

    /// Sample a collection of `n >= 0` points on this curve at the parameter values `t_n`,
    /// clamping `t_n` to lie inside the domain of the curve.
    ///
    /// The samples are returned in the same order as the parameter values `t_n` were provided and
    /// will include all results. This leaves the responsibility for things like filtering and
    /// sorting to the user for maximum flexibility.
    fn sample_iter_clamped(&self, iter: impl IntoIterator<Item = f32>) -> impl Iterator<Item = T> {
        iter.into_iter().map(|t| self.sample_clamped(t))
    }

    /// Create a new curve by mapping the values of this curve via a function `f`; i.e., if the
    /// sample at time `t` for this curve is `x`, the value at time `t` on the new curve will be
    /// `f(x)`.
    #[must_use]
    fn map<S, F>(self, f: F) -> MapCurve<T, S, Self, F>
    where
        F: Fn(T) -> S,
    {
        MapCurve {
            preimage: self,
            f,
            _phantom: PhantomData,
        }
    }

    /// Create a new [`Curve`] whose parameter space is related to the parameter space of this curve
    /// by `f`. For each time `t`, the sample from the new curve at time `t` is the sample from
    /// this curve at time `f(t)`. The given `domain` will be the domain of the new curve. The
    /// function `f` is expected to take `domain` into `self.domain()`.
    ///
    /// Note that this is the opposite of what one might expect intuitively; for example, if this
    /// curve has a parameter domain of `[0, 1]`, then stretching the parameter domain to
    /// `[0, 2]` would be performed as follows, dividing by what might be perceived as the scaling
    /// factor rather than multiplying:
    /// ```
    /// # use bevy_math::curve::*;
    /// let my_curve = ConstantCurve::new(Interval::UNIT, 1.0);
    /// let scaled_curve = my_curve.reparametrize(interval(0.0, 2.0).unwrap(), |t| t / 2.0);
    /// ```
    /// This kind of linear remapping is provided by the convenience method
    /// [`CurveExt::reparametrize_linear`], which requires only the desired domain for the new curve.
    ///
    /// # Examples
    /// ```
    /// // Reverse a curve:
    /// # use bevy_math::curve::*;
    /// # use bevy_math::vec2;
    /// let my_curve = ConstantCurve::new(Interval::UNIT, 1.0);
    /// let domain = my_curve.domain();
    /// let reversed_curve = my_curve.reparametrize(domain, |t| domain.end() - (t - domain.start()));
    ///
    /// // Take a segment of a curve:
    /// # let my_curve = ConstantCurve::new(Interval::UNIT, 1.0);
    /// let curve_segment = my_curve.reparametrize(interval(0.0, 0.5).unwrap(), |t| 0.5 + t);
    /// ```
    #[must_use]
    fn reparametrize<F>(self, domain: Interval, f: F) -> ReparamCurve<T, Self, F>
    where
        F: Fn(f32) -> f32,
    {
        ReparamCurve {
            domain,
            base: self,
            f,
            _phantom: PhantomData,
        }
    }

    /// Linearly reparametrize this [`Curve`], producing a new curve whose domain is the given
    /// `domain` instead of the current one. This operation is only valid for curves with bounded
    /// domains.
    ///
    /// # Errors
    ///
    /// If either this curve's domain or the given `domain` is unbounded, an error is returned.
    fn reparametrize_linear(
        self,
        domain: Interval,
    ) -> Result<LinearReparamCurve<T, Self>, LinearReparamError> {
        if !self.domain().is_bounded() {
            return Err(LinearReparamError::SourceCurveUnbounded);
        }

        if !domain.is_bounded() {
            return Err(LinearReparamError::TargetIntervalUnbounded);
        }

        Ok(LinearReparamCurve {
            base: self,
            new_domain: domain,
            _phantom: PhantomData,
        })
    }

    /// Reparametrize this [`Curve`] by sampling from another curve.
    ///
    /// The resulting curve samples at time `t` by first sampling `other` at time `t`, which produces
    /// another sample time `s` which is then used to sample this curve. The domain of the resulting
    /// curve is the domain of `other`.
    #[must_use]
    fn reparametrize_by_curve<C>(self, other: C) -> CurveReparamCurve<T, Self, C>
    where
        C: Curve<f32>,
    {
        CurveReparamCurve {
            base: self,
            reparam_curve: other,
            _phantom: PhantomData,
        }
    }

    /// Create a new [`Curve`] which is the graph of this one; that is, its output echoes the sample
    /// time as part of a tuple.
    ///
    /// For example, if this curve outputs `x` at time `t`, then the produced curve will produce
    /// `(t, x)` at time `t`. In particular, if this curve is a `Curve<T>`, the output of this method
    /// is a `Curve<(f32, T)>`.
    #[must_use]
    fn graph(self) -> GraphCurve<T, Self> {
        GraphCurve {
            base: self,
            _phantom: PhantomData,
        }
    }

    /// Create a new [`Curve`] by zipping this curve together with another.
    ///
    /// The sample at time `t` in the new curve is `(x, y)`, where `x` is the sample of `self` at
    /// time `t` and `y` is the sample of `other` at time `t`. The domain of the new curve is the
    /// intersection of the domains of its constituents.
    ///
    /// # Errors
    ///
    /// If the domain intersection would be empty, an error is returned instead.
    fn zip<S, C>(self, other: C) -> Result<ZipCurve<T, S, Self, C>, InvalidIntervalError>
    where
        C: Curve<S> + Sized,
    {
        let domain = self.domain().intersect(other.domain())?;
        Ok(ZipCurve {
            domain,
            first: self,
            second: other,
            _phantom: PhantomData,
        })
    }

    /// Create a new [`Curve`] by composing this curve end-to-start with another, producing another curve
    /// with outputs of the same type. The domain of the other curve is translated so that its start
    /// coincides with where this curve ends.
    ///
    /// # Errors
    ///
    /// A [`ChainError`] is returned if this curve's domain doesn't have a finite end or if
    /// `other`'s domain doesn't have a finite start.
    fn chain<C>(self, other: C) -> Result<ChainCurve<T, Self, C>, ChainError>
    where
        C: Curve<T>,
    {
        if !self.domain().has_finite_end() {
            return Err(ChainError::FirstEndInfinite);
        }
        if !other.domain().has_finite_start() {
            return Err(ChainError::SecondStartInfinite);
        }
        Ok(ChainCurve {
            first: self,
            second: other,
            _phantom: PhantomData,
        })
    }

    /// Create a new [`Curve`] inverting this curve on the x-axis, producing another curve with
    /// outputs of the same type, effectively playing backwards starting at `self.domain().end()`
    /// and transitioning over to `self.domain().start()`. The domain of the new curve is still the
    /// same.
    ///
    /// # Errors
    ///
    /// A [`ReverseError`] is returned if this curve's domain isn't bounded.
    fn reverse(self) -> Result<ReverseCurve<T, Self>, ReverseError> {
        self.domain()
            .is_bounded()
            .then(|| ReverseCurve {
                curve: self,
                _phantom: PhantomData,
            })
            .ok_or(ReverseError::SourceDomainEndInfinite)
    }

    /// Create a new [`Curve`] repeating this curve `N` times, producing another curve with outputs
    /// of the same type. The domain of the new curve will be bigger by a factor of `n + 1`.
    ///
    /// # Notes
    ///
    /// - this doesn't guarantee a smooth transition from one occurrence of the curve to its next
    ///   iteration. The curve will make a jump if `self.domain().start() != self.domain().end()`!
    /// - for `count == 0` the output of this adaptor is basically identical to the previous curve
    /// - the value at the transitioning points (`domain.end() * n` for `n >= 1`) in the results is the
    ///   value at `domain.end()` in the original curve
    ///
    /// # Errors
    ///
    /// A [`RepeatError`] is returned if this curve's domain isn't bounded.
    fn repeat(self, count: usize) -> Result<RepeatCurve<T, Self>, RepeatError> {
        self.domain()
            .is_bounded()
            .then(|| {
                // This unwrap always succeeds because `curve` has a valid Interval as its domain and the
                // length of `curve` cannot be NAN. It's still fine if it's infinity.
                let domain = Interval::new(
                    self.domain().start(),
                    self.domain().end() + self.domain().length() * count as f32,
                )
                .unwrap();
                RepeatCurve {
                    domain,
                    curve: self,
                    _phantom: PhantomData,
                }
            })
            .ok_or(RepeatError::SourceDomainUnbounded)
    }

    /// Create a new [`Curve`] repeating this curve forever, producing another curve with
    /// outputs of the same type. The domain of the new curve will be unbounded.
    ///
    /// # Notes
    ///
    /// - this doesn't guarantee a smooth transition from one occurrence of the curve to its next
    ///   iteration. The curve will make a jump if `self.domain().start() != self.domain().end()`!
    /// - the value at the transitioning points (`domain.end() * n` for `n >= 1`) in the results is the
    ///   value at `domain.end()` in the original curve
    ///
    /// # Errors
    ///
    /// A [`RepeatError`] is returned if this curve's domain isn't bounded.
    fn forever(self) -> Result<ForeverCurve<T, Self>, RepeatError> {
        self.domain()
            .is_bounded()
            .then(|| ForeverCurve {
                curve: self,
                _phantom: PhantomData,
            })
            .ok_or(RepeatError::SourceDomainUnbounded)
    }

    /// Create a new [`Curve`] chaining the original curve with its inverse, producing
    /// another curve with outputs of the same type. The domain of the new curve will be twice as
    /// long. The transition point is guaranteed to not make any jumps.
    ///
    /// # Errors
    ///
    /// A [`PingPongError`] is returned if this curve's domain isn't right-finite.
    fn ping_pong(self) -> Result<PingPongCurve<T, Self>, PingPongError> {
        self.domain()
            .has_finite_end()
            .then(|| PingPongCurve {
                curve: self,
                _phantom: PhantomData,
            })
            .ok_or(PingPongError::SourceDomainEndInfinite)
    }

    /// Create a new [`Curve`] by composing this curve end-to-start with another, producing another
    /// curve with outputs of the same type. The domain of the other curve is translated so that
    /// its start coincides with where this curve ends.
    ///
    ///
    /// Additionally the transition of the samples is guaranteed to make no sudden jumps. This is
    /// useful if you really just know about the shapes of your curves and don't want to deal with
    /// stitching them together properly when it would just introduce useless complexity. It is
    /// realized by translating the other curve so that its start sample point coincides with the
    /// current curves' end sample point.
    ///
    /// # Errors
    ///
    /// A [`ChainError`] is returned if this curve's domain doesn't have a finite end or if
    /// `other`'s domain doesn't have a finite start.
    fn chain_continue<C>(self, other: C) -> Result<ContinuationCurve<T, Self, C>, ChainError>
    where
        T: VectorSpace,
        C: Curve<T>,
    {
        if !self.domain().has_finite_end() {
            return Err(ChainError::FirstEndInfinite);
        }
        if !other.domain().has_finite_start() {
            return Err(ChainError::SecondStartInfinite);
        }

        let offset = self.sample_unchecked(self.domain().end())
            - other.sample_unchecked(self.domain().start());

        Ok(ContinuationCurve {
            first: self,
            second: other,
            offset,
            _phantom: PhantomData,
        })
    }

    /// Extract an iterator over evenly-spaced samples from this curve.
    ///
    /// # Errors
    ///
    /// If `samples` is less than 2 or if this curve has unbounded domain, a [`ResamplingError`]
    /// is returned.
    fn samples(&self, samples: usize) -> Result<impl Iterator<Item = T>, ResamplingError> {
        if samples < 2 {
            return Err(ResamplingError::NotEnoughSamples(samples));
        }
        if !self.domain().is_bounded() {
            return Err(ResamplingError::UnboundedDomain);
        }

        // Unwrap on `spaced_points` always succeeds because its error conditions are handled
        // above.
        Ok(self
            .domain()
            .spaced_points(samples)
            .unwrap()
            .map(|t| self.sample_unchecked(t)))
    }

    /// Borrow this curve rather than taking ownership of it. This is essentially an alias for a
    /// prefix `&`; the point is that intermediate operations can be performed while retaining
    /// access to the original curve.
    ///
    /// # Example
    /// ```
    /// # use bevy_math::curve::*;
    /// let my_curve = FunctionCurve::new(Interval::UNIT, |t| t * t + 1.0);
    ///
    /// // Borrow `my_curve` long enough to resample a mapped version. Note that `map` takes
    /// // ownership of its input.
    /// let samples = my_curve.by_ref().map(|x| x * 2.0).resample_auto(100).unwrap();
    ///
    /// // Do something else with `my_curve` since we retained ownership:
    /// let new_curve = my_curve.reparametrize_linear(interval(-1.0, 1.0).unwrap()).unwrap();
    /// ```
    fn by_ref(&self) -> &Self {
        self
    }

    /// Flip this curve so that its tuple output is arranged the other way.
    #[must_use]
    fn flip<U, V>(self) -> impl Curve<(V, U)>
    where
        Self: CurveExt<(U, V)>,
    {
        self.map(|(u, v)| (v, u))
    }
}

impl<C, T> CurveExt<T> for C where C: Curve<T> {}

/// Extension trait implemented by [curves], allowing access to generic resampling methods as
/// well as those based on [stable interpolation].
///
/// This trait is automatically implemented for all curves.
///
/// For more information, see the [module-level documentation].
///
/// [curves]: Curve
/// [stable interpolation]: crate::StableInterpolate
/// [module-level documentation]: self
#[cfg(feature = "alloc")]
pub trait CurveResampleExt<T>: Curve<T> {
    /// Resample this [`Curve`] to produce a new one that is defined by interpolation over equally
    /// spaced sample values, using the provided `interpolation` to interpolate between adjacent samples.
    /// The curve is interpolated on `segments` segments between samples. For example, if `segments` is 1,
    /// only the start and end points of the curve are used as samples; if `segments` is 2, a sample at
    /// the midpoint is taken as well, and so on.
    ///
    /// The interpolation takes two values by reference together with a scalar parameter and
    /// produces an owned value. The expectation is that `interpolation(&x, &y, 0.0)` and
    /// `interpolation(&x, &y, 1.0)` are equivalent to `x` and `y` respectively.
    ///
    /// # Errors
    ///
    /// If `segments` is zero or if this curve has unbounded domain, then a [`ResamplingError`] is
    /// returned.
    ///
    /// # Example
    /// ```
    /// # use bevy_math::*;
    /// # use bevy_math::curve::*;
    /// let quarter_rotation = FunctionCurve::new(interval(0.0, 90.0).unwrap(), |t| Rot2::degrees(t));
    /// // A curve which only stores three data points and uses `nlerp` to interpolate them:
    /// let resampled_rotation = quarter_rotation.resample(3, |x, y, t| x.nlerp(*y, t));
    /// ```
    fn resample<I>(
        &self,
        segments: usize,
        interpolation: I,
    ) -> Result<SampleCurve<T, I>, ResamplingError>
    where
        I: Fn(&T, &T, f32) -> T,
    {
        let samples = self.samples(segments + 1)?.collect_vec();
        Ok(SampleCurve {
            core: EvenCore {
                domain: self.domain(),
                samples,
            },
            interpolation,
        })
    }

    /// Resample this [`Curve`] to produce a new one that is defined by interpolation over equally
    /// spaced sample values, using [automatic interpolation] to interpolate between adjacent samples.
    /// The curve is interpolated on `segments` segments between samples. For example, if `segments` is 1,
    /// only the start and end points of the curve are used as samples; if `segments` is 2, a sample at
    /// the midpoint is taken as well, and so on.
    ///
    /// # Errors
    ///
    /// If `segments` is zero or if this curve has unbounded domain, a [`ResamplingError`] is returned.
    ///
    /// [automatic interpolation]: crate::common_traits::StableInterpolate
    fn resample_auto(&self, segments: usize) -> Result<SampleAutoCurve<T>, ResamplingError>
    where
        T: StableInterpolate,
    {
        let samples = self.samples(segments + 1)?.collect_vec();
        Ok(SampleAutoCurve {
            core: EvenCore {
                domain: self.domain(),
                samples,
            },
        })
    }

    /// Resample this [`Curve`] to produce a new one that is defined by interpolation over samples
    /// taken at a given set of times. The given `interpolation` is used to interpolate adjacent
    /// samples, and the `sample_times` are expected to contain at least two valid times within the
    /// curve's domain interval.
    ///
    /// Redundant sample times, non-finite sample times, and sample times outside of the domain
    /// are filtered out. With an insufficient quantity of data, a [`ResamplingError`] is
    /// returned.
    ///
    /// The domain of the produced curve stretches between the first and last sample times of the
    /// iterator.
    ///
    /// The interpolation takes two values by reference together with a scalar parameter and
    /// produces an owned value. The expectation is that `interpolation(&x, &y, 0.0)` and
    /// `interpolation(&x, &y, 1.0)` are equivalent to `x` and `y` respectively.
    ///
    /// # Errors
    ///
    /// If `sample_times` doesn't contain at least two distinct times after filtering, a
    /// [`ResamplingError`] is returned.
    fn resample_uneven<I>(
        &self,
        sample_times: impl IntoIterator<Item = f32>,
        interpolation: I,
    ) -> Result<UnevenSampleCurve<T, I>, ResamplingError>
    where
        I: Fn(&T, &T, f32) -> T,
    {
        let domain = self.domain();
        let mut times = sample_times
            .into_iter()
            .filter(|t| t.is_finite() && domain.contains(*t))
            .collect_vec();
        times.sort_by(f32::total_cmp);
        times.dedup();
        if times.len() < 2 {
            return Err(ResamplingError::NotEnoughSamples(times.len()));
        }
        let samples = times.iter().map(|t| self.sample_unchecked(*t)).collect();
        Ok(UnevenSampleCurve {
            core: UnevenCore { times, samples },
            interpolation,
        })
    }

    /// Resample this [`Curve`] to produce a new one that is defined by [automatic interpolation] over
    /// samples taken at the given set of times. The given `sample_times` are expected to contain at least
    /// two valid times within the curve's domain interval.
    ///
    /// Redundant sample times, non-finite sample times, and sample times outside of the domain
    /// are simply filtered out. With an insufficient quantity of data, a [`ResamplingError`] is
    /// returned.
    ///
    /// The domain of the produced [`UnevenSampleAutoCurve`] stretches between the first and last
    /// sample times of the iterator.
    ///
    /// # Errors
    ///
    /// If `sample_times` doesn't contain at least two distinct times after filtering, a
    /// [`ResamplingError`] is returned.
    ///
    /// [automatic interpolation]: crate::common_traits::StableInterpolate
    fn resample_uneven_auto(
        &self,
        sample_times: impl IntoIterator<Item = f32>,
    ) -> Result<UnevenSampleAutoCurve<T>, ResamplingError>
    where
        T: StableInterpolate,
    {
        let domain = self.domain();
        let mut times = sample_times
            .into_iter()
            .filter(|t| t.is_finite() && domain.contains(*t))
            .collect_vec();
        times.sort_by(f32::total_cmp);
        times.dedup();
        if times.len() < 2 {
            return Err(ResamplingError::NotEnoughSamples(times.len()));
        }
        let samples = times.iter().map(|t| self.sample_unchecked(*t)).collect();
        Ok(UnevenSampleAutoCurve {
            core: UnevenCore { times, samples },
        })
    }
}

#[cfg(feature = "alloc")]
impl<C, T> CurveResampleExt<T> for C where C: Curve<T> + ?Sized {}

/// An error indicating that a linear reparameterization couldn't be performed because of
/// malformed inputs.
#[derive(Debug, Error)]
#[error("Could not build a linear function to reparametrize this curve")]
pub enum LinearReparamError {
    /// The source curve that was to be reparametrized had unbounded domain.
    #[error("This curve has unbounded domain")]
    SourceCurveUnbounded,

    /// The target interval for reparameterization was unbounded.
    #[error("The target interval for reparameterization is unbounded")]
    TargetIntervalUnbounded,
}

/// An error indicating that a reversion of a curve couldn't be performed because of
/// malformed inputs.
#[derive(Debug, Error)]
#[error("Could not reverse this curve")]
pub enum ReverseError {
    /// The source curve that was to be reversed had unbounded domain end.
    #[error("This curve has an unbounded domain end")]
    SourceDomainEndInfinite,
}

/// An error indicating that a repetition of a curve couldn't be performed because of malformed
/// inputs.
#[derive(Debug, Error)]
#[error("Could not repeat this curve")]
pub enum RepeatError {
    /// The source curve that was to be repeated had unbounded domain.
    #[error("This curve has an unbounded domain")]
    SourceDomainUnbounded,
}

/// An error indicating that a ping ponging of a curve couldn't be performed because of
/// malformed inputs.
#[derive(Debug, Error)]
#[error("Could not ping pong this curve")]
pub enum PingPongError {
    /// The source curve that was to be ping ponged had unbounded domain end.
    #[error("This curve has an unbounded domain end")]
    SourceDomainEndInfinite,
}

/// An error indicating that an end-to-end composition couldn't be performed because of
/// malformed inputs.
#[derive(Debug, Error)]
#[error("Could not compose these curves together")]
pub enum ChainError {
    /// The right endpoint of the first curve was infinite.
    #[error("The first curve's domain has an infinite end")]
    FirstEndInfinite,

    /// The left endpoint of the second curve was infinite.
    #[error("The second curve's domain has an infinite start")]
    SecondStartInfinite,
}

/// An error indicating that a resampling operation could not be performed because of
/// malformed inputs.
#[derive(Debug, Error)]
#[error("Could not resample from this curve because of bad inputs")]
pub enum ResamplingError {
    /// This resampling operation was not provided with enough samples to have well-formed output.
    #[error("Not enough unique samples to construct resampled curve")]
    NotEnoughSamples(usize),

    /// This resampling operation failed because of an unbounded interval.
    #[error("Could not resample because this curve has unbounded domain")]
    UnboundedDomain,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ops, Quat};
    use alloc::vec::Vec;
    use approx::{assert_abs_diff_eq, AbsDiffEq};
    use core::f32::consts::TAU;
    use glam::*;

    #[test]
    fn curve_can_be_made_into_an_object() {
        let curve = ConstantCurve::new(Interval::UNIT, 42.0);
        let curve: &dyn Curve<f64> = &curve;

        assert_eq!(curve.sample(1.0), Some(42.0));
        assert_eq!(curve.sample(2.0), None);
    }

    #[test]
    fn constant_curves() {
        let curve = ConstantCurve::new(Interval::EVERYWHERE, 5.0);
        assert!(curve.sample_unchecked(-35.0) == 5.0);

        let curve = ConstantCurve::new(Interval::UNIT, true);
        assert!(curve.sample_unchecked(2.0));
        assert!(curve.sample(2.0).is_none());
    }

    #[test]
    fn function_curves() {
        let curve = FunctionCurve::new(Interval::EVERYWHERE, |t| t * t);
        assert!(curve.sample_unchecked(2.0).abs_diff_eq(&4.0, f32::EPSILON));
        assert!(curve.sample_unchecked(-3.0).abs_diff_eq(&9.0, f32::EPSILON));

        let curve = FunctionCurve::new(interval(0.0, f32::INFINITY).unwrap(), ops::log2);
        assert_eq!(curve.sample_unchecked(3.5), ops::log2(3.5));
        assert!(curve.sample_unchecked(-1.0).is_nan());
        assert!(curve.sample(-1.0).is_none());
    }

    #[test]
    fn linear_curve() {
        let start = Vec2::ZERO;
        let end = Vec2::new(1.0, 2.0);
        let curve = EasingCurve::new(start, end, EaseFunction::Linear);

        let mid = (start + end) / 2.0;

        [(0.0, start), (0.5, mid), (1.0, end)]
            .into_iter()
            .for_each(|(t, x)| {
                assert!(curve.sample_unchecked(t).abs_diff_eq(x, f32::EPSILON));
            });
    }

    #[test]
    fn easing_curves_step() {
        let start = Vec2::ZERO;
        let end = Vec2::new(1.0, 2.0);

        let curve = EasingCurve::new(start, end, EaseFunction::Steps(4, JumpAt::End));
        [
            (0.0, start),
            (0.249, start),
            (0.250, Vec2::new(0.25, 0.5)),
            (0.499, Vec2::new(0.25, 0.5)),
            (0.500, Vec2::new(0.5, 1.0)),
            (0.749, Vec2::new(0.5, 1.0)),
            (0.750, Vec2::new(0.75, 1.5)),
            (1.0, end),
        ]
        .into_iter()
        .for_each(|(t, x)| {
            assert!(curve.sample_unchecked(t).abs_diff_eq(x, f32::EPSILON));
        });
    }

    #[test]
    fn easing_curves_quadratic() {
        let start = Vec2::ZERO;
        let end = Vec2::new(1.0, 2.0);

        let curve = EasingCurve::new(start, end, EaseFunction::QuadraticIn);
        [
            (0.0, start),
            (0.25, Vec2::new(0.0625, 0.125)),
            (0.5, Vec2::new(0.25, 0.5)),
            (1.0, end),
        ]
        .into_iter()
        .for_each(|(t, x)| {
            assert!(curve.sample_unchecked(t).abs_diff_eq(x, f32::EPSILON),);
        });
    }

    #[expect(
        clippy::neg_multiply,
        reason = "Clippy doesn't like this, but it's correct"
    )]
    #[test]
    fn mapping() {
        let curve = FunctionCurve::new(Interval::EVERYWHERE, |t| t * 3.0 + 1.0);
        let mapped_curve = curve.map(|x| x / 7.0);
        assert_eq!(mapped_curve.sample_unchecked(3.5), (3.5 * 3.0 + 1.0) / 7.0);
        assert_eq!(
            mapped_curve.sample_unchecked(-1.0),
            (-1.0 * 3.0 + 1.0) / 7.0
        );
        assert_eq!(mapped_curve.domain(), Interval::EVERYWHERE);

        let curve = FunctionCurve::new(Interval::UNIT, |t| t * TAU);
        let mapped_curve = curve.map(Quat::from_rotation_z);
        assert_eq!(mapped_curve.sample_unchecked(0.0), Quat::IDENTITY);
        assert!(mapped_curve.sample_unchecked(1.0).is_near_identity());
        assert_eq!(mapped_curve.domain(), Interval::UNIT);
    }

    #[test]
    fn reverse() {
        let curve = FunctionCurve::new(Interval::new(0.0, 1.0).unwrap(), |t| t * 3.0 + 1.0);
        let rev_curve = curve.reverse().unwrap();
        assert_eq!(rev_curve.sample(-0.1), None);
        assert_eq!(rev_curve.sample(0.0), Some(1.0 * 3.0 + 1.0));
        assert_eq!(rev_curve.sample(0.5), Some(0.5 * 3.0 + 1.0));
        assert_eq!(rev_curve.sample(1.0), Some(0.0 * 3.0 + 1.0));
        assert_eq!(rev_curve.sample(1.1), None);

        let curve = FunctionCurve::new(Interval::new(-2.0, 1.0).unwrap(), |t| t * 3.0 + 1.0);
        let rev_curve = curve.reverse().unwrap();
        assert_eq!(rev_curve.sample(-2.1), None);
        assert_eq!(rev_curve.sample(-2.0), Some(1.0 * 3.0 + 1.0));
        assert_eq!(rev_curve.sample(-0.5), Some(-0.5 * 3.0 + 1.0));
        assert_eq!(rev_curve.sample(1.0), Some(-2.0 * 3.0 + 1.0));
        assert_eq!(rev_curve.sample(1.1), None);
    }

    #[test]
    fn repeat() {
        let curve = FunctionCurve::new(Interval::new(0.0, 1.0).unwrap(), |t| t * 3.0 + 1.0);
        let repeat_curve = curve.by_ref().repeat(1).unwrap();
        assert_eq!(repeat_curve.sample(-0.1), None);
        assert_eq!(repeat_curve.sample(0.0), Some(0.0 * 3.0 + 1.0));
        assert_eq!(repeat_curve.sample(0.5), Some(0.5 * 3.0 + 1.0));
        assert_eq!(repeat_curve.sample(0.99), Some(0.99 * 3.0 + 1.0));
        assert_eq!(repeat_curve.sample(1.0), Some(1.0 * 3.0 + 1.0));
        assert_eq!(repeat_curve.sample(1.01), Some(0.01 * 3.0 + 1.0));
        assert_eq!(repeat_curve.sample(1.5), Some(0.5 * 3.0 + 1.0));
        assert_eq!(repeat_curve.sample(1.99), Some(0.99 * 3.0 + 1.0));
        assert_eq!(repeat_curve.sample(2.0), Some(1.0 * 3.0 + 1.0));
        assert_eq!(repeat_curve.sample(2.01), None);

        let repeat_curve = curve.by_ref().repeat(3).unwrap();
        assert_eq!(repeat_curve.sample(2.0), Some(1.0 * 3.0 + 1.0));
        assert_eq!(repeat_curve.sample(3.0), Some(1.0 * 3.0 + 1.0));
        assert_eq!(repeat_curve.sample(4.0), Some(1.0 * 3.0 + 1.0));
        assert_eq!(repeat_curve.sample(5.0), None);

        let repeat_curve = curve.by_ref().forever().unwrap();
        assert_eq!(repeat_curve.sample(-1.0), Some(1.0 * 3.0 + 1.0));
        assert_eq!(repeat_curve.sample(2.0), Some(1.0 * 3.0 + 1.0));
        assert_eq!(repeat_curve.sample(3.0), Some(1.0 * 3.0 + 1.0));
        assert_eq!(repeat_curve.sample(4.0), Some(1.0 * 3.0 + 1.0));
        assert_eq!(repeat_curve.sample(5.0), Some(1.0 * 3.0 + 1.0));
    }

    #[test]
    fn ping_pong() {
        let curve = FunctionCurve::new(Interval::new(0.0, 1.0).unwrap(), |t| t * 3.0 + 1.0);
        let ping_pong_curve = curve.ping_pong().unwrap();
        assert_eq!(ping_pong_curve.sample(-0.1), None);
        assert_eq!(ping_pong_curve.sample(0.0), Some(0.0 * 3.0 + 1.0));
        assert_eq!(ping_pong_curve.sample(0.5), Some(0.5 * 3.0 + 1.0));
        assert_eq!(ping_pong_curve.sample(1.0), Some(1.0 * 3.0 + 1.0));
        assert_eq!(ping_pong_curve.sample(1.5), Some(0.5 * 3.0 + 1.0));
        assert_eq!(ping_pong_curve.sample(2.0), Some(0.0 * 3.0 + 1.0));
        assert_eq!(ping_pong_curve.sample(2.1), None);

        let curve = FunctionCurve::new(Interval::new(-2.0, 2.0).unwrap(), |t| t * 3.0 + 1.0);
        let ping_pong_curve = curve.ping_pong().unwrap();
        assert_eq!(ping_pong_curve.sample(-2.1), None);
        assert_eq!(ping_pong_curve.sample(-2.0), Some(-2.0 * 3.0 + 1.0));
        assert_eq!(ping_pong_curve.sample(-0.5), Some(-0.5 * 3.0 + 1.0));
        assert_eq!(ping_pong_curve.sample(2.0), Some(2.0 * 3.0 + 1.0));
        assert_eq!(ping_pong_curve.sample(4.5), Some(-0.5 * 3.0 + 1.0));
        assert_eq!(ping_pong_curve.sample(6.0), Some(-2.0 * 3.0 + 1.0));
        assert_eq!(ping_pong_curve.sample(6.1), None);
    }

    #[test]
    fn continue_chain() {
        let first = FunctionCurve::new(Interval::new(0.0, 1.0).unwrap(), |t| t * 3.0 + 1.0);
        let second = FunctionCurve::new(Interval::new(0.0, 1.0).unwrap(), |t| t * t);
        let c0_chain_curve = first.chain_continue(second).unwrap();
        assert_eq!(c0_chain_curve.sample(-0.1), None);
        assert_eq!(c0_chain_curve.sample(0.0), Some(0.0 * 3.0 + 1.0));
        assert_eq!(c0_chain_curve.sample(0.5), Some(0.5 * 3.0 + 1.0));
        assert_eq!(c0_chain_curve.sample(1.0), Some(1.0 * 3.0 + 1.0));
        assert_eq!(c0_chain_curve.sample(1.5), Some(1.0 * 3.0 + 1.0 + 0.25));
        assert_eq!(c0_chain_curve.sample(2.0), Some(1.0 * 3.0 + 1.0 + 1.0));
        assert_eq!(c0_chain_curve.sample(2.1), None);
    }

    #[test]
    fn reparameterization() {
        let curve = FunctionCurve::new(interval(1.0, f32::INFINITY).unwrap(), ops::log2);
        let reparametrized_curve = curve
            .by_ref()
            .reparametrize(interval(0.0, f32::INFINITY).unwrap(), ops::exp2);
        assert_abs_diff_eq!(reparametrized_curve.sample_unchecked(3.5), 3.5);
        assert_abs_diff_eq!(reparametrized_curve.sample_unchecked(100.0), 100.0);
        assert_eq!(
            reparametrized_curve.domain(),
            interval(0.0, f32::INFINITY).unwrap()
        );

        let reparametrized_curve = curve.by_ref().reparametrize(Interval::UNIT, |t| t + 1.0);
        assert_abs_diff_eq!(reparametrized_curve.sample_unchecked(0.0), 0.0);
        assert_abs_diff_eq!(reparametrized_curve.sample_unchecked(1.0), 1.0);
        assert_eq!(reparametrized_curve.domain(), Interval::UNIT);
    }

    #[test]
    fn multiple_maps() {
        // Make sure these actually happen in the right order.
        let curve = FunctionCurve::new(Interval::UNIT, ops::exp2);
        let first_mapped = curve.map(ops::log2);
        let second_mapped = first_mapped.map(|x| x * -2.0);
        assert_abs_diff_eq!(second_mapped.sample_unchecked(0.0), 0.0);
        assert_abs_diff_eq!(second_mapped.sample_unchecked(0.5), -1.0);
        assert_abs_diff_eq!(second_mapped.sample_unchecked(1.0), -2.0);
    }

    #[test]
    fn multiple_reparams() {
        // Make sure these happen in the right order too.
        let curve = FunctionCurve::new(Interval::UNIT, ops::exp2);
        let first_reparam = curve.reparametrize(interval(1.0, 2.0).unwrap(), ops::log2);
        let second_reparam = first_reparam.reparametrize(Interval::UNIT, |t| t + 1.0);
        assert_abs_diff_eq!(second_reparam.sample_unchecked(0.0), 1.0);
        assert_abs_diff_eq!(second_reparam.sample_unchecked(0.5), 1.5);
        assert_abs_diff_eq!(second_reparam.sample_unchecked(1.0), 2.0);
    }

    #[test]
    fn resampling() {
        let curve = FunctionCurve::new(interval(1.0, 4.0).unwrap(), ops::log2);

        // Need at least one segment to sample.
        let nice_try = curve.by_ref().resample_auto(0);
        assert!(nice_try.is_err());

        // The values of a resampled curve should be very close at the sample points.
        // Because of denominators, it's not literally equal.
        // (This is a tradeoff against O(1) sampling.)
        let resampled_curve = curve.by_ref().resample_auto(100).unwrap();
        for test_pt in curve.domain().spaced_points(101).unwrap() {
            let expected = curve.sample_unchecked(test_pt);
            assert_abs_diff_eq!(
                resampled_curve.sample_unchecked(test_pt),
                expected,
                epsilon = 1e-6
            );
        }

        // Another example.
        let curve = FunctionCurve::new(interval(0.0, TAU).unwrap(), ops::cos);
        let resampled_curve = curve.by_ref().resample_auto(1000).unwrap();
        for test_pt in curve.domain().spaced_points(1001).unwrap() {
            let expected = curve.sample_unchecked(test_pt);
            assert_abs_diff_eq!(
                resampled_curve.sample_unchecked(test_pt),
                expected,
                epsilon = 1e-6
            );
        }
    }

    #[test]
    fn uneven_resampling() {
        let curve = FunctionCurve::new(interval(0.0, f32::INFINITY).unwrap(), ops::exp);

        // Need at least two points to resample.
        let nice_try = curve.by_ref().resample_uneven_auto([1.0; 1]);
        assert!(nice_try.is_err());

        // Uneven sampling should produce literal equality at the sample points.
        // (This is part of what you get in exchange for O(log(n)) sampling.)
        let sample_points = (0..100).map(|idx| idx as f32 * 0.1);
        let resampled_curve = curve.by_ref().resample_uneven_auto(sample_points).unwrap();
        for idx in 0..100 {
            let test_pt = idx as f32 * 0.1;
            let expected = curve.sample_unchecked(test_pt);
            assert_eq!(resampled_curve.sample_unchecked(test_pt), expected);
        }
        assert_abs_diff_eq!(resampled_curve.domain().start(), 0.0);
        assert_abs_diff_eq!(resampled_curve.domain().end(), 9.9, epsilon = 1e-6);

        // Another example.
        let curve = FunctionCurve::new(interval(1.0, f32::INFINITY).unwrap(), ops::log2);
        let sample_points = (0..10).map(|idx| ops::exp2(idx as f32));
        let resampled_curve = curve.by_ref().resample_uneven_auto(sample_points).unwrap();
        for idx in 0..10 {
            let test_pt = ops::exp2(idx as f32);
            let expected = curve.sample_unchecked(test_pt);
            assert_eq!(resampled_curve.sample_unchecked(test_pt), expected);
        }
        assert_abs_diff_eq!(resampled_curve.domain().start(), 1.0);
        assert_abs_diff_eq!(resampled_curve.domain().end(), 512.0);
    }

    #[test]
    fn sample_iterators() {
        let times = [-0.5, 0.0, 0.5, 1.0, 1.5];

        let curve = FunctionCurve::new(Interval::EVERYWHERE, |t| t * 3.0 + 1.0);
        let samples = curve.sample_iter_unchecked(times).collect::<Vec<_>>();
        let [y0, y1, y2, y3, y4] = samples.try_into().unwrap();

        assert_eq!(y0, -0.5 * 3.0 + 1.0);
        assert_eq!(y1, 0.0 * 3.0 + 1.0);
        assert_eq!(y2, 0.5 * 3.0 + 1.0);
        assert_eq!(y3, 1.0 * 3.0 + 1.0);
        assert_eq!(y4, 1.5 * 3.0 + 1.0);

        let finite_curve = FunctionCurve::new(Interval::new(0.0, 1.0).unwrap(), |t| t * 3.0 + 1.0);
        let samples = finite_curve.sample_iter(times).collect::<Vec<_>>();
        let [y0, y1, y2, y3, y4] = samples.try_into().unwrap();

        assert_eq!(y0, None);
        assert_eq!(y1, Some(0.0 * 3.0 + 1.0));
        assert_eq!(y2, Some(0.5 * 3.0 + 1.0));
        assert_eq!(y3, Some(1.0 * 3.0 + 1.0));
        assert_eq!(y4, None);

        let samples = finite_curve.sample_iter_clamped(times).collect::<Vec<_>>();
        let [y0, y1, y2, y3, y4] = samples.try_into().unwrap();

        assert_eq!(y0, 0.0 * 3.0 + 1.0);
        assert_eq!(y1, 0.0 * 3.0 + 1.0);
        assert_eq!(y2, 0.5 * 3.0 + 1.0);
        assert_eq!(y3, 1.0 * 3.0 + 1.0);
        assert_eq!(y4, 1.0 * 3.0 + 1.0);
    }
}
