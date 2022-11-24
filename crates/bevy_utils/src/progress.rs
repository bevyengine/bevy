use std::ops::{Add, AddAssign, Div, Mul, RangeInclusive, Sub, SubAssign};
use thiserror::Error;

/// General representation of progress between two values.
#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    /// The minimum value that the progress can have, inclusive.
    min: f32,
    /// The maximum value that the progress can have, inlucsive.
    max: f32,
    /// The current value of progress.
    value: f32,
}

impl Progress {
    /// Creates a new progress using a `value`, and a `min` and `max` that defines a `range`.
    ///
    /// The `value` must be within the bounds of the `range` or returns a [`ProgressError`].
    pub fn new(value: f32, min: f32, max: f32) -> Result<Self, ProgressError> {
        Self::from_range(value, min..=max)
    }

    /// Creates a new progress using a `value` and a `range`.
    ///
    /// The `value` must be within the bounds of the `range` or returns a [`ProgressError::OutOfBounds`].
    pub fn from_range(value: f32, range: RangeInclusive<f32>) -> Result<Self, ProgressError> {
        if range.start() >= range.end() {
            Err(ProgressError::InvalidRange)
        } else if range.contains(&value) {
            Ok(Self {
                value,
                min: *range.start(),
                max: *range.end(),
            })
        } else {
            Err(ProgressError::OutOfBounds)
        }
    }

    /// Creates a new [`Progress`] using percent.
    /// `Min` = 0.0
    /// `Max` = 100.0
    pub fn from_percent(value: f32) -> Self {
        Self::from_range(value, 0.0..=100.0).unwrap()
    }

    /// Gets the min bound of the progress.
    pub fn min(&self) -> f32 {
        self.min
    }

    /// Gets the max bound of the progress.
    pub fn max(&self) -> f32 {
        self.max
    }

    /// Gets the bounds of the progress.
    pub fn bounds(&self) -> RangeInclusive<f32> {
        self.min..=self.max
    }

    /// Gets the current value of progress.
    pub fn progress(&self) -> f32 {
        self.value
    }

    /// Returns the current progress, normalized between 0 and 1.
    ///
    /// 0 represents value == min,
    /// 1 represents value == max.
    pub fn normalized(&self) -> f32 {
        remap_range(self.value, (self.min, self.max), (0.0, 1.0))
    }

    /// Sets the progress to a new value and returns the new value if successful.
    ///
    /// The `value` must be within the bounds of the `range` or returns a [`ProgressError::OutOfBounds`].
    pub fn set_progress(&mut self, new_value: f32) -> Result<f32, ProgressError> {
        if self.bounds().contains(&new_value) {
            self.value = new_value;
            Ok(self.value)
        } else {
            Err(ProgressError::OutOfBounds)
        }
    }
}

impl AddAssign<f32> for Progress {
    /// Increases the progress `value` with `rhs`.
    ///
    /// Clamps to the extent of the bounds.
    fn add_assign(&mut self, rhs: f32) {
        let new_value = self.value + rhs;
        if self.set_progress(new_value.min(self.max)).is_err() {
            unreachable!("This should have been within bounds.");
        }
    }
}

impl SubAssign<f32> for Progress {
    /// Decreases the progress `value` with `rhs`.
    ///
    /// Clamps to the extent of the bounds.
    fn sub_assign(&mut self, rhs: f32) {
        let new_value = self.value - rhs;
        if self.set_progress(new_value.max(self.min)).is_err() {
            unreachable!("This should have been within bounds.");
        }
    }
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 1.0,
            value: 0.0,
        }
    }
}

/// Error types for [`Progress`].
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ProgressError {
    #[error("Value is outside the bounds of the Progress.")]
    OutOfBounds,
    #[error("Tried creating a new [`Progress`] using a range that was not valid.`")]
    InvalidRange,
}

/// Maps a value from one range of values to a new range of values.
///
/// This is essentially an inverse linear interpolation followed by a normal linear interpolation.
#[inline]
pub fn remap_range<
    T: Add<Output = T> + Div<Output = T> + Sub<Output = T> + Mul<Output = T> + Copy,
>(
    value: T,
    old_range: (T, T),
    new_range: (T, T),
) -> T {
    (value - old_range.0) / (old_range.1 - old_range.0) * (new_range.1 - new_range.0) + new_range.0
}

#[cfg(test)]
mod tests {
    use crate::progress::{Progress, ProgressError};

