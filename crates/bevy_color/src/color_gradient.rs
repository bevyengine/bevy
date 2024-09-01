use crate::Mix;
use bevy_math::curve::{cores::EvenCoreError, Curve, Interval, SampleCurve};

/// A curve whose samples are defined by a collection of colors. Curves of this type are produced
/// by calling [`ColorGradient::to_curve`].
#[derive(Clone, Debug)]
pub struct ColorCurve<T: Mix + Clone, I> {
    curve: SampleCurve<T, I>,
}

impl<T, I> ColorCurve<T, I>
where
    T: Mix + Clone,
{
    /// Create a new [`ColorCurve`] from a collection of [mixable] types and a mixing function. The
    /// domain of this curve will always be `[0.0, len - 1]` where `len` is the amount of mixable
    /// objects in the collection.
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
    /// let broken = ColorCurve::new([RED], Srgba::mix);
    /// assert!(broken.is_err());
    /// let gradient = ColorCurve::new([RED, GREEN, BLUE], Srgba::mix);
    /// assert!(gradient.is_ok());
    /// assert_eq!(gradient.unwrap().domain(), Interval::new(0.0, 2.0).unwrap());
    /// ```
    pub fn new(
        colors: impl IntoIterator<Item = impl Into<T>>,
        interpolation: I,
    ) -> Result<Self, EvenCoreError>
    where
        I: Fn(&T, &T, f32) -> T,
    {
        let colors = colors.into_iter().map(|ic| ic.into()).collect::<Vec<_>>();
        Interval::new(0.0, colors.len().saturating_sub(1) as f32)
            .map_err(|_| EvenCoreError::NotEnoughSamples {
                samples: colors.len(),
            })
            .and_then(|domain| SampleCurve::new(domain, colors, interpolation))
            .map(|curve| Self { curve })
    }
}

/// Error related to violations of invariants of [`ColorCurve`]
#[derive(Debug, thiserror::Error)]
#[error("Couldn't construct a ColorCurve since there were too few colors. Got {0}, expected >=2")]
pub struct ColorCurveError(usize);

impl<T, F> Curve<T> for ColorCurve<T, F>
where
    T: Mix + Clone,
    F: Fn(&T, &T, f32) -> T,
{
    fn domain(&self) -> Interval {
        self.curve.domain()
    }

    fn sample_unchecked(&self, t: f32) -> T {
        self.curve.sample_unchecked(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palettes::basic;
    use crate::Srgba;

    #[test]
    fn test_color_curve() {
        let broken = ColorCurve::new([basic::RED], Srgba::mix);
        assert!(broken.is_err());

        let gradient = [basic::RED, basic::LIME, basic::BLUE];
        let curve = ColorCurve::new(gradient, Srgba::mix).unwrap();

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
