use crate::Mix;
use alloc::vec::Vec;
use bevy_math::curve::{
    cores::{EvenCore, EvenCoreError},
    Curve, Interval,
};

/// A curve whose samples are defined by a collection of colors.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct ColorCurve<T> {
    core: EvenCore<T>,
}

impl<T> ColorCurve<T>
where
    T: Mix + Clone,
{
    /// Create a new [`ColorCurve`] from a collection of [mixable] types. The domain of this curve
    /// will always be `[0.0, len - 1]` where `len` is the amount of mixable objects in the
    /// collection.
    ///
    /// This fails if there's not at least two mixable things in the collection.
    ///
    /// [mixable]: `Mix`
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_color::palettes::basic::*;
    /// # use bevy_color::Mix;
    /// # use bevy_color::Srgba;
    /// # use bevy_color::ColorCurve;
    /// # use bevy_math::curve::Interval;
    /// # use bevy_math::curve::Curve;
    /// let broken = ColorCurve::new([RED]);
    /// assert!(broken.is_err());
    /// let gradient = ColorCurve::new([RED, GREEN, BLUE]);
    /// assert!(gradient.is_ok());
    /// assert_eq!(gradient.unwrap().domain(), Interval::new(0.0, 2.0).unwrap());
    /// ```
    pub fn new(colors: impl IntoIterator<Item = T>) -> Result<Self, EvenCoreError> {
        let colors = colors.into_iter().collect::<Vec<_>>();
        Interval::new(0.0, colors.len().saturating_sub(1) as f32)
            .map_err(|_| EvenCoreError::NotEnoughSamples {
                samples: colors.len(),
            })
            .and_then(|domain| EvenCore::new(domain, colors))
            .map(|core| Self { core })
    }
}

impl<T> Curve<T> for ColorCurve<T>
where
    T: Mix + Clone,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.core.domain()
    }

    #[inline]
    fn sample_clamped(&self, t: f32) -> T {
        // `EvenCore::sample_with` clamps the input implicitly.
        self.core.sample_with(t, T::mix)
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        self.sample_clamped(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{palettes::basic, Srgba};
    use bevy_math::curve::{Curve, CurveExt};

    #[test]
    fn test_color_curve() {
        let broken = ColorCurve::new([basic::RED]);
        assert!(broken.is_err());

        let gradient = [basic::RED, basic::LIME, basic::BLUE];
        let curve = ColorCurve::new(gradient).unwrap();

        assert_eq!(curve.domain(), Interval::new(0.0, 2.0).unwrap());

        let brighter_curve = curve.map(|c: Srgba| c.mix(&basic::WHITE, 0.5));

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
