//! The [`Interval`] type for nonempty intervals used by the [`Curve`](super::Curve) trait.

use core::{
    cmp::{max_by, min_by},
    ops::RangeInclusive,
};
use itertools::Either;
use thiserror::Error;

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
#[cfg(all(feature = "serialize", feature = "bevy_reflect"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// A nonempty closed interval, possibly unbounded in either direction.
///
/// In other words, the interval may stretch all the way to positive or negative infinity, but it
/// will always have some nonempty interior.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct Interval {
    start: f32,
    end: f32,
}

/// An error that indicates that an operation would have returned an invalid [`Interval`].
#[derive(Debug, Error)]
#[error("The resulting interval would be invalid (empty or with a NaN endpoint)")]
pub struct InvalidIntervalError;

/// An error indicating that spaced points could not be extracted from an unbounded interval.
#[derive(Debug, Error)]
#[error("Cannot extract spaced points from an unbounded interval")]
pub struct SpacedPointsError;

/// An error indicating that a linear map between intervals could not be constructed because of
/// unboundedness.
#[derive(Debug, Error)]
#[error("Could not construct linear function to map between intervals")]
pub(super) enum LinearMapError {
    /// The source interval being mapped out of was unbounded.
    #[error("The source interval is unbounded")]
    SourceUnbounded,

    /// The target interval being mapped into was unbounded.
    #[error("The target interval is unbounded")]
    TargetUnbounded,
}

impl Interval {
    /// Create a new [`Interval`] with the specified `start` and `end`. The interval can be unbounded
    /// but cannot be empty (so `start` must be less than `end`) and neither endpoint can be NaN; invalid
    /// parameters will result in an error.
    #[inline]
    pub const fn new(start: f32, end: f32) -> Result<Self, InvalidIntervalError> {
        if start >= end || start.is_nan() || end.is_nan() {
            Err(InvalidIntervalError)
        } else {
            Ok(Self { start, end })
        }
    }

    /// An interval of length 1.0, starting at 0.0 and ending at 1.0.
    pub const UNIT: Self = Self {
        start: 0.0,
        end: 1.0,
    };

    /// An interval which stretches across the entire real line from negative infinity to infinity.
    pub const EVERYWHERE: Self = Self {
        start: f32::NEG_INFINITY,
        end: f32::INFINITY,
    };

    /// Get the start of this interval.
    #[inline]
    pub const fn start(self) -> f32 {
        self.start
    }

    /// Get the end of this interval.
    #[inline]
    pub const fn end(self) -> f32 {
        self.end
    }

    /// Create an [`Interval`] by intersecting this interval with another. Returns an error if the
    /// intersection would be empty (hence an invalid interval).
    pub fn intersect(self, other: Interval) -> Result<Interval, InvalidIntervalError> {
        let lower = max_by(self.start, other.start, f32::total_cmp);
        let upper = min_by(self.end, other.end, f32::total_cmp);
        Self::new(lower, upper)
    }

    /// Get the length of this interval. Note that the result may be infinite (`f32::INFINITY`).
    #[inline]
    pub const fn length(self) -> f32 {
        self.end - self.start
    }

    /// Returns `true` if this interval is bounded — that is, if both its start and end are finite.
    ///
    /// Equivalently, an interval is bounded if its length is finite.
    #[inline]
    pub const fn is_bounded(self) -> bool {
        self.length().is_finite()
    }

    /// Returns `true` if this interval has a finite start.
    #[inline]
    pub const fn has_finite_start(self) -> bool {
        self.start.is_finite()
    }

    /// Returns `true` if this interval has a finite end.
    #[inline]
    pub const fn has_finite_end(self) -> bool {
        self.end.is_finite()
    }

    /// Returns `true` if `item` is contained in this interval.
    #[inline]
    pub fn contains(self, item: f32) -> bool {
        (self.start..=self.end).contains(&item)
    }

    /// Returns `true` if the other interval is contained in this interval.
    ///
    /// This is non-strict: each interval will contain itself.
    #[inline]
    pub const fn contains_interval(self, other: Self) -> bool {
        self.start <= other.start && self.end >= other.end
    }

    /// Clamp the given `value` to lie within this interval.
    #[inline]
    pub const fn clamp(self, value: f32) -> f32 {
        value.clamp(self.start, self.end)
    }

    /// Get an iterator over equally-spaced points from this interval in increasing order.
    /// If `points` is 1, the start of this interval is returned. If `points` is 0, an empty
    /// iterator is returned. An error is returned if the interval is unbounded.
    #[inline]
    pub fn spaced_points(
        self,
        points: usize,
    ) -> Result<impl Iterator<Item = f32>, SpacedPointsError> {
        if !self.is_bounded() {
            return Err(SpacedPointsError);
        }
        if points < 2 {
            // If `points` is 1, this is `Some(self.start)` as an iterator, and if `points` is 0,
            // then this is `None` as an iterator. This is written this way to avoid having to
            // introduce a ternary disjunction of iterators.
            let iter = (points == 1).then_some(self.start).into_iter();
            return Ok(Either::Left(iter));
        }
        let step = self.length() / (points - 1) as f32;
        let iter = (0..points).map(move |x| self.start + x as f32 * step);
        Ok(Either::Right(iter))
    }

