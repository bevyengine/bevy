use crate::{LinearRgba, Mix};

/// Represents a range of colors that can be linearly interpolated, defined by a start and
/// end point which must be in the same color space. It works for any color type that
/// implements [`Mix`].
///
/// This is useful for defining gradients or animated color transitions.
pub struct ColorRange<T: Mix> {
    start: T,
    end: T,
}

impl<T> ColorRange<T>
where
    T: Mix,
{
    /// Construct a new color range from the start and end values.
    pub fn new(start: T, end: T) -> Self {
        Self { start, end }
    }

    /// Get the color value at the given interpolation factor, which should be between 0.0
    /// and 1.0.
    pub fn at(&self, factor: f32) -> T {
        self.start.mix(&self.end, factor)
    }
}

/// A type-erased color range that can be used to interpolate between colors in various
/// color spaces. Note that both the start and end points must be in the same color space.
pub trait AnyColorRange {
    /// Get the color value at the given interpolation factor, converted to linear RGBA.
    fn at_linear(&self, factor: f32) -> LinearRgba;
}

/// Generic implementation for any type that implements [`Mix`] and can be converted into
/// [`LinearRgba`].
impl<T: Mix> AnyColorRange for ColorRange<T>
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
        let range = ColorRange::new(Srgba::RED, Srgba::BLUE);
        assert_eq!(range.at(0.0), Srgba::RED);
        assert_eq!(range.at(0.5), Srgba::new(0.5, 0.0, 0.5, 1.0));
        assert_eq!(range.at(1.0), Srgba::BLUE);

        let lred: LinearRgba = Srgba::RED.into();
        let lblue: LinearRgba = Srgba::BLUE.into();

        let range = ColorRange::new(lred, lblue);
        assert_eq!(range.at(0.0), lred);
        assert_eq!(range.at(0.5), LinearRgba::new(0.5, 0.0, 0.5, 1.0));
        assert_eq!(range.at(1.0), lblue);
    }
}
