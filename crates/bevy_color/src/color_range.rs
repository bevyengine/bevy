use std::ops::Range;

use crate::Mix;

/// Represents a range of colors that can be linearly interpolated, defined by a start and
/// end point which must be in the same color space. It works for any color type that
/// implements [`Mix`].
///
/// This is useful for defining gradients or animated color transitions.
pub trait ColorRange<T: Mix> {
    /// Get the color value at the given interpolation factor, which should be between 0.0 (start)
    /// and 1.0 (end).
    fn at(&self, factor: f32) -> T;
}

impl<T: Mix> ColorRange<T> for Range<T> {
    fn at(&self, factor: f32) -> T {
        self.start.mix(&self.end, factor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LinearRgba, Srgba};

    #[test]
    fn test_color_range() {
        let range = Srgba::RED..Srgba::BLUE;
        assert_eq!(range.at(0.0), Srgba::RED);
        assert_eq!(range.at(0.5), Srgba::new(0.5, 0.0, 0.5, 1.0));
        assert_eq!(range.at(1.0), Srgba::BLUE);

        let lred: LinearRgba = Srgba::RED.into();
        let lblue: LinearRgba = Srgba::BLUE.into();

        let range = lred..lblue;
        assert_eq!(range.at(0.0), lred);
        assert_eq!(range.at(0.5), LinearRgba::new(0.5, 0.0, 0.5, 1.0));
        assert_eq!(range.at(1.0), lblue);
    }
}
