use std::ops::Range;

use crate::Mix;

/// Represents a range of colors that can be linearly interpolated, defined by a start and
/// end point which must be in the same color space. It works for any color type that
/// implements [`Mix`].
///
/// This is useful for defining simple gradients or animated color transitions.
pub trait ColorRange<T: Mix> {
    /// Get the color value at the given interpolation factor, which should be between 0.0 (start)
    /// and 1.0 (end).
    fn at(&self, factor: f32) -> T;
}

impl<T: Mix> ColorRange<T> for Range<T> {
    fn at(&self, factor: f32) -> T {
        self.start.mix(&self.end, factor.clamp(0.0, 1.0))
    }
}

/// Represents a gradient of a minimum of 1 up to arbitrary many colors. Supported colors have to
/// implement [`Mix`] and have to be from the same color space.
///
/// By default the color values are linearly interpolated.
///
/// This is useful for defining complex gradients or animated color transitions.
pub struct ColorGradient<T: Mix> {
    colors: Vec<T>,
}

impl<T: Mix> ColorGradient<T> {
    /// Create a new [`ColorGradient`] from a collection of [mixable] types.
    ///
    /// This fails if there's not at least one mixable type in the collection.
    ///
    /// [mixable]: `Mix`
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_color::palettes::basic::*;
    /// # use bevy_color::ColorGradient;
    /// let gradient = ColorGradient::new([RED, GREEN, BLUE]);
    /// assert!(gradient.is_ok());
    /// ```
    pub fn new(colors: impl IntoIterator<Item = T>) -> Result<Self, ColorGradientError> {
        let colors = colors.into_iter().collect::<Vec<_>>();
        let len = colors.len();
        (!colors.is_empty())
            .then(|| Self { colors })
            .ok_or_else(|| ColorGradientError(len))
    }
}

impl<T: Mix> ColorRange<T> for ColorGradient<T> {
    fn at(&self, factor: f32) -> T {
        match self.colors.len() {
            len if len == 0 => {
                unreachable!("at least 1 by construction")
            }
            len if len == 1 => {
                // This weirdness exists to prevent adding a `Clone` bound on the type `T` and instead
                // work with what we already have here
                self.colors[0].mix(&self.colors[0], 0.0)
            }
            len => {
                // clamp to range of valid indices
                let factor = factor.clamp(0.0, (len - 1) as f32);
                let fract = factor.fract();
                if fract == 0.0 {
                    // doesn't need clamping since it already was clamped to valid indices
                    let exact_n = factor as usize;
                    // weirdness again
                    self.colors[exact_n].mix(&self.colors[exact_n], 0.0)
                } else {
                    // SAFETY: we know that `len != 0` and `len != 1` here so `len >= 2`
                    let below = (factor.floor() as usize).min(len - 2);
                    self.colors[below].mix(&self.colors[below + 1], fract)
                }
            }
        }
    }
}

/// Error related to violations of invariants of [`ColorGradient`]
#[derive(Debug, thiserror::Error)]
#[error(
    "Couldn't construct a ColorGradient since there were too few colors. Got {0}, expected >=1"
)]
pub struct ColorGradientError(usize);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palettes::basic;
    use crate::{LinearRgba, Srgba};

    #[test]
    fn test_color_range() {
        let range = basic::RED..basic::BLUE;
        assert_eq!(range.at(0.0), basic::RED);
        assert_eq!(range.at(0.5), Srgba::new(0.5, 0.0, 0.5, 1.0));
        assert_eq!(range.at(1.0), basic::BLUE);

        let lred: LinearRgba = basic::RED.into();
        let lblue: LinearRgba = basic::BLUE.into();

        let range = lred..lblue;
        assert_eq!(range.at(0.0), lred);
        assert_eq!(range.at(0.5), LinearRgba::new(0.5, 0.0, 0.5, 1.0));
        assert_eq!(range.at(1.0), lblue);
    }

    #[test]
    fn test_color_gradient() {
        let gradient =
            ColorGradient::new([basic::RED, basic::LIME, basic::BLUE]).expect("Valid gradient");
        assert_eq!(gradient.at(-1.5), basic::RED);
        assert_eq!(gradient.at(-1.0), basic::RED);
        assert_eq!(gradient.at(0.0), basic::RED);
        assert_eq!(gradient.at(0.5), Srgba::new(0.5, 0.5, 0.0, 1.0));
        assert_eq!(gradient.at(1.0), basic::LIME);
        assert_eq!(gradient.at(1.5), Srgba::new(0.0, 0.5, 0.5, 1.0));
        assert_eq!(gradient.at(2.0), basic::BLUE);
        assert_eq!(gradient.at(2.5), basic::BLUE);
        assert_eq!(gradient.at(3.0), basic::BLUE);

        let lred: LinearRgba = basic::RED.into();
        let lgreen: LinearRgba = basic::LIME.into();
        let lblue: LinearRgba = basic::BLUE.into();

        let gradient = ColorGradient::new([lred, lgreen, lblue]).expect("Valid gradient");
        assert_eq!(gradient.at(-1.5), lred);
        assert_eq!(gradient.at(-1.0), lred);
        assert_eq!(gradient.at(0.0), lred);
        assert_eq!(gradient.at(0.5), LinearRgba::new(0.5, 0.5, 0.0, 1.0));
        assert_eq!(gradient.at(1.0), lgreen);
        assert_eq!(gradient.at(1.5), LinearRgba::new(0.0, 0.5, 0.5, 1.0));
        assert_eq!(gradient.at(2.0), lblue);
        assert_eq!(gradient.at(2.5), lblue);
        assert_eq!(gradient.at(3.0), lblue);
    }
}
