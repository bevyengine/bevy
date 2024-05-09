//! The [`Interval`] type for nonempty intervals used by the [`Curve`](super::Curve) trait.

use std::{
    cmp::{max_by, min_by},
    ops::RangeInclusive,
};
use thiserror::Error;

/// A nonempty closed interval, possibly infinite in either direction.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Interval {
    start: f32,
    end: f32,
}

/// An error that indicates that an operation would have returned an invalid [`Interval`].
#[derive(Debug, Error)]
#[error("The resulting interval would be invalid (empty or with a NaN endpoint)")]
pub struct InvalidIntervalError;

/// An error indicating that an infinite interval was used where it was inappropriate.
#[derive(Debug, Error)]
#[error("This operation does not make sense in the context of an infinite interval")]
pub struct InfiniteIntervalError;

/// An error indicating that spaced points on an interval could not be formed.
#[derive(Debug, Error)]
#[error("Could not sample evenly-spaced points with these inputs")]
pub enum SpacedPointsError {
    /// This operation failed because fewer than two points were requested.
    #[error("Parameter `points` must be at least 2")]
    NotEnoughPoints,

    /// This operation failed because the underlying interval is unbounded.
    #[error("Cannot sample evenly-spaced points on an infinite interval")]
    InfiniteInterval(InfiniteIntervalError),
}

impl Interval {
    /// Create a new [`Interval`] with the specified `start` and `end`. The interval can be infinite
    /// but cannot be empty and neither endpoint can be NaN; invalid parameters will result in an error.
    pub fn new(start: f32, end: f32) -> Result<Self, InvalidIntervalError> {
        if start >= end || start.is_nan() || end.is_nan() {
            Err(InvalidIntervalError)
        } else {
            Ok(Self { start, end })
        }
    }

    /// Get the start of this interval.
    #[inline]
    pub fn start(self) -> f32 {
        self.start
    }

    /// Get the end of this interval.
    #[inline]
    pub fn end(self) -> f32 {
        self.end
    }

    /// Create an [`Interval`] by intersecting this interval with another. Returns an error if the
    /// intersection would be empty (hence an invalid interval).
    pub fn intersect(self, other: Interval) -> Result<Interval, InvalidIntervalError> {
        let lower = max_by(self.start, other.start, |x, y| x.partial_cmp(y).unwrap());
        let upper = min_by(self.end, other.end, |x, y| x.partial_cmp(y).unwrap());
        Self::new(lower, upper)
    }

    /// Get the length of this interval. Note that the result may be infinite (`f32::INFINITY`).
    #[inline]
    pub fn length(self) -> f32 {
        self.end - self.start
    }

    /// Returns `true` if this interval is finite.
    #[inline]
    pub fn is_finite(self) -> bool {
        self.length().is_finite()
    }

    /// Returns `true` if `item` is contained in this interval.
    #[inline]
    pub fn contains(self, item: f32) -> bool {
        (self.start..=self.end).contains(&item)
    }

    /// Clamp the given `value` to lie within this interval.
    #[inline]
    pub fn clamp(self, value: f32) -> f32 {
        value.clamp(self.start, self.end)
    }

    /// Get the linear map which maps this curve onto the `other` one. Returns an error if either
    /// interval is infinite.
    pub fn linear_map_to(self, other: Self) -> Result<impl Fn(f32) -> f32, InfiniteIntervalError> {
        if !self.is_finite() || !other.is_finite() {
            return Err(InfiniteIntervalError);
        }
        let scale = other.length() / self.length();
        Ok(move |x| (x - self.start) * scale + other.start)
    }

    /// Get an iterator over equally-spaced points from this interval in increasing order.
    /// Returns `None` if `points` is less than 2; the spaced points always include the endpoints.
    pub fn spaced_points(
        self,
        points: usize,
    ) -> Result<impl Iterator<Item = f32>, SpacedPointsError> {
        if points < 2 {
            return Err(SpacedPointsError::NotEnoughPoints);
        }
        if !self.is_finite() {
            return Err(SpacedPointsError::InfiniteInterval(InfiniteIntervalError));
        }
        let step = self.length() / (points - 1) as f32;
        Ok((0..points).map(move |x| self.start + x as f32 * step))
    }
}

