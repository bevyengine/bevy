use crate::{ColorRange, Mix};
use bevy_math::curve::{Curve, Interval};

/// Represents a gradient of a minimum of 1 up to arbitrary many colors. Supported colors have to
/// implement [`Mix`] and have to be from the same color space.
///
/// By default the color values are linearly interpolated.
///
/// This is useful for defining complex gradients or animated color transitions.
#[derive(Debug, Clone)]
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
            .ok_or(ColorGradientError(len))
    }

    /// Converts the [`ColorGradient`] to a [`ColorCurve`] which implements the [`Curve`] trait
    /// along with its adaptor methods
    /// # Example
    ///
    /// ```
    /// # use bevy_color::palettes::basic::*;
    /// # use bevy_color::ColorGradient;
    /// # use bevy_color::Srgba;
    /// # use bevy_color::Mix;
    /// # use bevy_math::curve::Curve;
    /// let gradient = ColorGradient::new([RED, GREEN, BLUE]).unwrap();
    /// let curve = gradient.to_curve();
    ///
    /// // you can then apply useful methods ontop of the gradient
    /// let brighter_curve = curve.map(|c| c.mix(&WHITE, 0.25));
    ///
    /// assert_eq!(brighter_curve.sample_unchecked(0.0), Srgba::new(1.0, 0.25, 0.25, 1.0));
    /// ```
    pub fn to_curve(self) -> ColorCurve<T> {
        let domain =
            Interval::new(0.0, (self.colors.len() - 1) as f32).expect("at least 1 by construction");
        ColorCurve {
            domain,
            gradient: self,
        }
    }
}

impl<T: Mix> ColorRange<T> for ColorGradient<T> {
    fn at(&self, factor: f32) -> T {
        match self.colors.len() {
            0 => {
                unreachable!("at least 1 by construction")
            }
            1 => {
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

/// A curve whose samples are defined by a [`ColorGradient`]. Curves of this type are produced by
/// calling [`ColorGradient::to_curve`].
#[derive(Clone, Debug)]
pub struct ColorCurve<T: Mix> {
    domain: Interval,
    gradient: ColorGradient<T>,
}

impl<T: Mix> Curve<T> for ColorCurve<T> {
    fn domain(&self) -> Interval {
        self.domain
    }

    fn sample_unchecked(&self, t: f32) -> T {
        self.gradient.at(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palettes::basic;
    use crate::{LinearRgba, Srgba};

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

    #[test]
    fn test_color_curve() {
        let gradient = ColorGradient::new([basic::RED, basic::LIME, basic::BLUE]).unwrap();
        let curve = gradient.to_curve();

        assert_eq!(curve.domain(), Interval::new(0.0, 2.0).unwrap());

        let brighter_curve = curve.map(|c| c.mix(&basic::WHITE, 0.5));

        [
            (-0.1, None),
            (0.0, Some([1.0, 0.5, 0.5, 1.0])),
            (0.5, Some([0.75, 0.75, 0.5, 1.0])),
            (1.0, Some([0.5, 1.0, 0.5, 1.0])),
            (1.5, Some([0.5, 0.75, 0.75, 1.0])),
            (2.0, Some([0.5, 0.5, 1.0, 1.0])),
            (2.1, None),
        ]
        .map(|(t, maybe_rgba)| {
            let maybe_srgba = maybe_rgba.map(|[r, g, b, a]| Srgba::new(r, g, b, a));
            (t, maybe_srgba)
        })
        .into_iter()
        .for_each(|(t, maybe_color)| {
            assert_eq!(brighter_curve.sample(t), maybe_color);
        });
    }
}