    /// Creating a valid [`Progress`] should work.
    #[test]
    fn valid_range() {
        let min = 0.0;
        let max = 1.0;
        let value = 0.5;
        assert_eq!(
            Progress::from_range(value, min..=max),
            Ok(Progress { min, max, value })
        );
        assert_eq!(
            Progress::new(value, min, max),
            Ok(Progress { min, max, value })
        );
    }

    /// Using a reverse range should not be a considered a valid [`Progress`].
    #[test]
    fn reverse_range() {
        let min = 0.0;
        let max = 1.0;
        let value = 0.5;
        assert_eq!(
            Progress::from_range(value, max..=min),
            Err(ProgressError::InvalidRange)
        );
        assert_eq!(
            Progress::new(value, max, min),
            Err(ProgressError::InvalidRange)
        );
    }

    /// `min` and `max` in a range should be different values otherwise it should not be a considered a valid [`Progress`].
    #[test]
    fn nonsensical_range() {
        let value = 1.0;
        assert_eq!(
            Progress::from_range(value, value..=value),
            Err(ProgressError::InvalidRange)
        );
        assert_eq!(
            Progress::new(value, value, value),
            Err(ProgressError::InvalidRange)
        );
    }

    /// If the `value` is outside the range, we should get a [`ProgressError::OutOfBounds`] error.
    #[test]
    fn out_of_bounds() {
        let min = 0.0;
        let max = 1.0;
        let value = 10.0;
        assert_eq!(
            Progress::from_range(value, min..=max),
            Err(ProgressError::OutOfBounds)
        );
        assert_eq!(
            Progress::new(value, min, max),
            Err(ProgressError::OutOfBounds)
        );
    }

    /// Upating the `value` should work after the [`Progress`] has been created.
    #[test]
    fn set_value_in_bounds() {
        let min = 0.0;
        let max = 1.0;
        let value = 0.5;

        let mut progress = Progress::from_range(value, min..=max).unwrap();
        assert_eq!(progress.progress(), value);
        let result = progress.set_progress(0.8);
        assert!(result.is_ok());
        // progress should be changed from the original
        assert_ne!(progress.progress(), value);
    }

    /// Upating the `value` to something out of bounds, should produce a [`ProgressError::OutOfBounds`] error.
    #[test]
    fn set_value_out_of_bounds() {
        let min = 0.0;
        let max = 1.0;
        let value = 0.5;

        let mut progress = Progress::from_range(value, min..=max).unwrap();
        assert_eq!(progress.progress(), value);
        let result = progress.set_progress(10.0);
        assert_eq!(result, Err(ProgressError::OutOfBounds));
        // progress should be unchanged from the original
        assert_eq!(progress.progress(), value);
    }

    /// Test that we can [`AddAssign`] to the [`Progress`] struct and have the inner value change.
    #[test]
    fn add_assign() {
        let min = 0.0;
        let max = 1.0;
        let value = 0.5;

        let mut progress = Progress::from_range(value, min..=max).unwrap();
        progress += value;
        assert_eq!(progress.progress(), value + value);
    }

    /// Test that we can [`SubAssign`] to the [`Progress`] struct and have the inner value change.
    #[test]
    fn sub_assign() {
        let min = 0.0;
        let max = 1.0;
        let value = 0.5;

        let mut progress = Progress::from_range(value, min..=max).unwrap();
        progress -= value;
        assert_eq!(progress.progress(), value - value);
    }

    /// [`AddAssign`] out of the range bound should panic.
    #[test]
    fn add_assign_out_of_bounds() {
        let min = 0.0;
        let max = 1.0;
        let value = 0.5;

        let mut progress = Progress::from_range(value, min..=max).unwrap();
        assert_eq!(progress.progress(), value);
        // When increasing out of bounds,
        progress += 10.0;
        // value should be clamped to max.
        assert_eq!(progress.progress(), max);
    }

    /// [`SubAssign`] out of the range bound should panic.
    #[test]
    fn sub_assign_out_of_bounds() {
        let min = 0.0;
        let max = 1.0;
        let value = 0.5;

        let mut progress = Progress::from_range(value, min..=max).unwrap();
        assert_eq!(progress.progress(), value);
        // When decreasing out of bounds,
        progress -= 10.0;
        // value should be clamped to min.
        assert_eq!(progress.progress(), min);
    }
}