    /// Get the linear function which maps this interval onto the `other` one. Returns an error if either
    /// interval is unbounded.
    #[inline]
    pub(super) fn linear_map_to(self, other: Self) -> Result<impl Fn(f32) -> f32, LinearMapError> {
        if !self.is_bounded() {
            return Err(LinearMapError::SourceUnbounded);
        }

        if !other.is_bounded() {
            return Err(LinearMapError::TargetUnbounded);
        }

        let scale = other.length() / self.length();
        Ok(move |x| (x - self.start) * scale + other.start)
    }
}

impl TryFrom<RangeInclusive<f32>> for Interval {
    type Error = InvalidIntervalError;
    fn try_from(range: RangeInclusive<f32>) -> Result<Self, Self::Error> {
        Interval::new(*range.start(), *range.end())
    }
}

/// Create an [`Interval`] with a given `start` and `end`. Alias of [`Interval::new`].
#[inline]
pub const fn interval(start: f32, end: f32) -> Result<Interval, InvalidIntervalError> {
    Interval::new(start, end)
}

#[cfg(test)]
mod tests {
    use crate::ops;

    use super::*;
    use alloc::vec::Vec;
    use approx::{assert_abs_diff_eq, AbsDiffEq};

    #[test]
    fn make_intervals() {
        let ivl = Interval::new(2.0, -1.0);
        assert!(ivl.is_err());

        let ivl = Interval::new(-0.0, 0.0);
        assert!(ivl.is_err());

        let ivl = Interval::new(f32::NEG_INFINITY, 15.5);
        assert!(ivl.is_ok());

        let ivl = Interval::new(-2.0, f32::INFINITY);
        assert!(ivl.is_ok());

        let ivl = Interval::new(f32::NEG_INFINITY, f32::INFINITY);
        assert!(ivl.is_ok());

        let ivl = Interval::new(f32::INFINITY, f32::NEG_INFINITY);
        assert!(ivl.is_err());

        let ivl = Interval::new(-1.0, f32::NAN);
        assert!(ivl.is_err());

        let ivl = Interval::new(f32::NAN, -42.0);
        assert!(ivl.is_err());

        let ivl = Interval::new(f32::NAN, f32::NAN);
        assert!(ivl.is_err());

        let ivl = Interval::new(0.0, 1.0);
        assert!(ivl.is_ok());
    }

    #[test]
    fn lengths() {
        let ivl = interval(-5.0, 10.0).unwrap();
        assert!(ops::abs(ivl.length() - 15.0) <= f32::EPSILON);

        let ivl = interval(5.0, 100.0).unwrap();
        assert!(ops::abs(ivl.length() - 95.0) <= f32::EPSILON);

        let ivl = interval(0.0, f32::INFINITY).unwrap();
        assert_eq!(ivl.length(), f32::INFINITY);

        let ivl = interval(f32::NEG_INFINITY, 0.0).unwrap();
        assert_eq!(ivl.length(), f32::INFINITY);

        let ivl = Interval::EVERYWHERE;
        assert_eq!(ivl.length(), f32::INFINITY);
    }

    #[test]
    fn intersections() {
        let ivl1 = interval(-1.0, 1.0).unwrap();
        let ivl2 = interval(0.0, 2.0).unwrap();
        let ivl3 = interval(-3.0, 0.0).unwrap();
        let ivl4 = interval(0.0, f32::INFINITY).unwrap();
        let ivl5 = interval(f32::NEG_INFINITY, 0.0).unwrap();
        let ivl6 = Interval::EVERYWHERE;

        assert!(ivl1.intersect(ivl2).is_ok_and(|ivl| ivl == Interval::UNIT));
        assert!(ivl1
            .intersect(ivl3)
            .is_ok_and(|ivl| ivl == interval(-1.0, 0.0).unwrap()));
        assert!(ivl2.intersect(ivl3).is_err());
        assert!(ivl1.intersect(ivl4).is_ok_and(|ivl| ivl == Interval::UNIT));
        assert!(ivl1
            .intersect(ivl5)
            .is_ok_and(|ivl| ivl == interval(-1.0, 0.0).unwrap()));
        assert!(ivl4.intersect(ivl5).is_err());
        assert_eq!(ivl1.intersect(ivl6).unwrap(), ivl1);
        assert_eq!(ivl4.intersect(ivl6).unwrap(), ivl4);
        assert_eq!(ivl5.intersect(ivl6).unwrap(), ivl5);
    }

