use std::ops::Range;

use crate::{LinearRgba, Mix};

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

/// A type-erased color range that can be used to interpolate between colors in various
/// color spaces. Note that both the start and end points must be in the same color space.
/// This is useful for defining an animated color transition, such that the color space can
/// be chosen when the range is created, but the interpolation can be done without knowing
/// the color space.
pub trait AnyColorRange {
    /// Get the color value at the given interpolation factor, converted to linear RGBA.
    fn at_linear(&self, factor: f32) -> LinearRgba;
}

/// Generic implementation for any type that implements [`Mix`] and can be converted into
/// [`LinearRgba`].
impl<T: Mix> AnyColorRange for Range<T>
where
    T: Into<LinearRgba>,
{
    fn at_linear(&self, factor: f32) -> LinearRgba {
        self.at(factor).into()
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
