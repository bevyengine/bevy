//! A module containing utility helper structs to transform a [`Curve`] into another. This is useful
//! for building up complex curves from simple segments.
use core::marker::PhantomData;

use crate::VectorSpace;

use super::{Curve, Interval};

/// The curve that results from chaining one curve with another. The second curve is
/// effectively reparametrized so that its start is at the end of the first.
///
/// Curves of this type are produced by [`Curve::chain`].
///
/// # Domain
///
/// The first curve's domain must be right-finite and the second's must be left-finite to get a
/// valid [`ChainCurve`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct ChainCurve<T, C, D> {
    pub(super) first: C,
    pub(super) second: D,
    pub(super) _phantom: PhantomData<T>,
}

impl<T, C, D> Curve<T> for ChainCurve<T, C, D>
where
    C: Curve<T>,
    D: Curve<T>,
{
    #[inline]
    fn domain(&self) -> Interval {
        // This unwrap always succeeds because `first` has a valid Interval as its domain and the
        // length of `second` cannot be NAN. It's still fine if it's infinity.
        Interval::new(
            self.first.domain().start(),
            self.first.domain().end() + self.second.domain().length(),
        )
        .unwrap()
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        if t > self.first.domain().end() {
            self.second.sample_unchecked(
                // `t - first.domain.end` computes the offset into the domain of the second.
                t - self.first.domain().end() + self.second.domain().start(),
            )
        } else {
            self.first.sample_unchecked(t)
        }
    }
}

/// The curve that results from reversing another.
///
/// Curves of this type are produced by [`Curve::reverse`].
///
/// # Domain
///
/// The original curve's domain must be bounded to get a valid [`ReverseCurve`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct ReverseCurve<T, C> {
    pub(super) curve: C,
    pub(super) _phantom: PhantomData<T>,
}

impl<T, C> Curve<T> for ReverseCurve<T, C>
where
    C: Curve<T>,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.curve.domain()
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        self.curve
            .sample_unchecked(self.domain().end() - (t - self.domain().start()))
    }
}

/// The curve that results from repeating a curve `N` times.
///
/// # Notes
///
/// - the value at the transitioning points (`domain.end() * n` for `n >= 1`) in the results is the
///   value at `domain.end()` in the original curve
///
/// Curves of this type are produced by [`Curve::repeat`].
///
/// # Domain
///
/// The original curve's domain must be bounded to get a valid [`RepeatCurve`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct RepeatCurve<T, C> {
    pub(super) domain: Interval,
    pub(super) curve: C,
    pub(super) _phantom: PhantomData<T>,
}

impl<T, C> Curve<T> for RepeatCurve<T, C>
where
    C: Curve<T>,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.domain
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        // the domain is bounded by construction
        let d = self.curve.domain();
        let cyclic_t = (t - d.start()).rem_euclid(d.length());
        let t = if t != d.start() && cyclic_t == 0.0 {
            d.end()
        } else {
            d.start() + cyclic_t
        };
        self.curve.sample_unchecked(t)
    }
}

/// The curve that results from repeating a curve forever.
///
/// # Notes
///
/// - the value at the transitioning points (`domain.end() * n` for `n >= 1`) in the results is the
///   value at `domain.end()` in the original curve
///
/// Curves of this type are produced by [`Curve::forever`].
///
/// # Domain
///
/// The original curve's domain must be bounded to get a valid [`ForeverCurve`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct ForeverCurve<T, C> {
    pub(super) curve: C,
    pub(super) _phantom: PhantomData<T>,
}

impl<T, C> Curve<T> for ForeverCurve<T, C>
where
    C: Curve<T>,
{
    #[inline]
    fn domain(&self) -> Interval {
        Interval::EVERYWHERE
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        // the domain is bounded by construction
        let d = self.curve.domain();
        let cyclic_t = (t - d.start()).rem_euclid(d.length());
        let t = if t != d.start() && cyclic_t == 0.0 {
            d.end()
        } else {
            d.start() + cyclic_t
        };
        self.curve.sample_unchecked(t)
    }
}

/// The curve that results from chaining a curve with its reversed version. The transition point
/// is guaranteed to make no jump.
///
/// Curves of this type are produced by [`Curve::ping_pong`].
///
/// # Domain
///
/// The original curve's domain must be right-finite to get a valid [`PingPongCurve`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct PingPongCurve<T, C> {
    pub(super) curve: C,
    pub(super) _phantom: PhantomData<T>,
}

impl<T, C> Curve<T> for PingPongCurve<T, C>
where
    C: Curve<T>,
{
    #[inline]
    fn domain(&self) -> Interval {
        // This unwrap always succeeds because `curve` has a valid Interval as its domain and the
        // length of `curve` cannot be NAN. It's still fine if it's infinity.
        Interval::new(
            self.curve.domain().start(),
            self.curve.domain().end() + self.curve.domain().length(),
        )
        .unwrap()
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        // the domain is bounded by construction
        let final_t = if t > self.curve.domain().end() {
            self.curve.domain().end() * 2.0 - t
        } else {
            t
        };
        self.curve.sample_unchecked(final_t)
    }
}

/// The curve that results from chaining two curves.
///
/// Additionally the transition of the samples is guaranteed to not make sudden jumps. This is
/// useful if you really just know about the shapes of your curves and don't want to deal with
/// stitching them together properly when it would just introduce useless complexity. It is
/// realized by translating the second curve so that its start sample point coincides with the
/// first curves' end sample point.
///
/// Curves of this type are produced by [`Curve::chain_continue`].
///
/// # Domain
///
/// The first curve's domain must be right-finite and the second's must be left-finite to get a
/// valid [`ContinuationCurve`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct ContinuationCurve<T, C, D> {
    pub(super) first: C,
    pub(super) second: D,
    // cache the offset in the curve directly to prevent triple sampling for every sample we make
    pub(super) offset: T,
    pub(super) _phantom: PhantomData<T>,
}

impl<T, C, D> Curve<T> for ContinuationCurve<T, C, D>
where
    T: VectorSpace,
    C: Curve<T>,
    D: Curve<T>,
{
    #[inline]
    fn domain(&self) -> Interval {
        // This unwrap always succeeds because `curve` has a valid Interval as its domain and the
        // length of `curve` cannot be NAN. It's still fine if it's infinity.
        Interval::new(
            self.first.domain().start(),
            self.first.domain().end() + self.second.domain().length(),
        )
        .unwrap()
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        if t > self.first.domain().end() {
            self.second.sample_unchecked(
                // `t - first.domain.end` computes the offset into the domain of the second.
                t - self.first.domain().end() + self.second.domain().start(),
            ) + self.offset
        } else {
            self.first.sample_unchecked(t)
        }
    }
}