    #[test]
    fn containment() {
        let ivl = Interval::UNIT;
        assert!(ivl.contains(0.0));
        assert!(ivl.contains(1.0));
        assert!(ivl.contains(0.5));
        assert!(!ivl.contains(-0.1));
        assert!(!ivl.contains(1.1));
        assert!(!ivl.contains(f32::NAN));

        let ivl = interval(3.0, f32::INFINITY).unwrap();
        assert!(ivl.contains(3.0));
        assert!(ivl.contains(2.0e5));
        assert!(ivl.contains(3.5e6));
        assert!(!ivl.contains(2.5));
        assert!(!ivl.contains(-1e5));
        assert!(!ivl.contains(f32::NAN));
    }

    #[test]
    fn interval_containment() {
        let ivl = Interval::UNIT;
        assert!(ivl.contains_interval(interval(-0.0, 0.5).unwrap()));
        assert!(ivl.contains_interval(interval(0.5, 1.0).unwrap()));
        assert!(ivl.contains_interval(interval(0.25, 0.75).unwrap()));
        assert!(!ivl.contains_interval(interval(-0.25, 0.5).unwrap()));
        assert!(!ivl.contains_interval(interval(0.5, 1.25).unwrap()));
        assert!(!ivl.contains_interval(interval(0.25, f32::INFINITY).unwrap()));
        assert!(!ivl.contains_interval(interval(f32::NEG_INFINITY, 0.75).unwrap()));

        let big_ivl = interval(0.0, f32::INFINITY).unwrap();
        assert!(big_ivl.contains_interval(interval(0.0, 5.0).unwrap()));
        assert!(big_ivl.contains_interval(interval(0.0, f32::INFINITY).unwrap()));
        assert!(big_ivl.contains_interval(interval(1.0, 5.0).unwrap()));
        assert!(!big_ivl.contains_interval(interval(-1.0, f32::INFINITY).unwrap()));
        assert!(!big_ivl.contains_interval(interval(-2.0, 5.0).unwrap()));
    }

    #[test]
    fn boundedness() {
        assert!(!Interval::EVERYWHERE.is_bounded());
        assert!(interval(0.0, 3.5e5).unwrap().is_bounded());
        assert!(!interval(-2.0, f32::INFINITY).unwrap().is_bounded());
        assert!(!interval(f32::NEG_INFINITY, 5.0).unwrap().is_bounded());
    }

    #[test]
    fn linear_maps() {
        let ivl1 = interval(-3.0, 5.0).unwrap();
        let ivl2 = Interval::UNIT;
        let map = ivl1.linear_map_to(ivl2);
        assert!(map.is_ok_and(|f| f(-3.0).abs_diff_eq(&0.0, f32::EPSILON)
            && f(5.0).abs_diff_eq(&1.0, f32::EPSILON)
            && f(1.0).abs_diff_eq(&0.5, f32::EPSILON)));

        let ivl1 = Interval::UNIT;
        let ivl2 = Interval::EVERYWHERE;
        assert!(ivl1.linear_map_to(ivl2).is_err());

        let ivl1 = interval(f32::NEG_INFINITY, -4.0).unwrap();
        let ivl2 = Interval::UNIT;
        assert!(ivl1.linear_map_to(ivl2).is_err());
    }

    #[test]
    fn spaced_points() {
        let ivl = interval(0.0, 50.0).unwrap();
        let points_iter: Vec<f32> = ivl.spaced_points(1).unwrap().collect();
        assert_abs_diff_eq!(points_iter[0], 0.0);
        assert_eq!(points_iter.len(), 1);
        let points_iter: Vec<f32> = ivl.spaced_points(2).unwrap().collect();
        assert_abs_diff_eq!(points_iter[0], 0.0);
        assert_abs_diff_eq!(points_iter[1], 50.0);
        let points_iter = ivl.spaced_points(21).unwrap();
        let step = ivl.length() / 20.0;
        for (index, point) in points_iter.enumerate() {
            let expected = ivl.start() + step * index as f32;
            assert_abs_diff_eq!(point, expected);
        }

        let ivl = interval(-21.0, 79.0).unwrap();
        let points_iter = ivl.spaced_points(10000).unwrap();
        let step = ivl.length() / 9999.0;
        for (index, point) in points_iter.enumerate() {
            let expected = ivl.start() + step * index as f32;
            assert_abs_diff_eq!(point, expected);
        }

        let ivl = interval(-1.0, f32::INFINITY).unwrap();
        let points_iter = ivl.spaced_points(25);
        assert!(points_iter.is_err());

        let ivl = interval(f32::NEG_INFINITY, -25.0).unwrap();
        let points_iter = ivl.spaced_points(9);
        assert!(points_iter.is_err());
    }
}