impl TryFrom<RangeInclusive<f32>> for Interval {
    type Error = InvalidIntervalError;
    fn try_from(range: RangeInclusive<f32>) -> Result<Self, Self::Error> {
        Interval::new(*range.start(), *range.end())
    }
}

/// Create an [`Interval`] with a given `start` and `end`. Alias of [`Interval::new`].
pub fn interval(start: f32, end: f32) -> Result<Interval, InvalidIntervalError> {
    Interval::new(start, end)
}

/// The [`Interval`] from negative infinity to infinity.
pub fn everywhere() -> Interval {
    Interval::new(f32::NEG_INFINITY, f32::INFINITY).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert!((ivl.length() - 15.0).abs() <= f32::EPSILON);

        let ivl = interval(5.0, 100.0).unwrap();
        assert!((ivl.length() - 95.0).abs() <= f32::EPSILON);

        let ivl = interval(0.0, f32::INFINITY).unwrap();
        assert_eq!(ivl.length(), f32::INFINITY);

        let ivl = interval(f32::NEG_INFINITY, 0.0).unwrap();
        assert_eq!(ivl.length(), f32::INFINITY);

        let ivl = everywhere();
        assert_eq!(ivl.length(), f32::INFINITY);
    }

    #[test]
    fn intersections() {
        let ivl1 = interval(-1.0, 1.0).unwrap();
        let ivl2 = interval(0.0, 2.0).unwrap();
        let ivl3 = interval(-3.0, 0.0).unwrap();
        let ivl4 = interval(0.0, f32::INFINITY).unwrap();
        let ivl5 = interval(f32::NEG_INFINITY, 0.0).unwrap();
        let ivl6 = everywhere();

        assert!(ivl1
            .intersect(ivl2)
            .is_ok_and(|ivl| ivl == interval(0.0, 1.0).unwrap()));
        assert!(ivl1
            .intersect(ivl3)
            .is_ok_and(|ivl| ivl == interval(-1.0, 0.0).unwrap()));
        assert!(ivl2.intersect(ivl3).is_err());
        assert!(ivl1
            .intersect(ivl4)
            .is_ok_and(|ivl| ivl == interval(0.0, 1.0).unwrap()));
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
        let ivl = interval(0.0, 1.0).unwrap();
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
    fn finiteness() {
        assert!(!everywhere().is_finite());
        assert!(interval(0.0, 3.5e5).unwrap().is_finite());
        assert!(!interval(-2.0, f32::INFINITY).unwrap().is_finite());
        assert!(!interval(f32::NEG_INFINITY, 5.0).unwrap().is_finite());
    }

    #[test]
    fn linear_maps() {
        let ivl1 = interval(-3.0, 5.0).unwrap();
        let ivl2 = interval(0.0, 1.0).unwrap();
        let map = ivl1.linear_map_to(ivl2);
        assert!(map.is_ok_and(|f| f(-3.0).abs_diff_eq(&0.0, f32::EPSILON)
            && f(5.0).abs_diff_eq(&1.0, f32::EPSILON)
            && f(1.0).abs_diff_eq(&0.5, f32::EPSILON)));

        let ivl1 = interval(0.0, 1.0).unwrap();
        let ivl2 = everywhere();
        assert!(ivl1.linear_map_to(ivl2).is_err());

        let ivl1 = interval(f32::NEG_INFINITY, -4.0).unwrap();
        let ivl2 = interval(0.0, 1.0).unwrap();
        assert!(ivl1.linear_map_to(ivl2).is_err());
    }

    #[test]
    fn spaced_points() {
        let ivl = interval(0.0, 50.0).unwrap();
        let points_iter = ivl.spaced_points(1);
        assert!(points_iter.is_err());
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
